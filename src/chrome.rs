//! DPI-aware, antialiased window surface and shadow, sliced once for cheap resizing.
use crate::{AppWindow, ChromeSkin};
use slint::winit_030::{EventResult, WinitWindowAccessor, winit::event::WindowEvent};
use slint::{ComponentHandle, Image, Rgba8Pixel, SharedPixelBuffer};

pub const PADDING: f32 = 36.0;

fn distance(x: f32, y: f32, width: f32, height: f32, radius: f32) -> f32 {
    let qx = (x - width * 0.5).abs() - (width * 0.5 - radius);
    let qy = (y - height * 0.5).abs() - (height * 0.5 - radius);
    qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - radius
}

fn patch(scale: f32, total: (u32, u32), bounds: (u32, u32, u32, u32)) -> Image {
    let (ox, oy, width, height) = bounds;
    let (tw, th) = (total.0 as f32, total.1 as f32);
    let margin = PADDING * scale;
    let (cw, ch, r) = (tw - 2.0 * margin, th - 2.0 * margin, 19.0 * scale);
    let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(width, height);
    let pixels = buffer.make_mut_slice();
    for y in 0..height {
        for x in 0..width {
            let (px, py) = (
                (ox + x) as f32 + 0.5 - margin,
                (oy + y) as f32 + 0.5 - margin,
            );
            // One physical-pixel coverage ramp gives a smooth edge at fractional DPI.
            let coverage = (0.5 - distance(px, py, cw, ch, r)).clamp(0.0, 1.0);
            let d = distance(px, py - 6.0 * scale, cw, ch, r).max(0.0);
            let ambient = 0.18 * (-0.5 * (d / (9.0 * scale)).powi(2)).exp();
            let d = distance(px, py - 2.0 * scale, cw, ch, r).max(0.0);
            let contact = 0.08 * (-0.5 * (d / (3.0 * scale)).powi(2)).exp();
            let shadow = 1.0 - (1.0 - ambient) * (1.0 - contact);
            let alpha = coverage + shadow * (1.0 - coverage);
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
                    (color[0] * coverage / alpha).round() as u8,
                    (color[1] * coverage / alpha).round() as u8,
                    (color[2] * coverage / alpha).round() as u8,
                    a,
                )
            };
        }
    }
    Image::from_rgba8(buffer)
}

fn render(scale: f32) -> ChromeSkin {
    let x = ((PADDING + 19.0) * scale).ceil() as u32;
    let top = ((PADDING + 30.0) * scale).ceil() as u32;
    let bottom = x;
    let total = (2 * x + 1, top + bottom + 1);
    let image = |bounds| patch(scale, total, bounds);
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

pub fn refresh(ui: &AppWindow) {
    let scale = ui
        .window()
        .with_winit_window(|w| w.scale_factor() as f32)
        .unwrap_or(ui.window().scale_factor());
    ui.set_chrome_skin(render(scale));
}

pub fn connect(ui: &AppWindow) {
    refresh(ui);
    let weak = ui.as_weak();
    ui.window().on_winit_window_event(move |_, event| {
        if let WindowEvent::ScaleFactorChanged { scale_factor, .. } = event
            && let Some(ui) = weak.upgrade()
        {
            ui.set_chrome_skin(render(*scale_factor as f32));
        }
        EventResult::Propagate
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_dpi_scales_produce_valid_chrome() {
        for scale in [1.0, 1.25, 1.5, 2.0] {
            let skin = render(scale);
            assert!(skin.cap_x > PADDING);
            assert!(skin.cap_top > skin.cap_x);
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
            let sample = ((PADDING - 6.0) * scale).round() as usize;
            assert!(
                (8..100).contains(&side.as_slice()[sample].a),
                "shadow outside the card must be visible and translucent"
            );
        }
    }
}
