//! Opt-in debug-only integration checks. Snapshots always contain synthetic data.

use std::{cell::RefCell, fs::File, io::BufWriter, path::Path, rc::Rc, time::Duration};

use slint::{ComponentHandle, Model};

use crate::AppWindow;

type Outcome = Rc<RefCell<Option<Result<(), String>>>>;

pub fn settings_check_mode() -> Option<bool> {
    if std::env::args().any(|arg| arg == "--qa-settings-write") {
        Some(true)
    } else if std::env::args().any(|arg| arg == "--qa-settings-read") {
        Some(false)
    } else {
        None
    }
}

pub fn settings_test_path() -> Option<std::path::PathBuf> {
    let mut args = std::env::args_os();
    while let Some(arg) = args.next() {
        if arg == "--qa-settings-file" {
            return args.next().map(std::path::PathBuf::from);
        }
    }
    None
}

pub fn start_settings_check(ui: &AppWindow, write: bool) -> Outcome {
    let outcome = Rc::new(RefCell::new(None));
    let (weak, result) = (ui.as_weak(), outcome.clone());
    slint::Timer::single_shot(Duration::from_millis(150), move || {
        let Some(ui) = weak.upgrade() else { return };
        let check = || -> Result<(), String> {
            if !ui.get_qa_preview() || settings_test_path().is_none() {
                return Err("Settings QA requires a preview and an explicit test file".into());
            }
            if write {
                ui.invoke_language_changed(false);
                ui.invoke_mode_changed(false);
                ui.invoke_length_changed(37);
                for (index, selected) in [false, true, false, true].into_iter().enumerate() {
                    if ui.get_options().row_data(index).unwrap().selected != selected {
                        ui.invoke_toggle_type(index as i32);
                    }
                }
                ui.invoke_mode_changed(true);
                ui.invoke_length_changed(9);
                ui.invoke_length_edited("999".into());
                ui.set_about(true);
                ui.set_copied(true);
            } else {
                let saved = crate::platform::settings::load(&settings_test_path().unwrap());
                let pin = saved.mode == crate::platform::settings::Mode::Pin;
                if ui.get_chinese()
                    || ui.get_pin() != pin
                    || ui.get_length_value() != if pin { 9 } else { 37 }
                    || ui.get_length_invalid()
                    || ui.get_about()
                    || ui.get_copied()
                    || ui
                        .get_generated_password()
                        .chars()
                        .filter(|c| !c.is_whitespace())
                        .count()
                        != if pin { 9 } else { 37 }
                    || (pin
                        && !ui
                            .get_generated_password()
                            .bytes()
                            .all(|b| b.is_ascii_digit()))
                    || (ui.get_pin_progress() - if pin { 1.0 } else { 0.0 }).abs() > 0.001
                    || ui.get_language_progress().abs() > 0.001
                    || (ui.get_result_extra() - ui.get_target_result_extra()).abs() > 0.5
                {
                    return Err(
                        "Restart did not restore options or reset transient UI state".into(),
                    );
                }
                let height = ui
                    .window()
                    .size()
                    .to_logical(ui.window().scale_factor())
                    .height;
                if (height - ui.get_page_height() - 2.0 * ui.get_shadow_padding()).abs() > 1.5 {
                    return Err(format!(
                        "Restart window height {height} did not fit page {} plus shadow {} (PIN {pin})",
                        ui.get_page_height(),
                        2.0 * ui.get_shadow_padding()
                    ));
                }
                let selections: Vec<_> = ui.get_options().iter().map(|o| o.selected).collect();
                if selections != [false, true, false, true] {
                    return Err("Restart did not restore character selection".into());
                }
                ui.invoke_mode_changed(false);
                if ui.get_length_value() != 37 {
                    return Err("Restart did not restore the separate Random length".into());
                }
                ui.invoke_mode_changed(true);
                if ui.get_length_value() != 9 {
                    return Err("Restart did not restore the separate PIN length".into());
                }
                ui.invoke_mode_changed(pin);
            }
            Ok(())
        };
        *result.borrow_mut() = Some(check());
        if write && std::env::args().any(|arg| arg == "--qa-settings-wait") {
            slint::Timer::single_shot(Duration::from_millis(650), move || {
                let saved = crate::platform::settings::load(&settings_test_path().unwrap());
                if saved.mode != crate::platform::settings::Mode::Pin
                    || saved.random_length != 37
                    || saved.pin_length != 9
                {
                    *result.borrow_mut() =
                        Some(Err("Debounced settings were not saved before close".into()));
                }
                let _ = slint::quit_event_loop();
            });
            return;
        }
        // Exit before the debounce fires, exercising the synchronous close flush.
        let _ = slint::quit_event_loop();
    });
    outcome
}

pub fn start(ui: &AppWindow) -> Outcome {
    let outcome = Rc::new(RefCell::new(None));
    let (weak, result) = (ui.as_weak(), outcome.clone());
    slint::Timer::single_shot(Duration::from_millis(150), move || {
        let Some(ui) = weak.upgrade() else { return };
        if let Err(message) = check_callbacks(&ui) {
            *result.borrow_mut() = Some(Err(message));
            let _ = slint::quit_event_loop();
            return;
        }
        prepare_scene(&ui, 0);
        capture_next(ui.as_weak(), 0, result);
    });
    outcome
}

fn check_callbacks(ui: &AppWindow) -> Result<(), String> {
    if !ui.get_qa_preview() {
        return Err("Refusing to capture real password data".into());
    }
    use slint::winit_030::{WinitWindowAccessor, winit::window::WindowButtons};
    if ui
        .window()
        .with_winit_window(|w| w.enabled_buttons().contains(WindowButtons::MAXIMIZE))
        != Some(false)
    {
        return Err("Native maximize control was not disabled".into());
    }
    for index in 0..3 {
        ui.invoke_toggle_type(index);
    }
    ui.invoke_toggle_type(3);
    if ui.get_options().iter().filter(|c| c.selected).count() != 1 {
        return Err("Minimum-one selection guard failed".into());
    }
    for index in 0..3 {
        ui.invoke_toggle_type(index);
    }
    ui.invoke_length_edited("1000".into());
    if !ui.get_length_invalid() {
        return Err("Out-of-range input was accepted".into());
    }
    ui.invoke_length_committed("1000".into());
    if ui.get_length_invalid() || ui.get_length_value() != 20 {
        return Err("Invalid edit did not revert".into());
    }
    ui.invoke_length_changed(32);
    ui.invoke_mode_changed(true);
    if ui.get_length_value() != 6 {
        return Err("PIN default was not retained".into());
    }
    ui.invoke_length_changed(8);
    ui.invoke_mode_changed(false);
    if ui.get_length_value() != 32 {
        return Err("Random length was not retained".into());
    }
    ui.invoke_mode_changed(true);
    if ui.get_length_value() != 8 {
        return Err("PIN length was not retained".into());
    }
    ui.invoke_mode_changed(false);
    ui.invoke_length_changed(20);
    // Exercise the explicit Copy action with synthetic preview data only.
    click(
        ui,
        320.0 + ui.get_shadow_padding(),
        238.0 + ui.get_shadow_padding(),
    )?;
    if !ui.get_copied() {
        snapshot(
            ui,
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("work/qa/copy-check.png"),
        )?;
        return Err(format!(
            "Native clipboard copy failed (UI status: {})",
            ui.get_status()
        ));
    }
    // Clear only the synthetic UI value to prove the result frame regenerates it.
    ui.set_generated_password("".into());
    click(
        ui,
        170.0 + ui.get_shadow_padding(),
        192.0 + ui.get_shadow_padding(),
    )?;
    if ui.get_copied() || ui.get_generated_password().is_empty() {
        return Err("Clicking the result frame did not regenerate the password".into());
    }
    ui.set_generated_password("".into());
    ui.set_copied(true);
    for event in [
        slint::platform::WindowEvent::KeyPressed {
            text: slint::platform::Key::Space.into(),
        },
        slint::platform::WindowEvent::KeyReleased {
            text: slint::platform::Key::Space.into(),
        },
    ] {
        ui.window()
            .dispatch_event_with_result(event)
            .map_err(|e| e.to_string())?;
    }
    if ui.get_copied() || ui.get_generated_password().is_empty() {
        return Err("Keyboard regeneration failed".into());
    }
    let displayed = ui.get_generated_password();
    click(
        ui,
        74.0 + ui.get_shadow_padding(),
        15.0 + ui.get_shadow_padding(),
    )?;
    if !ui.get_about() {
        snapshot(
            ui,
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("work/qa/menu-check.png"),
        )?;
        return Err("Title-bar menu did not open About".into());
    }
    // The menu keeps keyboard focus after opening About; Return activates Back.
    for event in [
        slint::platform::WindowEvent::KeyPressed {
            text: slint::platform::Key::Return.into(),
        },
        slint::platform::WindowEvent::KeyReleased {
            text: slint::platform::Key::Return.into(),
        },
    ] {
        ui.window()
            .dispatch_event_with_result(event)
            .map_err(|e| e.to_string())?;
    }
    if ui.get_generated_password() != displayed || ui.get_about() {
        return Err("Keyboard Back failed or changed the current Random password".into());
    }
    Ok(())
}

fn click(ui: &AppWindow, x: f32, y: f32) -> Result<(), String> {
    use slint::platform::{PointerEventButton, WindowEvent};
    let position = slint::LogicalPosition::new(x, y);
    for event in [
        WindowEvent::PointerMoved { position },
        WindowEvent::PointerPressed {
            position,
            button: PointerEventButton::Left,
        },
        WindowEvent::PointerReleased {
            position,
            button: PointerEventButton::Left,
        },
    ] {
        ui.window()
            .dispatch_event_with_result(event)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn prepare_scene(ui: &AppWindow, scene: usize) {
    ui.set_about(false);
    ui.invoke_language_changed(scene < 3 || scene == 7 || scene == 13);
    ui.invoke_mode_changed(scene == 1 || scene == 4 || scene == 13);
    ui.invoke_length_changed(if scene == 6 {
        64
    } else if scene == 1 || scene == 4 || scene == 13 {
        6
    } else {
        20
    });
    if scene == 2 || scene == 5 || scene == 8 {
        ui.set_about(true);
    }
    if scene == 7 {
        let padding = 2.0 * ui.get_shadow_padding();
        ui.window()
            .set_size(slint::LogicalSize::new(380.0 + padding, 320.0 + padding));
    }
}

fn capture_next(weak: slint::Weak<AppWindow>, scene: usize, outcome: Outcome) {
    if scene >= 9 {
        let Some(ui) = weak.upgrade() else { return };
        let width = ui
            .window()
            .size()
            .to_logical(ui.window().scale_factor())
            .width
            - 2.0 * ui.get_shadow_padding();
        let x = match scene {
            9 => 74.0,
            10 => width - 67.0,
            12 | 13 => 20.0,
            _ => width - 22.0,
        };
        let position = slint::LogicalPosition::new(
            ui.get_shadow_padding() + x,
            ui.get_shadow_padding() + if scene >= 12 { 178.0 } else { 15.0 },
        );
        if let Err(error) = ui
            .window()
            .dispatch_event_with_result(slint::platform::WindowEvent::PointerMoved { position })
        {
            *outcome.borrow_mut() = Some(Err(error.to_string()));
            let _ = slint::quit_event_loop();
            return;
        }
    }
    let began = std::time::Instant::now();
    let position = weak.upgrade().map(|ui| ui.window().position());
    let (mid_weak, mid_outcome) = (weak.clone(), outcome.clone());
    slint::Timer::single_shot(Duration::from_millis(120), move || {
        let Some(ui) = mid_weak.upgrade() else { return };
        if (ui.get_card_top() - ui.get_shadow_padding()).abs() > 0.5 {
            *mid_outcome.borrow_mut() = Some(Err("Card top moved during transition".into()));
            let _ = slint::quit_event_loop();
            return;
        }
        if position.is_some_and(|before| before != ui.window().position()) {
            *mid_outcome.borrow_mut() =
                Some(Err("Native window top-left moved during transition".into()));
            let _ = slint::quit_event_loop();
            return;
        }
        let height = ui
            .window()
            .size()
            .to_logical(ui.window().scale_factor())
            .height
            - 2.0 * ui.get_shadow_padding();
        if (scene == 1 && !(233.0..263.0).contains(&height))
            || (scene == 2 && !(150.0..233.0).contains(&height))
        {
            *mid_outcome.borrow_mut() = Some(Err(format!(
                "Page height jumped instead of animating (scene {scene}, height {height}, page {}, elapsed {}ms)",
                ui.get_page_height(),
                began.elapsed().as_millis()
            )));
            let _ = slint::quit_event_loop();
            return;
        }
        // Sample an empty point inside the card's left padding during motion.
        // This catches fading the entire white card into the gray window backdrop.
        let bright = ui.window().take_snapshot().is_ok_and(|pixels| {
            let scale = ui.window().scale_factor();
            let x = ((ui.get_shadow_padding() + 4.0) * scale).round() as usize;
            let y = ((ui.get_shadow_padding() + 120.0) * scale).round() as usize;
            pixels
                .as_slice()
                .get(y * pixels.width() as usize + x)
                .is_some_and(|pixel| {
                    pixel.r >= 250 && pixel.g >= 250 && pixel.b >= 250 && pixel.a == 255
                })
        });
        if !bright {
            *mid_outcome.borrow_mut() = Some(Err("White card dimmed during transition".into()));
            let _ = slint::quit_event_loop();
        }
    });
    slint::Timer::single_shot(Duration::from_millis(450), move || {
        let Some(ui) = weak.upgrade() else { return };
        if (ui.get_card_top() - ui.get_shadow_padding()).abs() > 0.5 {
            *outcome.borrow_mut() = Some(Err("Card top is not anchored".into()));
            let _ = slint::quit_event_loop();
            return;
        }
        let logical_size = ui.window().size().to_logical(ui.window().scale_factor());
        let corners_clear = ui.window().take_snapshot().is_ok_and(|pixels| {
            let scale = ui.window().scale_factor();
            let offset = ((ui.get_shadow_padding() + 3.0) * scale).round() as usize;
            let width = pixels.width() as usize;
            let left = pixels.as_slice()[offset * width + offset];
            let right = pixels.as_slice()[offset * width + width - offset - 1];
            let shadow_x = ((ui.get_shadow_padding() - 6.0) * scale).round() as usize;
            let shadow = pixels.as_slice()[pixels.height() as usize / 2 * width + shadow_x];
            left.a < 100
                && right.a < 100
                && shadow.a > 8
                && shadow.a < 100
                && pixels.as_slice()[0].a == 0
        });
        if !corners_clear {
            *outcome.borrow_mut() = Some(Err(
                "Window surface lost its transparent corners or visible shadow".into(),
            ));
            let _ = slint::quit_event_loop();
            return;
        }
        if scene >= 7 && (logical_size.width - 380.0 - 2.0 * ui.get_shadow_padding()).abs() > 1.5 {
            *outcome.borrow_mut() = Some(Err(format!(
                "Compact window width was overridden: {}",
                logical_size.width
            )));
            let _ = slint::quit_event_loop();
            return;
        }
        if scene != 7
            && (logical_size.height
                - ui.get_page_height().max(150.0)
                - 2.0 * ui.get_shadow_padding())
            .abs()
                > 1.5
        {
            *outcome.borrow_mut() = Some(Err(
                "Native window did not follow the animated page height".into(),
            ));
            let _ = slint::quit_event_loop();
            return;
        }
        if (9..12).contains(&scene) {
            let width = logical_size.width - 2.0 * ui.get_shadow_padding();
            // Menu stays unpainted; system buttons still fill their parent frames.
            let points = match scene {
                9 => [(11.0, 8.0), (77.0, 22.0)],
                10 => [(width - 87.0, 2.0), (width - 47.0, 28.0)],
                _ => [(width - 42.0, 2.0), (width - 2.0, 28.0)],
            };
            let visible_hover = ui.window().take_snapshot().is_ok_and(|pixels| {
                let scale = ui.window().scale_factor();
                points.into_iter().all(|(x, y)| {
                    let x = ((ui.get_shadow_padding() + x) * scale).round() as usize;
                    let y = ((ui.get_shadow_padding() + y) * scale).round() as usize;
                    let pixel = pixels.as_slice()[y * pixels.width() as usize + x];
                    if scene == 9 {
                        pixel.r == 243 && pixel.g == 244 && pixel.b == 247 && pixel.a == 255
                    } else if scene == 11 {
                        pixel.r > 180 && pixel.g < 100 && pixel.b < 110 && pixel.a == 255
                    } else {
                        pixel.r < 240 && pixel.g < 240 && pixel.b < 245 && pixel.a == 255
                    }
                })
            });
            if !visible_hover {
                *outcome.borrow_mut() = Some(Err(format!(
                    "Title-bar hover appearance was incorrect in scene {scene}"
                )));
                let _ = slint::quit_event_loop();
                return;
            }
        }
        if scene >= 12 {
            if scene == 13 {
                ui.set_generated_password("".into());
                ui.set_copied(true);
                if let Err(error) = click(
                    &ui,
                    170.0 + ui.get_shadow_padding(),
                    162.0 + ui.get_shadow_padding(),
                ) {
                    *outcome.borrow_mut() = Some(Err(error));
                    let _ = slint::quit_event_loop();
                    return;
                }
                let result = ui.get_generated_password();
                if ui.get_copied()
                    || result.len() != 6
                    || !result.bytes().all(|byte| byte.is_ascii_digit())
                {
                    *outcome.borrow_mut() = Some(Err("PIN result click did not regenerate".into()));
                    let _ = slint::quit_event_loop();
                    return;
                }
            }
            let hover_correct = ui.window().take_snapshot().is_ok_and(|pixels| {
                let scale = ui.window().scale_factor();
                let top = if scene == 13 { 142.0 } else { 172.0 };
                let x = ((ui.get_shadow_padding() + 20.0) * scale).round() as usize;
                let y = ((ui.get_shadow_padding() + top + 10.0) * scale).round() as usize;
                let pixel = pixels.as_slice()[y * pixels.width() as usize + x];
                pixel.r == 234 && pixel.g == 242 && pixel.b == 255 && pixel.a == 255
            });
            if !hover_correct {
                *outcome.borrow_mut() = Some(Err("Result hover did not use #EAF2FF".into()));
                let _ = slint::quit_event_loop();
                return;
            }
        }
        let names = [
            "random-zh",
            "pin-zh",
            "about-zh",
            "random-en",
            "pin-en",
            "about-en",
            "long-password",
            "compact-zh",
            "compact-about-en",
            "menu-no-hover",
            "hover-minimize",
            "hover-close",
            "hover-result-random",
            "hover-result-pin",
        ];
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("work/qa")
            .join(format!("{}.png", names[scene]));
        if let Err(message) = snapshot(&ui, &path) {
            *outcome.borrow_mut() = Some(Err(message));
            let _ = slint::quit_event_loop();
            return;
        }
        if scene == names.len() - 1 {
            *outcome.borrow_mut() = Some(Ok(()));
            let _ = slint::quit_event_loop();
        } else {
            prepare_scene(&ui, scene + 1);
            capture_next(ui.as_weak(), scene + 1, outcome);
        }
    });
}

fn snapshot(ui: &AppWindow, path: &Path) -> Result<(), String> {
    if !ui.get_qa_preview() {
        return Err("Preview mode required".into());
    }
    std::fs::create_dir_all(path.parent().ok_or("Missing QA directory")?)
        .map_err(|e| e.to_string())?;
    let pixels = ui.window().take_snapshot().map_err(|e| e.to_string())?;
    let file = File::create(path).map_err(|e| e.to_string())?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), pixels.width(), pixels.height());
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer
        .write_image_data(pixels.as_bytes())
        .map_err(|e| e.to_string())?;
    Ok(())
}
