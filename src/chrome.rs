//! DPI-aware, antialiased window surface and shadow, sliced once for cheap resizing.
use crate::{AppWindow, ChromeSkin};
use slint::winit_030::{EventResult, WinitWindowAccessor, winit::event::WindowEvent};
use slint::{ComponentHandle, Image, Rgba8Pixel, SharedPixelBuffer};

pub const LEFT: f32 = 22.0;
pub const RIGHT: f32 = 22.0;
pub const TOP: f32 = 16.0;
pub const BOTTOM: f32 = 28.0;
pub const HORIZONTAL: f32 = LEFT + RIGHT;
pub const VERTICAL: f32 = TOP + BOTTOM;
const RADIUS: f32 = 19.0;
const BORDER_WIDTH: f32 = 1.0;
const BORDER_COLOR: [f32; 3] = [220.0, 225.0, 234.0];
const AMBIENT_SIGMA: f32 = 9.0;
const CONTACT_SIGMA: f32 = 3.0;
const AMBIENT_OFFSET: f32 = 6.0;
const CONTACT_OFFSET: f32 = 2.0;

fn distance(x: f32, y: f32, width: f32, height: f32, radius: f32) -> f32 {
    let qx = (x - width * 0.5).abs() - (width * 0.5 - radius);
    let qy = (y - height * 0.5).abs() - (height * 0.5 - radius);
    qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - radius
}

fn gaussian_blur(mask: &[f32], width: usize, height: usize, sigma: f32) -> Vec<f32> {
    let radius = (3.0 * sigma).ceil() as usize;
    let mut kernel: Vec<_> = (0..=2 * radius)
        .map(|i| (-0.5 * ((i as f32 - radius as f32) / sigma).powi(2)).exp())
        .collect();
    let sum: f32 = kernel.iter().sum();
    kernel.iter_mut().for_each(|weight| *weight /= sum);
    let mut horizontal = vec![0.0; mask.len()];
    let mut blurred = vec![0.0; mask.len()];
    for y in 0..height {
        for x in 0..width {
            let from = x.saturating_sub(radius);
            let to = (x + radius + 1).min(width);
            horizontal[y * width + x] = mask[y * width + from..y * width + to]
                .iter()
                .zip(&kernel[radius + from - x..radius + to - x])
                .map(|(alpha, weight)| alpha * weight)
                .sum();
        }
    }
    for y in 0..height {
        let from = y.saturating_sub(radius);
        let to = (y + radius + 1).min(height);
        for x in 0..width {
            blurred[y * width + x] = (from..to)
                .map(|source_y| horizontal[source_y * width + x] * kernel[radius + source_y - y])
                .sum();
        }
    }
    blurred
}

fn shifted_alpha(mask: &[f32], width: usize, height: usize, x: usize, y: f32) -> f32 {
    let low = y.floor();
    let sample = |row: f32| {
        if row < 0.0 || row >= height as f32 {
            0.0
        } else {
            mask[row as usize * width + x]
        }
    };
    sample(low) * (1.0 - (y - low)) + sample(low + 1.0) * (y - low)
}

fn surface(scale: f32, width: u32, height: u32) -> SharedPixelBuffer<Rgba8Pixel> {
    let (left, top) = (LEFT * scale, TOP * scale);
    let (cw, ch, r) = (
        width as f32 - HORIZONTAL * scale,
        height as f32 - VERTICAL * scale,
        RADIUS * scale,
    );
    // A downward-offset blur samples rows above the output bitmap. Keep those
    // convolution rows even when the visible transparent margin is tightened.
    let apron = (AMBIENT_OFFSET * scale).ceil() as u32;
    let blur_height = height + 2 * apron;
    let mut coverage = Vec::with_capacity((width * blur_height) as usize);
    for y in 0..blur_height {
        for x in 0..width {
            coverage.push(
                (0.5 - distance(
                    x as f32 + 0.5 - left,
                    y as f32 + 0.5 - top - apron as f32,
                    cw,
                    ch,
                    r,
                ))
                .clamp(0.0, 1.0),
            );
        }
    }
    // Figma's exported filter blurs SourceAlpha with sigma 9 / 3, then offsets
    // the two layers by 6 / 2px. Blurring coverage preserves half-strength edges;
    // a Gaussian of distance instead starts near full opacity and looks too dark.
    let ambient = gaussian_blur(
        &coverage,
        width as usize,
        blur_height as usize,
        AMBIENT_SIGMA * scale,
    );
    let contact = gaussian_blur(
        &coverage,
        width as usize,
        blur_height as usize,
        CONTACT_SIGMA * scale,
    );
    let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(width, height);
    let pixels = buffer.make_mut_slice();
    let inset = BORDER_WIDTH * scale;
    for y in 0..height {
        for x in 0..width {
            let index = ((y + apron) * width + x) as usize;
            let coverage = coverage[index];
            let ambient = 0.18
                * shifted_alpha(
                    &ambient,
                    width as usize,
                    blur_height as usize,
                    x as usize,
                    (y + apron) as f32 - AMBIENT_OFFSET * scale,
                );
            let contact = 0.08
                * shifted_alpha(
                    &contact,
                    width as usize,
                    blur_height as usize,
                    x as usize,
                    (y + apron) as f32 - CONTACT_OFFSET * scale,
                );
            let shadow = 1.0 - (1.0 - ambient) * (1.0 - contact);
            let alpha = coverage + shadow * (1.0 - coverage);
            let px = x as f32 + 0.5 - left;
            let py = y as f32 + 0.5 - top;
            // Subtract the inset coverage to form a DPI-scaled inside stroke.
            // Sharing the outer coverage keeps the existing antialiasing and shadow.
            let inner = (0.5
                - distance(
                    px - inset,
                    py - inset,
                    cw - 2.0 * inset,
                    ch - 2.0 * inset,
                    r - inset,
                ))
            .clamp(0.0, coverage);
            let stroke = coverage - inner;
            let color = if py < 30.0 * scale {
                [243.0, 244.0, 247.0]
            } else {
                [255.0; 3]
            };
            let pixel = &mut pixels[(y * width + x) as usize];
            let a = (alpha * 255.0).round() as u8;
            *pixel = if a < 1 {
                Rgba8Pixel::new(0, 0, 0, 0)
            } else {
                Rgba8Pixel::new(
                    ((color[0] * inner + BORDER_COLOR[0] * stroke) / alpha).round() as u8,
                    ((color[1] * inner + BORDER_COLOR[1] * stroke) / alpha).round() as u8,
                    ((color[2] * inner + BORDER_COLOR[2] * stroke) / alpha).round() as u8,
                    a,
                )
            };
        }
    }
    buffer
}

fn patch(surface: &SharedPixelBuffer<Rgba8Pixel>, bounds: (u32, u32, u32, u32)) -> Image {
    let (ox, oy, width, height) = bounds;
    let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(width, height);
    for y in 0..height {
        let source = ((oy + y) * surface.width() + ox) as usize;
        let destination = (y * width) as usize;
        buffer.make_mut_slice()[destination..destination + width as usize]
            .copy_from_slice(&surface.as_slice()[source..source + width as usize]);
    }
    Image::from_rgba8(buffer)
}

pub(crate) fn render(scale: f32) -> ChromeSkin {
    // Keep the entire curved shadow in fixed corners before stretching flat
    // edges. Otherwise a small source rectangle smears corner shadows sideways.
    let corner = RADIUS + 3.0 * AMBIENT_SIGMA;
    let x = ((LEFT + corner) * scale).ceil() as u32;
    let top = ((TOP + (corner + AMBIENT_OFFSET).max(30.0)) * scale).ceil() as u32;
    let bottom = ((BOTTOM + corner + AMBIENT_OFFSET) * scale).ceil() as u32;
    let total = (2 * x + 1, top + bottom + 1);
    let surface = surface(scale, total.0, total.1);
    let image = |bounds| patch(&surface, bounds);
    ChromeSkin {
        top_left: image((0, 0, x, top)),
        top: image((x, 0, 1, top)),
        top_right: image((x + 1, 0, x, top)),
        left: image((0, top, x, 1)),
        center: image((x, top, 1, 1)),
        right: image((x + 1, top, x, 1)),
        bottom_left: image((0, top + 1, x, bottom)),
        bottom: image((x, top + 1, 1, bottom)),
        bottom_right: image((x + 1, top + 1, x, bottom)),
        cap_x: x as f32 / scale,
        cap_top: top as f32 / scale,
        cap_bottom: bottom as f32 / scale,
    }
}

/// Pixel-inclusive card spans. Keep every antialiased card-edge pixel; only the
/// original shadow/transparent pixels are moved to the click-through layer.
pub(crate) fn card_spans(scale: f32, width: u32, height: u32) -> Vec<Option<(u32, u32)>> {
    let (left, top) = (LEFT * scale, TOP * scale);
    let (cw, ch) = (
        width as f32 - HORIZONTAL * scale,
        height as f32 - VERTICAL * scale,
    );
    (0..height)
        .map(|y| {
            let inside = |x: u32| {
                distance(
                    x as f32 + 0.5 - left,
                    y as f32 + 0.5 - top,
                    cw,
                    ch,
                    RADIUS * scale,
                ) < 0.5
            };
            if !inside(width / 2) {
                return None;
            }
            let (mut lo, mut hi) = (0, width / 2);
            while lo < hi {
                let mid = (lo + hi) / 2;
                if inside(mid) {
                    hi = mid;
                } else {
                    lo = mid + 1;
                }
            }
            Some((lo, width - lo))
        })
        .collect()
}

#[derive(Clone)]
pub struct Controller {
    #[cfg(target_os = "windows")]
    initialized: std::rc::Rc<std::cell::Cell<bool>>,
    #[cfg(target_os = "windows")]
    placement: crate::platform::window::Placement,
    #[cfg(target_os = "windows")]
    scheduled: std::rc::Rc<std::cell::Cell<bool>>,
    #[cfg(target_os = "windows")]
    native:
        std::rc::Rc<std::cell::RefCell<(bool, Option<crate::platform::windows_chrome::Shadow>)>>,
}

impl Controller {
    pub fn initialize(&self, ui: &AppWindow) {
        #[cfg(target_os = "windows")]
        self.initialized.set(true);
        self.sync(ui);
    }
    pub fn sync(&self, ui: &AppWindow) {
        #[cfg(target_os = "windows")]
        {
            if self.scheduled.replace(true) {
                return;
            }
            let (weak, controller) = (ui.as_weak(), self.clone());
            slint::Timer::single_shot(std::time::Duration::ZERO, move || {
                controller.scheduled.set(false);
                if let Some(ui) = weak.upgrade() {
                    controller.present(&ui);
                }
            });
        }
        #[cfg(not(target_os = "windows"))]
        let _ = ui;
    }

    #[cfg(target_os = "windows")]
    fn present(&self, ui: &AppWindow) {
        use zeroize::Zeroize;
        if !self.initialized.get() {
            return;
        }
        let Some(scale) = ui
            .window()
            .with_winit_window(|window| window.scale_factor() as f32)
        else {
            return;
        };
        if (ui.window().scale_factor() - scale).abs() > 0.001 {
            return;
        }
        if crate::platform::windows_startup::pending() {
            let expected = (ui.get_page_height().max(150.0) + VERTICAL) * scale;
            if (ui.window().size().height as f32 - expected).abs() > 1.0 {
                return;
            }
        }
        let skin = ui.get_chrome_skin();
        if skin.top_left.size().width != (skin.cap_x * scale).round() as u32 {
            ui.set_chrome_skin(render(scale));
        }
        let Ok(mut frame) = ui.window().take_snapshot() else {
            return;
        };
        let mut first_frame = false;
        ui.window().with_winit_window(|window| {
            let Ok(mut state) = self.native.try_borrow_mut() else {
                return;
            };
            if state.0 {
                return;
            }
            let result = (|| {
                self.placement.place_initial(window);
                if state.1.is_none() {
                    state.1 = Some(crate::platform::windows_chrome::Shadow::new(window)?);
                }
                state
                    .1
                    .as_mut()
                    .unwrap()
                    .sync(window, &ui.get_chrome_skin(), &frame)
            })();
            if let Ok(first) = result {
                first_frame = first;
            } else if let Err(error) = result {
                if let Some(shadow) = state.1.take() {
                    shadow.restore_native();
                }
                state.0 = true;
                eprintln!("PassGen window chrome: {error}");
            }
            #[cfg(all(debug_assertions, feature = "ui-qa"))]
            crate::qa::record_native_frame();
        });
        frame.make_mut_bytes().zeroize();
        #[cfg(all(debug_assertions, feature = "ui-qa"))]
        if first_frame {
            ui.invoke_native_initialized();
        }
        #[cfg(not(all(debug_assertions, feature = "ui-qa")))]
        let _ = first_frame;
    }
}

pub fn refresh(ui: &AppWindow) {
    let scale = ui
        .window()
        .with_winit_window(|w| w.scale_factor() as f32)
        .unwrap_or(ui.window().scale_factor());
    ui.set_chrome_skin(render(scale));
}

pub fn connect(ui: &AppWindow, placement: crate::platform::window::Placement) -> Controller {
    refresh(ui);
    let controller = Controller {
        #[cfg(target_os = "windows")]
        initialized: Default::default(),
        #[cfg(target_os = "windows")]
        placement: placement.clone(),
        #[cfg(target_os = "windows")]
        scheduled: Default::default(),
        #[cfg(target_os = "windows")]
        native: Default::default(),
    };
    let events = controller.clone();
    let weak = ui.as_weak();
    ui.window().on_winit_window_event(move |window, event| {
        placement.observe(window, event);
        if let WindowEvent::ScaleFactorChanged { scale_factor, .. } = event
            && let Some(ui) = weak.upgrade()
        {
            ui.set_chrome_skin(render(*scale_factor as f32));
        }
        if matches!(
            event,
            WindowEvent::Resized(_)
                | WindowEvent::Moved(_)
                | WindowEvent::RedrawRequested
                | WindowEvent::Focused(_)
                | WindowEvent::ScaleFactorChanged { .. }
        ) && let Some(ui) = weak.upgrade()
        {
            events.sync(&ui);
        }
        EventResult::Propagate
    });
    controller
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn input_mask_keeps_card_edges_and_excludes_shadow_at_each_dpi() {
        for scale in [1.0_f32, 1.25, 1.5, 2.0] {
            let width = (394.0 * scale).round() as u32;
            let height = (307.0 * scale).round() as u32;
            let rows = card_spans(scale, width, height);
            let top = (TOP * scale).round() as usize;
            let left = (LEFT * scale).floor() as u32;
            assert!(rows[..top].iter().all(Option::is_none));
            let (first, last) = rows[height as usize / 2].unwrap();
            assert_eq!(first, left);
            assert_eq!(last, width - left);
            assert!(
                rows[top].unwrap().0 > first,
                "rounded corner lost its input hole"
            );
            assert!(
                rows[(height as f32 - BOTTOM * scale).ceil() as usize..]
                    .iter()
                    .all(Option::is_none)
            );
            for (first, last) in rows.into_iter().flatten() {
                assert!(first >= left && last <= width - left && first < last);
            }
        }
    }

    #[test]
    fn all_dpi_scales_produce_valid_chrome() {
        for scale in [1.0, 1.25, 1.5, 2.0] {
            let skin = render(scale);
            assert!(skin.cap_x > LEFT);
            assert!(skin.cap_top >= skin.cap_x);
            assert!(skin.cap_bottom > skin.cap_top);
            assert!(skin.top_left.size().width > 0);
            let pixels = skin.top_left.to_rgba8().expect("RGBA window skin");
            assert_eq!(
                pixels.as_slice()[0].a,
                0,
                "shadow must fade before the outer window edge"
            );
            assert!(
                pixels
                    .as_slice()
                    .iter()
                    .any(|p| p.a > 80 && p.a < 254 && p.r > 100),
                "rounded edge needs fractional pixel coverage"
            );
            let side = skin.left.to_rgba8().expect("RGBA side shadow");
            let sample = ((LEFT - 6.0) * scale).round() as usize;
            assert!(
                (8..18).contains(&side.as_slice()[sample].a),
                "side shadow must match Figma's light edge profile"
            );
            let top = skin.top.to_rgba8().expect("RGBA top shadow");
            assert!(
                top.as_slice()[(4.0 * scale).round() as usize].a > 0,
                "tight top margin clipped the offset blur's sampling apron"
            );
            let edge = (TOP * scale).round() as usize;
            assert_eq!(
                top.as_slice()[edge],
                Rgba8Pixel::new(220, 225, 234, 255),
                "inside top stroke lost its design color"
            );
            let edge = (LEFT * scale).round() as usize;
            let border = side.as_slice()[edge];
            assert!(
                (220..=235).contains(&border.r)
                    && (225..=240).contains(&border.g)
                    && (234..=245).contains(&border.b)
                    && border.a == 255,
                "inside side stroke lost its coverage-aware design color"
            );
            let interior = ((LEFT + 2.0) * scale).round() as usize;
            assert_eq!(
                side.as_slice()[interior],
                Rgba8Pixel::new(255, 255, 255, 255),
                "stroke must not expand into the body"
            );
            let sample = ((TOP - 1.0) * scale).round() as usize;
            assert!(
                (8..22).contains(&top.as_slice()[sample].a),
                "top shadow became too dark"
            );
            let bottom = skin.bottom.to_rgba8().expect("RGBA bottom shadow");
            let sample = ((skin.cap_bottom - BOTTOM) * scale).round() as usize;
            assert!(
                (39..54).contains(&bottom.as_slice()[sample].a),
                "bottom shadow must retain Figma's downward depth"
            );
        }
    }
}
