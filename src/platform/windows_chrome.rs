//! Per-pixel-alpha card presentation and a click-through shadow companion.
//! Win32 resources are UI-thread-owned. The displayed DIB remains alive while
//! Windows uses its alpha for composition and hit testing; buffers are wiped
//! before replacement and destruction. No frames are written to files or logs.
#![allow(unsafe_code)]

use crate::ChromeSkin;
use slint::winit_030::winit::{
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::Window,
};
use slint::{Rgba8Pixel, SharedPixelBuffer};
use std::{cell::Cell, io, mem, ptr};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM},
    Graphics::Gdi::*,
    System::Threading::GetCurrentThreadId,
    UI::WindowsAndMessaging::*,
};
use zeroize::Zeroize;

struct Tiles {
    images: Vec<SharedPixelBuffer<Rgba8Pixel>>,
    x: u32,
    top: u32,
    bottom: u32,
}

impl Tiles {
    fn new(skin: &ChromeSkin) -> io::Result<Self> {
        let images = [
            &skin.top_left,
            &skin.top,
            &skin.top_right,
            &skin.left,
            &skin.center,
            &skin.right,
            &skin.bottom_left,
            &skin.bottom,
            &skin.bottom_right,
        ]
        .into_iter()
        .map(|image| {
            image
                .to_rgba8()
                .ok_or_else(|| io::Error::other("Missing chrome pixels"))
        })
        .collect::<io::Result<Vec<_>>>()?;
        let (x, top, bottom) = (images[0].width(), images[0].height(), images[6].height());
        Ok(Self {
            images,
            x,
            top,
            bottom,
        })
    }

    fn alpha(&self, x: u32, y: u32, width: u32, height: u32) -> u8 {
        let (column, sx) = if x < self.x {
            (0, x)
        } else if x >= width - self.x {
            (2, x - (width - self.x))
        } else {
            (1, 0)
        };
        let (row, sy) = if y < self.top {
            (0, y)
        } else if y >= height - self.bottom {
            (2, y - (height - self.bottom))
        } else {
            (1, 0)
        };
        let image = &self.images[row * 3 + column];
        image.as_slice()[(sy * image.width() + sx) as usize].a
    }
}

pub struct Shadow {
    owner: HWND,
    window: HWND,
    scale: f32,
    size: (u32, u32),
    tiles: Option<Tiles>,
    card_surface: Option<Layer>,
    shadow_surface: Option<Layer>,
    movement: Box<Movement>,
    movement_hook: HHOOK,
}

// A native drag runs a nested Windows message loop. Slint timers can be delayed
// there, so synchronize the cached layered image and companion in the position
// message itself. The callback never renders or borrows application state.
struct Movement {
    owner: HWND,
    companion: HWND,
    presenting: Cell<bool>,
    ready: Cell<bool>,
    failed: Cell<bool>,
}

thread_local! {
    static MOVEMENT: Cell<*const Movement> = const { Cell::new(ptr::null()) };
}

unsafe extern "system" fn follow_position(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: a nonnegative WH_CALLWNDPROCRET code supplies CWPRETSTRUCT. The
    // thread-local pointer targets a stable Box retained until the hook is removed.
    // The original window procedure has already received WM_MOVE/WM_SIZE normally.
    unsafe {
        if code >= 0 {
            let message = &*(lparam as *const CWPRETSTRUCT);
            let data = MOVEMENT.with(Cell::get);
            if !data.is_null() && message.message == WM_WINDOWPOSCHANGED {
                let movement = &*data;
                if message.hwnd == movement.owner
                    && movement.ready.get()
                    && !movement.presenting.replace(true)
                {
                    if movement.follow(movement.owner).is_err() {
                        movement.failed.set(true);
                    }
                    movement.presenting.set(false);
                }
            }
        }
        CallNextHookEx(ptr::null_mut(), code, wparam, lparam)
    }
}

impl Movement {
    fn follow(&self, owner: HWND) -> io::Result<()> {
        // SAFETY: handles and position messages belong to this UI thread. A
        // position-only layered update reuses Windows' cached pixels, without
        // resizing the window or reading the renderer's live buffers.
        unsafe {
            if IsIconic(owner) != 0 || IsWindowVisible(owner) == 0 {
                ShowWindow(self.companion, SW_HIDE);
                return Ok(());
            }
            let mut rect: RECT = mem::zeroed();
            if GetWindowRect(owner, &mut rect) == 0 {
                return Err(io::Error::last_os_error());
            }
            let position = POINT {
                x: rect.left,
                y: rect.top,
            };
            if UpdateLayeredWindow(
                owner,
                ptr::null_mut(),
                &position,
                ptr::null(),
                ptr::null_mut(),
                ptr::null(),
                0,
                ptr::null(),
                0,
            ) == 0
            {
                return Err(io::Error::last_os_error());
            }
            if SetWindowPos(
                self.companion,
                ptr::null_mut(),
                position.x,
                position.y,
                0,
                0,
                SWP_NOACTIVATE | SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_SHOWWINDOW | SWP_NOSIZE,
            ) == 0
            {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(())
    }
}

impl Shadow {
    pub fn new(window: &Window) -> io::Result<Self> {
        let handle = window.window_handle().map_err(io::Error::other)?;
        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            return Err(io::Error::other("Expected a Windows window"));
        };
        let owner = handle.hwnd.get() as HWND;
        // SAFETY: the built-in STATIC class needs no custom window procedure;
        // owner is a live winit HWND on this same UI thread. Layered + transparent
        // forwards input to other processes, not just same-thread HWNDs.
        let window = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                windows_sys::w!("STATIC"),
                windows_sys::w!("PassGen shadow"),
                WS_POPUP,
                0,
                0,
                1,
                1,
                owner,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null(),
            )
        };
        if window.is_null() {
            return Err(io::Error::last_os_error());
        }
        let movement = Box::new(Movement {
            owner,
            companion: window,
            presenting: Cell::new(false),
            ready: Cell::new(false),
            failed: Cell::new(false),
        });
        MOVEMENT.with(|active| active.set(movement.as_ref()));
        // SAFETY: the hook only observes this creating thread, never another
        // process. Box allocation remains stable when Shadow moves. Drop unhooks
        // before freeing it. This uses USER32, without a common-controls runtime.
        let movement_hook = unsafe {
            SetWindowsHookExW(
                WH_CALLWNDPROCRET,
                Some(follow_position),
                ptr::null_mut(),
                GetCurrentThreadId(),
            )
        };
        if movement_hook.is_null() {
            let error = io::Error::last_os_error();
            MOVEMENT.with(|active| active.set(ptr::null()));
            unsafe {
                DestroyWindow(window);
            }
            return Err(error);
        }
        // SAFETY: after successfully creating the companion, disable the
        // backend's blur-behind path for this live HWND. Per-pixel composition
        // owns its alpha instead; no hard window-region clipping is applied.
        unsafe {
            use windows_sys::Win32::Graphics::Dwm::*;
            let mut blur: DWM_BLURBEHIND = mem::zeroed();
            blur.dwFlags = DWM_BB_ENABLE;
            DwmEnableBlurBehindWindow(owner, &blur);
        }
        Ok(Self {
            owner,
            window,
            scale: 0.0,
            size: (0, 0),
            tiles: None,
            card_surface: None,
            shadow_surface: None,
            movement,
            movement_hook,
        })
    }

    pub fn restore_native(&self) {
        // SAFETY: this live owner HWND is used only on its creating UI thread.
        unsafe {
            if IsWindow(self.owner) != 0 {
                let style = GetWindowLongPtrW(self.owner, GWL_EXSTYLE);
                SetWindowLongPtrW(self.owner, GWL_EXSTYLE, style & !(WS_EX_LAYERED as isize));
                use windows_sys::Win32::Graphics::Dwm::*;
                let region = CreateRectRgn(0, 0, -1, -1);
                if !region.is_null() {
                    let mut blur: DWM_BLURBEHIND = mem::zeroed();
                    blur.dwFlags = DWM_BB_ENABLE | DWM_BB_BLURREGION;
                    blur.fEnable = 1;
                    blur.hRgnBlur = region;
                    DwmEnableBlurBehindWindow(self.owner, &blur);
                    DeleteObject(region);
                }
                InvalidateRect(self.owner, ptr::null(), 0);
            }
        }
    }

    pub fn sync(
        &mut self,
        window: &Window,
        skin: &ChromeSkin,
        frame: &SharedPixelBuffer<Rgba8Pixel>,
    ) -> io::Result<bool> {
        // SAFETY: native handles stay on the winit UI thread; all queried RECTs
        // are initialized and do not contain application content.
        unsafe {
            if self.movement.failed.get() {
                return Err(io::Error::other(
                    "Could not synchronize layered window movement",
                ));
            }
            if IsIconic(self.owner) != 0 || IsWindowVisible(self.owner) == 0 {
                ShowWindow(self.window, SW_HIDE);
                return Ok(false);
            }
            let mut rect: RECT = mem::zeroed();
            if GetWindowRect(self.owner, &mut rect) == 0 {
                return Err(io::Error::last_os_error());
            }
            let size = (frame.width(), frame.height());
            if size.0 == 0 || size.1 == 0 {
                return Ok(false);
            }
            // Slint's cached geometry can lag the native Resized event. Never
            // let an older rendered frame undo a user resize or animation step.
            let actual = window.inner_size();
            if size != (actual.width, actual.height) {
                return Ok(false);
            }
            let style = GetWindowLongPtrW(self.owner, GWL_EXSTYLE);
            if style & WS_EX_LAYERED as isize == 0 {
                SetWindowLongPtrW(self.owner, GWL_EXSTYLE, style | WS_EX_LAYERED as isize);
            }
            let scale = window.scale_factor() as f32;
            let spans = crate::chrome::card_spans(scale, size.0, size.1);
            if scale != self.scale || self.tiles.is_none() {
                self.tiles = Some(Tiles::new(skin)?);
            }
            let tiles = self.tiles.as_ref().unwrap();
            if size.0 < 2 * tiles.x || size.1 < tiles.top + tiles.bottom {
                return Ok(false);
            }
            if self.size != size || self.scale != scale {
                let layer = layer_for(&mut self.shadow_surface, size)?;
                layer.present(
                    self.window,
                    Some(POINT {
                        x: rect.left,
                        y: rect.top,
                    }),
                    true,
                    |x, y| {
                        if in_card(&spans, x, y) {
                            0
                        } else {
                            u32::from(tiles.alpha(x, y, size.0, size.1)) << 24
                        }
                    },
                )?;
                self.size = size;
                self.scale = scale;
            }
            let layer = layer_for(&mut self.card_surface, size)?;
            // Full bounds refresh Windows' alpha hit map after a GDI repaint.
            // The current-native-size check above prevents stale-frame resizes.
            // UpdateLayeredWindow itself sends position messages. Avoid
            // recursively presenting while the new image is being committed.
            self.movement.presenting.set(true);
            let presented = layer.present(
                self.owner,
                Some(POINT {
                    x: rect.left,
                    y: rect.top,
                }),
                true,
                |x, y| {
                    if in_card(&spans, x, y) {
                        bgra(frame.as_slice()[(y * size.0 + x) as usize])
                    } else {
                        0
                    }
                },
            );
            self.movement.presenting.set(false);
            presented?;
            self.movement.ready.set(true);
            let first = crate::platform::windows_startup::reveal(self.owner)?;
            self.movement.presenting.set(true);
            let followed = self.movement.follow(self.owner);
            self.movement.presenting.set(false);
            followed?;
            if first {
                SetForegroundWindow(self.owner);
            }
            Ok(first)
        }
    }
}

fn in_card(spans: &[Option<(u32, u32)>], x: u32, y: u32) -> bool {
    spans[y as usize].is_some_and(|(left, right)| x >= left && x < right)
}

fn bgra(pixel: Rgba8Pixel) -> u32 {
    let a = u32::from(pixel.a);
    (a << 24)
        | (((u32::from(pixel.r) * a + 127) / 255) << 16)
        | (((u32::from(pixel.g) * a + 127) / 255) << 8)
        | ((u32::from(pixel.b) * a + 127) / 255)
}

/// Reusable native display surface. Its current pixels are necessary UI state;
/// wipe them before the next frame, allocation replacement or destruction.
struct Layer {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    bits: *mut u32,
    size: (u32, u32),
}

fn layer_for(layer: &mut Option<Layer>, size: (u32, u32)) -> io::Result<&mut Layer> {
    if layer.as_ref().is_none_or(|l| l.size != size) {
        *layer = Some(Layer::new(size)?);
    }
    Ok(layer.as_mut().unwrap())
}

impl Layer {
    fn new(size: (u32, u32)) -> io::Result<Self> {
        // SAFETY: this top-down 32-bit DIB owns exactly width*height pixels.
        // Every GDI resource is paired with cleanup in Drop on this UI thread.
        unsafe {
            let mut info: BITMAPINFO = mem::zeroed();
            info.bmiHeader.biSize = mem::size_of::<BITMAPINFOHEADER>() as u32;
            info.bmiHeader.biWidth = size.0 as i32;
            info.bmiHeader.biHeight = -(size.1 as i32);
            info.bmiHeader.biPlanes = 1;
            info.bmiHeader.biBitCount = 32;
            info.bmiHeader.biCompression = BI_RGB;
            let dc = CreateCompatibleDC(ptr::null_mut());
            if dc.is_null() {
                return Err(io::Error::last_os_error());
            }
            let mut bits = ptr::null_mut();
            let bitmap = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, ptr::null_mut(), 0);
            if bitmap.is_null() || bits.is_null() {
                if !bitmap.is_null() {
                    DeleteObject(bitmap);
                }
                DeleteDC(dc);
                return Err(io::Error::last_os_error());
            }
            let previous = SelectObject(dc, bitmap);
            Ok(Self {
                dc,
                bitmap,
                previous,
                bits: bits.cast(),
                size,
            })
        }
    }

    fn pixels(&mut self) -> &mut [u32] {
        // SAFETY: this DIB is exclusively owned, has the checked allocation size,
        // and is used only on its creating UI thread.
        unsafe {
            std::slice::from_raw_parts_mut(self.bits, self.size.0 as usize * self.size.1 as usize)
        }
    }

    fn present(
        &mut self,
        hwnd: HWND,
        position: Option<POINT>,
        resize: bool,
        pixel: impl Fn(u32, u32) -> u32,
    ) -> io::Result<()> {
        let (width, height) = self.size;
        let pixels = self.pixels();
        pixels.zeroize();
        for y in 0..height {
            for x in 0..width {
                pixels[(y * width + x) as usize] = pixel(x, y);
            }
        }
        // SAFETY: the source DC/bitmap and destination HWND are live. The DIB
        // stays valid after presentation until a subsequent frame or shutdown.
        unsafe {
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            let size = SIZE {
                cx: width as i32,
                cy: height as i32,
            };
            if UpdateLayeredWindow(
                hwnd,
                ptr::null_mut(),
                position.as_ref().map_or(ptr::null(), |p| p),
                if resize { &size } else { ptr::null() },
                self.dc,
                &POINT { x: 0, y: 0 },
                0,
                &blend,
                ULW_ALPHA,
            ) == 0
            {
                return Err(io::Error::last_os_error());
            }
            GdiFlush();
        }
        Ok(())
    }
}

impl Drop for Layer {
    fn drop(&mut self) {
        self.pixels().zeroize();
        // SAFETY: restore the previous selected object before deleting our own
        // bitmap and DC. No native handles escape this UI-thread-owned object.
        unsafe {
            SelectObject(self.dc, self.previous);
            DeleteObject(self.bitmap);
            DeleteDC(self.dc);
        }
    }
}

impl Drop for Shadow {
    fn drop(&mut self) {
        // SAFETY: owned companion windows may already have been destroyed when
        // their owner closed. Check both validity and ownership before cleanup.
        unsafe {
            UnhookWindowsHookEx(self.movement_hook);
            MOVEMENT.with(|active| {
                if std::ptr::eq(active.get(), self.movement.as_ref()) {
                    active.set(ptr::null());
                }
            });
            if IsWindow(self.window) != 0 && GetWindow(self.window, GW_OWNER) == self.owner {
                DestroyWindow(self.window);
            }
        }
    }
}

#[cfg(all(debug_assertions, feature = "ui-qa"))]
pub(crate) fn check_synchronous_movement(window: &Window) -> Result<(), String> {
    let handle = window.window_handle().map_err(|e| e.to_string())?;
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return Err("Expected a Windows window".into());
    };
    let owner = handle.hwnd.get() as HWND;
    // SAFETY: this synthetic-preview check runs on the window's UI thread. Each
    // move is checked before returning to Slint's deferred timer loop. No pixels,
    // settings or passwords are read, saved or included in error messages.
    unsafe {
        let data = MOVEMENT.with(Cell::get);
        if data.is_null() || (*data).owner != owner {
            return Err("Native movement synchronization was not installed".into());
        }
        let movement = &*data;
        if !movement.ready.get() {
            return Err("Native layered image was not ready".into());
        }
        let mut original: RECT = mem::zeroed();
        if GetWindowRect(owner, &mut original) == 0 {
            return Err("Could not query preview position".into());
        }
        let check = (|| {
            for (dx, dy) in [(45, 36), (-71, 20), (91, -43), (0, 0)] {
                if SetWindowPos(
                    owner,
                    ptr::null_mut(),
                    original.left + dx,
                    original.top + dy,
                    0,
                    0,
                    SWP_NOACTIVATE | SWP_NOZORDER | SWP_NOSIZE,
                ) == 0
                {
                    return Err("Could not move preview window".into());
                }
                let mut main: RECT = mem::zeroed();
                let mut shadow: RECT = mem::zeroed();
                if GetWindowRect(owner, &mut main) == 0
                    || GetWindowRect(movement.companion, &mut shadow) == 0
                    || (main.left, main.top, main.right, main.bottom)
                        != (shadow.left, shadow.top, shadow.right, shadow.bottom)
                    || movement.failed.get()
                {
                    return Err("Card and shadow moved out of sync before deferred redraw".into());
                }
            }
            Ok(())
        })();
        SetWindowPos(
            owner,
            ptr::null_mut(),
            original.left,
            original.top,
            0,
            0,
            SWP_NOACTIVATE | SWP_NOZORDER | SWP_NOSIZE,
        );
        check
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_shadow_sampling_handles_each_dpi_and_resized_window() {
        for scale in [1.0_f32, 1.25, 1.5, 2.0] {
            let tiles = Tiles::new(&crate::chrome::render(scale)).unwrap();
            for logical in [(394.0, 194.0), (424.0, 307.0)] {
                let (width, height) = (
                    (logical.0 * scale).round() as u32,
                    (logical.1 * scale).round() as u32,
                );
                let mut visible = 0;
                for y in 0..height {
                    for x in 0..width {
                        visible += usize::from(tiles.alpha(x, y, width, height) > 0);
                    }
                }
                assert!(visible > 0);
                assert_eq!(tiles.alpha(0, 0, width, height), 0);
            }
        }
    }
}
