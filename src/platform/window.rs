use slint::winit_030::WinitWindowAccessor;
use slint::winit_030::winit::window::{WindowAttributes, WindowButtons};

pub fn attributes(attributes: WindowAttributes) -> WindowAttributes {
    let attributes =
        attributes.with_enabled_buttons(WindowButtons::CLOSE | WindowButtons::MINIMIZE);
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

pub async fn initialize(window: &slint::Window) -> Result<(), slint::PlatformError> {
    // Slint sets MAXIMIZE from its resize constraints after the attributes hook.
    // Apply the requested button set once the native window has actually been created.
    window
        .winit_window()
        .await?
        .set_enabled_buttons(WindowButtons::CLOSE | WindowButtons::MINIMIZE);
    Ok(())
}

/// The OS width can be newer than Slint's cached geometry during a resize.
pub fn logical_width(window: &slint::Window) -> f32 {
    window
        .with_winit_window(|w| w.inner_size().width as f32 / w.scale_factor() as f32)
        .unwrap_or_else(|| window.size().to_logical(window.scale_factor()).width)
}
