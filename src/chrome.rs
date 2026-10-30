//! DPI-aware, antialiased window surface and shadow, sliced once for cheap resizing.
use crate::{AppWindow, ChromeSkin};
use slint::winit_030::{EventResult, WinitWindowAccessor, winit::event::WindowEvent};
use slint::{ComponentHandle, Image, Rgba8Pixel, SharedPixelBuffer};

pub const PADDING: f32 = 36.0;
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
    let margin = PADDING * scale;
    let (cw, ch, r) = (
        width as f32 - 2.0 * margin,
        height as f32 - 2.0 * margin,
        RADIUS * scale,
    );
    let mut coverage = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            coverage.push(
                (0.5 - distance(x as f32 + 0.5 - margin, y as f32 + 0.5 - margin, cw, ch, r))
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
        height as usize,
        AMBIENT_SIGMA * scale,
    );
    let contact = gaussian_blur(
        &coverage,
        width as usize,
        height as usize,
        CONTACT_SIGMA * scale,
    );
    let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(width, height);
    let pixels = buffer.make_mut_slice();
    let inset = BORDER_WIDTH * scale;
    for y in 0..height {
        for x in 0..width {
            let index = (y * width + x) as usize;
            let coverage = coverage[index];
            let ambient = 0.18
                * shifted_alpha(
                    &ambient,
                    width as usize,
                    height as usize,
                    x as usize,
                    y as f32 - AMBIENT_OFFSET * scale,
                );
            let contact = 0.08
                * shifted_alpha(
                    &contact,
                    width as usize,
                    height as usize,
                    x as usize,
                    y as f32 - CONTACT_OFFSET * scale,
                );
            let shadow = 1.0 - (1.0 - ambient) * (1.0 - contact);
            let alpha = coverage + shadow * (1.0 - coverage);
            let px = x as f32 + 0.5 - margin;
            let py = y as f32 + 0.5 - margin;
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

fn render(scale: f32) -> ChromeSkin {
    // Keep the entire curved shadow in fixed corners before stretching flat
    // edges. Otherwise a small source rectangle smears corner shadows sideways.
    let corner = RADIUS + 3.0 * AMBIENT_SIGMA;
    let x = ((PADDING + corner) * scale).ceil() as u32;
    let top = ((PADDING + (corner + AMBIENT_OFFSET).max(30.0)) * scale).ceil() as u32;
    let bottom = ((PADDING + corner + AMBIENT_OFFSET) * scale).ceil() as u32;
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

pub fn refresh(ui: &AppWindow) {
    let scale = ui
        .window()
        .with_winit_window(|w| w.scale_factor() as f32)
        .unwrap_or(ui.window().scale_factor());
    ui.set_chrome_skin(render(scale));
}

pub fn connect(ui: &AppWindow, placement: crate::platform::window::Placement) {
    refresh(ui);
    let weak = ui.as_weak();
    ui.window().on_winit_window_event(move |window, event| {
        placement.observe(window, event);
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
                (8..18).contains(&side.as_slice()[sample].a),
                "side shadow must match Figma's light edge profile"
            );
            let top = skin.top.to_rgba8().expect("RGBA top shadow");
            let edge = (PADDING * scale).round() as usize;
            assert_eq!(
                top.as_slice()[edge],
                Rgba8Pixel::new(220, 225, 234, 255),
                "inside top stroke lost its design color"
            );
            let edge = (PADDING * scale).round() as usize;
            assert_eq!(
                side.as_slice()[edge],
                Rgba8Pixel::new(220, 225, 234, 255),
                "inside side stroke lost its design color"
            );
            let interior = ((PADDING + 2.0) * scale).round() as usize;
            assert_eq!(
                side.as_slice()[interior],
                Rgba8Pixel::new(255, 255, 255, 255),
                "stroke must not expand into the body"
            );
            let sample = ((PADDING - 1.0) * scale).round() as usize;
            assert!(
                (8..22).contains(&top.as_slice()[sample].a),
                "top shadow became too dark"
            );
            let bottom = skin.bottom.to_rgba8().expect("RGBA bottom shadow");
            let sample = ((skin.cap_bottom - PADDING) * scale).round() as usize;
            assert!(
                (39..54).contains(&bottom.as_slice()[sample].a),
                "bottom shadow must retain Figma's downward depth"
            );
        }
    }
}
