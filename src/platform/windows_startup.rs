//! Keep the native window cloaked until its first complete alpha frame is ready.
#![allow(unsafe_code)]

use std::{cell::Cell, io, mem, ptr};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    Graphics::Dwm::*,
    System::Threading::GetCurrentThreadId,
    UI::WindowsAndMessaging::*,
};

thread_local! {
    static ARMED: Cell<bool> = const { Cell::new(false) };
    static CLOAKED: Cell<HWND> = const { Cell::new(ptr::null_mut()) };
    static HOOK: Cell<HHOOK> = const { Cell::new(ptr::null_mut()) };
    #[cfg(all(debug_assertions, feature = "ui-qa"))]
    static FIRST_FRAME_WAS_CLOAKED: Cell<bool> = const { Cell::new(false) };
}

pub struct Startup {
    hook: HHOOK,
}

impl Startup {
    pub fn new() -> io::Result<Self> {
        // SAFETY: thread-local call-window hook; no injection into other apps.
        // It is installed before winit creates/shows our main HWND.
        let hook = unsafe {
            SetWindowsHookExW(
                WH_CALLWNDPROC,
                Some(before_message),
                ptr::null_mut(),
                GetCurrentThreadId(),
            )
        };
        if hook.is_null() {
            return Err(io::Error::last_os_error());
        }
        ARMED.with(|armed| armed.set(true));
        HOOK.with(|active| active.set(hook));
        CLOAKED.with(|window| window.set(ptr::null_mut()));
        Ok(Self { hook })
    }
}

unsafe extern "system" fn before_message(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: Windows provides a CWPSTRUCT for nonnegative hook codes. The fixed
    // title check restricts cloaking to PassGen's main window on this UI thread.
    unsafe {
        if code >= 0 && ARMED.with(Cell::get) {
            let message = &*(lparam as *const CWPSTRUCT);
            if message.message == WM_SHOWWINDOW && message.wParam != 0 {
                let mut title = [0u16; 64];
                let count = GetWindowTextW(message.hwnd, title.as_mut_ptr(), title.len() as i32);
                let title = String::from_utf16_lossy(&title[..count.max(0) as usize]);
                if title == "PassGen" || title == "PassGen [UI preview]" {
                    let cloak: i32 = 1;
                    if DwmSetWindowAttribute(
                        message.hwnd,
                        DWMWA_CLOAK as u32,
                        (&cloak as *const i32).cast(),
                        mem::size_of_val(&cloak) as u32,
                    ) >= 0
                    {
                        CLOAKED.with(|window| window.set(message.hwnd));
                    }
                }
            }
        }
        CallNextHookEx(ptr::null_mut(), code, wparam, lparam)
    }
}

pub fn pending() -> bool {
    ARMED.with(Cell::get)
}

/// Returns true once, after a complete native frame has been committed.
pub fn reveal(hwnd: HWND) -> io::Result<bool> {
    if !pending() {
        return Ok(false);
    }
    // SAFETY: the caller has just presented our live HWND at its final startup
    // size, DPI and position. Uncloaking does not change its geometry or alpha.
    unsafe {
        #[cfg(all(debug_assertions, feature = "ui-qa"))]
        {
            let mut cloaked: u32 = 0;
            let result = DwmGetWindowAttribute(
                hwnd,
                DWMWA_CLOAKED as u32,
                (&mut cloaked as *mut u32).cast(),
                mem::size_of_val(&cloaked) as u32,
            );
            FIRST_FRAME_WAS_CLOAKED.with(|value| {
                value.set(result >= 0 && cloaked != 0 && CLOAKED.with(Cell::get) == hwnd)
            });
        }
        let cloak: i32 = 0;
        let result = DwmSetWindowAttribute(
            hwnd,
            DWMWA_CLOAK as u32,
            (&cloak as *const i32).cast(),
            mem::size_of_val(&cloak) as u32,
        );
        if result < 0 {
            return Err(io::Error::other(format!(
                "Unable to reveal window ({result:#x})"
            )));
        }
    }
    ARMED.with(|armed| armed.set(false));
    // The startup hook has no purpose after the first reveal.
    let hook = HOOK.with(|active| active.replace(ptr::null_mut()));
    if !hook.is_null() {
        // SAFETY: this is the thread-local hook installed by Startup::new.
        unsafe {
            UnhookWindowsHookEx(hook);
        }
    }
    CLOAKED.with(|window| window.set(ptr::null_mut()));
    crate::platform::instance::mark_ready(hwnd);
    Ok(true)
}

#[cfg(all(debug_assertions, feature = "ui-qa"))]
pub fn check_first_frame() -> Result<(), String> {
    if pending() || !FIRST_FRAME_WAS_CLOAKED.with(Cell::get) {
        Err("Main window was not cloaked before its first complete frame".into())
    } else {
        Ok(())
    }
}

impl Drop for Startup {
    fn drop(&mut self) {
        ARMED.with(|armed| armed.set(false));
        // SAFETY: stop our own thread hook, and release any cloak if startup
        // failed before a frame could be shown. No unrelated window is touched.
        unsafe {
            HOOK.with(|active| {
                if active.get() == self.hook {
                    active.set(ptr::null_mut());
                    UnhookWindowsHookEx(self.hook);
                }
            });
            let hwnd = CLOAKED.with(|window| window.replace(ptr::null_mut()));
            if !hwnd.is_null() && IsWindow(hwnd) != 0 {
                let cloak: i32 = 0;
                DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_CLOAK as u32,
                    (&cloak as *const i32).cast(),
                    mem::size_of_val(&cloak) as u32,
                );
            }
        }
    }
}
