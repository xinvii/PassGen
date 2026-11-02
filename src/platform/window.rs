use std::{cell::Cell, rc::Rc};

use super::settings::WindowPosition;
use slint::winit_030::WinitWindowAccessor;
use slint::winit_030::winit::window::{WindowAttributes, WindowButtons};
use slint::winit_030::winit::{dpi::PhysicalPosition, event::WindowEvent, window::Window};

pub fn attributes(
    attributes: WindowAttributes,
    position: Option<WindowPosition>,
) -> WindowAttributes {
    let attributes = attributes
        .with_enabled_buttons(WindowButtons::CLOSE | WindowButtons::MINIMIZE)
        .with_decorations(false);
    let attributes = if let Some(position) = position {
        attributes.with_position(PhysicalPosition::new(position.x, position.y))
    } else {
        attributes
    };
    #[cfg(target_os = "windows")]
    {
        use slint::winit_030::winit::platform::windows::{
            CornerPreference, WindowAttributesExtWindows,
        };
        attributes
            .with_undecorated_shadow(false)
            .with_border_color(None)
            .with_corner_preference(CornerPreference::DoNotRound)
    }
    #[cfg(not(target_os = "windows"))]
    attributes
}

pub fn begin_resize(window: &slint::Window, direction: i32) {
    use slint::winit_030::winit::window::ResizeDirection;
    let direction = match direction {
        0 => ResizeDirection::North,
        1 => ResizeDirection::East,
        2 => ResizeDirection::South,
        3 => ResizeDirection::West,
        4 => ResizeDirection::NorthEast,
        5 => ResizeDirection::SouthEast,
        6 => ResizeDirection::SouthWest,
        7 => ResizeDirection::NorthWest,
        _ => return,
    };
    window.with_winit_window(|w| {
        let _ = w.drag_resize_window(direction);
    });
}

pub async fn initialize(
    window: &slint::Window,
    placement: &Placement,
) -> Result<(), slint::PlatformError> {
    // Slint sets MAXIMIZE from its resize constraints after the attributes hook.
    // Apply the requested button set once the native window has actually been created.
    let native = window.winit_window().await?;
    native.set_enabled_buttons(WindowButtons::CLOSE | WindowButtons::MINIMIZE);
    #[cfg(not(target_os = "windows"))]
    placement.place_initial(&native);
    #[cfg(target_os = "windows")]
    let _ = placement;
    Ok(())
}

#[derive(Clone, Copy, Debug)]
struct Screen {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

#[derive(Clone, Copy)]
struct Insets {
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
    legacy: u32,
}
impl Insets {
    fn scaled(scale: f64) -> Self {
        let px = |value: f32| (f64::from(value) * scale).round() as u32;
        Self {
            left: px(crate::chrome::LEFT),
            top: px(crate::chrome::TOP),
            right: px(crate::chrome::RIGHT),
            bottom: px(crate::chrome::BOTTOM),
            legacy: (36.0 * scale).round() as u32,
        }
    }
    fn decode(self, p: WindowPosition) -> WindowPosition {
        WindowPosition {
            x: coordinate(i64::from(p.x) + i64::from(self.legacy) - i64::from(self.left)),
            y: coordinate(i64::from(p.y) + i64::from(self.legacy) - i64::from(self.top)),
        }
    }
    fn encode(self, p: WindowPosition) -> WindowPosition {
        WindowPosition {
            x: coordinate(i64::from(p.x) - i64::from(self.legacy) + i64::from(self.left)),
            y: coordinate(i64::from(p.y) - i64::from(self.legacy) + i64::from(self.top)),
        }
    }
}

fn screens(window: &Window) -> (Vec<Screen>, usize) {
    let monitors: Vec<_> = window.available_monitors().collect();
    let preferred = window
        .primary_monitor()
        .or_else(|| window.current_monitor());
    let primary = preferred
        .and_then(|preferred| monitors.iter().position(|monitor| *monitor == preferred))
        .unwrap_or(0);
    (
        monitors
            .iter()
            .map(|monitor| Screen {
                x: monitor.position().x,
                y: monitor.position().y,
                width: monitor.size().width,
                height: monitor.size().height,
            })
            .collect(),
        primary,
    )
}

fn overlap(position: WindowPosition, size: (u32, u32), padding: Insets, screen: Screen) -> u64 {
    let x = i64::from(position.x) + i64::from(padding.left);
    let y = i64::from(position.y) + i64::from(padding.top);
    let width = i64::from(size.0.saturating_sub(padding.left + padding.right));
    let height = i64::from(size.1.saturating_sub(padding.top + padding.bottom));
    let horizontal = ((x + width).min(i64::from(screen.x) + i64::from(screen.width))
        - x.max(i64::from(screen.x)))
    .max(0);
    let vertical = ((y + height).min(i64::from(screen.y) + i64::from(screen.height))
        - y.max(i64::from(screen.y)))
    .max(0);
    horizontal as u64 * vertical as u64
}

fn coordinate(value: i64) -> i32 {
    value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

fn initial_position(
    saved: Option<WindowPosition>,
    screens: &[Screen],
    primary: usize,
    size: (u32, u32),
    padding: Insets,
) -> Option<WindowPosition> {
    let fallback = *screens.get(primary).or_else(|| screens.first())?;
    let target = saved.map(|p| padding.decode(p)).and_then(|position| {
        screens
            .iter()
            .copied()
            .map(|screen| (overlap(position, size, padding, screen), screen))
            .max_by_key(|(area, _)| *area)
            .filter(|(area, _)| *area > 0)
            .map(|(_, screen)| (position, screen))
    });
    let (position, screen) = target.unwrap_or_else(|| {
        (
            WindowPosition {
                x: coordinate(
                    i64::from(fallback.x)
                        + (i64::from(fallback.width) - i64::from(size.0)
                            + i64::from(padding.right)
                            - i64::from(padding.left))
                            / 2,
                ),
                y: coordinate(
                    i64::from(fallback.y)
                        + (i64::from(fallback.height) - i64::from(size.1)
                            + i64::from(padding.bottom)
                            - i64::from(padding.top))
                            / 2,
                ),
            },
            fallback,
        )
    });
    // Fit the visible card, allowing its transparent shadow margin off-screen.
    let minimum_x = i64::from(screen.x) - i64::from(padding.left);
    let minimum_y = i64::from(screen.y) - i64::from(padding.top);
    let maximum_x = (i64::from(screen.x) + i64::from(screen.width) - i64::from(size.0)
        + i64::from(padding.right))
    .max(minimum_x);
    let maximum_y = (i64::from(screen.y) + i64::from(screen.height) - i64::from(size.1)
        + i64::from(padding.bottom))
    .max(minimum_y);
    Some(WindowPosition {
        x: coordinate(i64::from(position.x).clamp(minimum_x, maximum_x)),
        y: coordinate(i64::from(position.y).clamp(minimum_y, maximum_y)),
    })
}

#[derive(Clone, Default)]
pub struct Placement {
    position: Rc<Cell<Option<WindowPosition>>>,
    initialized: Rc<Cell<bool>>,
}

impl Placement {
    pub fn new(position: Option<WindowPosition>) -> Self {
        Self {
            position: Rc::new(Cell::new(position)),
            initialized: Rc::new(Cell::new(false)),
        }
    }

    pub fn position(&self) -> Option<WindowPosition> {
        self.position.get()
    }

    pub(crate) fn place_initial(&self, window: &Window) {
        if self.initialized.get() {
            return;
        }
        let size = window.outer_size();
        if size.width == 0 || size.height == 0 {
            return;
        }
        let (screens, primary) = screens(window);
        let padding = Insets::scaled(window.scale_factor());
        let position = initial_position(
            self.position.get(),
            &screens,
            primary,
            (size.width, size.height),
            padding,
        );
        self.initialized.set(true);
        if let Some(position) = position {
            window.set_outer_position(PhysicalPosition::new(position.x, position.y));
            self.position.set(Some(padding.encode(position)));
        }
        self.capture(window);
    }

    fn capture(&self, window: &Window) {
        if !self.initialized.get() || window.is_minimized() == Some(true) || window.is_maximized() {
            return;
        }
        if let Ok(position) = window.outer_position() {
            let position = WindowPosition {
                x: position.x,
                y: position.y,
            };
            let size = window.outer_size();
            let padding = Insets::scaled(window.scale_factor());
            if screens(window)
                .0
                .iter()
                .any(|screen| overlap(position, (size.width, size.height), padding, *screen) > 0)
            {
                self.position.set(Some(padding.encode(position)));
            }
        }
    }

    pub fn observe(&self, window: &slint::Window, event: &WindowEvent) {
        window.with_winit_window(|window| {
            if matches!(
                event,
                WindowEvent::Resized(_) | WindowEvent::RedrawRequested
            ) {
                #[cfg(not(target_os = "windows"))]
                self.place_initial(window);
            }
            if matches!(event, WindowEvent::Moved(_) | WindowEvent::CloseRequested) {
                self.capture(window);
            }
        });
    }

    pub fn capture_before_close(&self, window: &slint::Window) {
        window.with_winit_window(|window| self.capture(window));
    }
}

/// The OS width can be newer than Slint's cached geometry during a resize.
pub fn logical_width(window: &slint::Window) -> f32 {
    window
        .with_winit_window(|w| w.inner_size().width as f32 / w.scale_factor() as f32)
        .unwrap_or_else(|| window.size().to_logical(window.scale_factor()).width)
}

#[cfg(all(debug_assertions, feature = "ui-qa"))]
pub(crate) fn native_position(saved: WindowPosition, scale: f64) -> WindowPosition {
    Insets::scaled(scale).decode(saved)
}

#[cfg(test)]
mod tests {
    use super::*;
    const PRIMARY: Screen = Screen {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };

    #[test]
    fn first_launch_centers_each_page_and_scaled_window() {
        for size in [(394, 307), (394, 277), (394, 194), (788, 614)] {
            let padding = Insets::scaled(if size.0 == 788 { 2.0 } else { 1.0 });
            let position = initial_position(None, &[PRIMARY], 0, size, padding).unwrap();
            assert!((2 * position.x + size.0 as i32 - 1920).abs() <= 1);
            assert!(
                (2 * position.y + size.1 as i32 + padding.top as i32
                    - padding.bottom as i32
                    - 1080)
                    .abs()
                    <= 1
            );
        }
    }

    #[test]
    fn restores_negative_coordinates_on_a_left_monitor() {
        let left = Screen {
            x: -1920,
            ..PRIMARY
        };
        let saved = WindowPosition { x: -1600, y: 170 };
        assert_eq!(
            initial_position(
                Some(saved),
                &[PRIMARY, left],
                0,
                (591, 461),
                Insets::scaled(1.5)
            ),
            Some(WindowPosition { x: -1579, y: 200 })
        );
    }

    #[test]
    fn missing_monitor_recenters_and_partial_position_is_fitted() {
        let saved = WindowPosition { x: 6000, y: -2000 };
        assert_eq!(
            initial_position(Some(saved), &[PRIMARY], 0, (394, 307), Insets::scaled(1.0)),
            initial_position(None, &[PRIMARY], 0, (394, 307), Insets::scaled(1.0))
        );
        let fitted = initial_position(
            Some(WindowPosition { x: 1800, y: 1000 }),
            &[PRIMARY],
            0,
            (394, 307),
            Insets::scaled(1.0),
        )
        .unwrap();
        assert_eq!(fitted, WindowPosition { x: 1548, y: 801 });
    }

    #[test]
    fn tiny_or_missing_screen_does_not_panic() {
        assert_eq!(
            initial_position(None, &[], 0, (394, 307), Insets::scaled(1.0)),
            None
        );
        let screen = Screen {
            x: 0,
            y: 0,
            width: 200,
            height: 150,
        };
        assert_eq!(
            initial_position(None, &[screen], 0, (394, 307), Insets::scaled(1.0)),
            Some(WindowPosition { x: -22, y: -16 })
        );
    }

    #[test]
    fn changing_shadow_insets_preserves_legacy_visible_position() {
        for scale in [1.0, 1.25, 1.5, 2.0] {
            let padding = Insets::scaled(scale);
            let saved = WindowPosition { x: -1200, y: 160 };
            let actual = padding.decode(saved);
            assert_eq!(
                actual.x + padding.left as i32,
                saved.x + padding.legacy as i32
            );
            assert_eq!(
                actual.y + padding.top as i32,
                saved.y + padding.legacy as i32
            );
            assert_eq!(padding.encode(actual), saved);
        }
    }
}
