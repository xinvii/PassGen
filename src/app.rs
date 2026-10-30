use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::Rc,
    time::Duration,
};

use passgen::core::{self, CharacterTypes, GenerateError};
use slint::{ComponentHandle, ModelRc, VecModel};
use zeroize::Zeroizing;

use crate::platform::settings::{self, Language, Mode, Settings};
use crate::{AppWindow, CharacterOption, platform::clipboard::ClipboardService};

struct State {
    chinese: bool,
    types: CharacterTypes,
    pin: bool,
    random_length: usize,
    pin_length: usize,
    secret: Zeroizing<String>,
    clipboard: ClipboardService,
    copy_epoch: u64,
    settings_path: Option<PathBuf>,
    saved_settings: Rc<Cell<Settings>>,
    save_timer: slint::Timer,
    placement: crate::platform::window::Placement,
}

// Only debug builds can opt into deterministic, non-secret UI preview data.
// This type and the CLI switch are absent from release builds.
#[cfg(debug_assertions)]
struct PreviewRandom(u8);

#[cfg(debug_assertions)]
impl core::RandomSource for PreviewRandom {
    fn fill(&mut self, bytes: &mut [u8]) -> Result<(), GenerateError> {
        for byte in bytes {
            *byte = self.0;
            self.0 = self.0.wrapping_add(73);
        }
        Ok(())
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            chinese: true,
            types: CharacterTypes::ALL,
            pin: false,
            random_length: 20,
            pin_length: 6,
            secret: Zeroizing::new(String::new()),
            clipboard: ClipboardService::default(),
            copy_epoch: 0,
            settings_path: None,
            saved_settings: Rc::new(Cell::new(Settings::default())),
            save_timer: slint::Timer::default(),
            placement: crate::platform::window::Placement::default(),
        }
    }
}

impl State {
    fn from_settings(settings: Settings, path: Option<PathBuf>) -> Self {
        let settings = settings.validated();
        let mut state = Self {
            chinese: settings.language == Language::Chinese,
            pin: settings.mode == Mode::Pin,
            random_length: settings.random_length,
            pin_length: settings.pin_length,
            settings_path: path,
            saved_settings: Rc::new(Cell::new(settings)),
            placement: crate::platform::window::Placement::new(settings.window_position),
            ..Self::default()
        };
        for (index, selected) in settings.character_types.into_iter().enumerate() {
            if !selected {
                state.types.toggle(index);
            }
        }
        state
    }

    fn settings(&self) -> Settings {
        Settings {
            language: if self.chinese {
                Language::Chinese
            } else {
                Language::English
            },
            mode: if self.pin { Mode::Pin } else { Mode::Random },
            random_length: self.random_length,
            pin_length: self.pin_length,
            character_types: std::array::from_fn(|index| self.types.contains(index)),
            window_position: self.placement.position(),
            ..Settings::default()
        }
    }

    fn remember(&self, ui: &AppWindow) {
        let Some(path) = self.settings_path.clone() else {
            return;
        };
        self.save_timer.stop();
        let settings = self.settings();
        if settings == self.saved_settings.get() {
            return;
        }
        let (saved, weak) = (self.saved_settings.clone(), ui.as_weak());
        self.save_timer.start(
            slint::TimerMode::SingleShot,
            Duration::from_millis(400),
            move || {
                if settings::save(&path, settings).is_ok() {
                    saved.set(settings);
                } else if let Some(ui) = weak.upgrade()
                    && ui.get_status().is_empty()
                {
                    ui.set_status(
                        if ui.get_chinese() {
                            "无法保存设置。"
                        } else {
                            "Unable to save settings."
                        }
                        .into(),
                    );
                }
            },
        );
    }

    fn flush_settings(&self) -> std::io::Result<()> {
        self.save_timer.stop();
        let settings = self.settings();
        if let Some(path) = &self.settings_path
            && settings != self.saved_settings.get()
        {
            settings::save(path, settings)?;
            self.saved_settings.set(settings);
        }
        Ok(())
    }

    fn length(&self) -> usize {
        if self.pin {
            self.pin_length
        } else {
            self.random_length
        }
    }

    fn maximum(&self) -> usize {
        if self.pin {
            core::MAX_PIN_LENGTH
        } else {
            core::MAX_RANDOM_LENGTH
        }
    }

    fn set_length(&mut self, length: usize) {
        if self.pin {
            self.pin_length = length;
        } else {
            self.random_length = length;
        }
    }

    fn sync(&self, ui: &AppWindow, sync_input: bool) {
        ui.set_chinese(self.chinese);
        ui.set_pin(self.pin);
        ui.set_length_value(self.length() as i32);
        if sync_input {
            ui.set_length_text(self.length().to_string().into());
            ui.set_length_invalid(false);
        }
        let labels = if ui.get_chinese() {
            [
                "小写字母 (a-z)",
                "大写字母 (A-Z)",
                "数字 (0-9)",
                "特殊符号 (!@#$)",
            ]
        } else {
            [
                "Lowercase (a-z)",
                "Uppercase (A-Z)",
                "Numbers (0-9)",
                "Symbols (!@#$)",
            ]
        };
        let short = ["a-z", "A-Z", "0-9", "!@#$"];
        ui.set_options(ModelRc::new(VecModel::from(
            (0..4)
                .map(|index| CharacterOption {
                    label: labels[index].into(),
                    short_label: short[index].into(),
                    selected: self.types.contains(index),
                })
                .collect::<Vec<_>>(),
        )));
    }

    fn generate(&mut self, ui: &AppWindow) {
        // Drop our previous secret and release the UI reference before replacing it.
        ui.set_generated_password("".into());
        ui.set_can_copy(false);
        ui.set_copied(false);
        ui.set_status("".into());
        self.copy_epoch = self.copy_epoch.wrapping_add(1);
        self.secret = Zeroizing::new(String::new());
        #[cfg(debug_assertions)]
        let result = if ui.get_qa_preview() {
            core::generate_with(self.length(), self.pin, self.types, &mut PreviewRandom(19))
        } else {
            core::generate(self.length(), self.pin, self.types)
        };
        #[cfg(not(debug_assertions))]
        let result = core::generate(self.length(), self.pin, self.types);
        match result {
            Ok(secret) => {
                let mut display = Zeroizing::new(String::with_capacity(secret.len() + 1));
                for (index, chunk) in secret.as_bytes().chunks(32).enumerate() {
                    if index > 0 {
                        display.push('\n');
                    }
                    // The generator's alphabet is ASCII by construction.
                    display.push_str(std::str::from_utf8(chunk).expect("ASCII password"));
                }
                ui.set_generated_password(display.as_str().into());
                self.secret = secret;
                ui.set_can_copy(true);
            }
            Err(error) => ui.set_status(
                match error {
                    GenerateError::RandomUnavailable => {
                        if ui.get_chinese() {
                            "系统随机源不可用，请重试。"
                        } else {
                            "The system random source is unavailable. Please retry."
                        }
                    }
                    GenerateError::InvalidLength => {
                        if ui.get_chinese() {
                            "密码长度无效。"
                        } else {
                            "Invalid password length."
                        }
                    }
                    GenerateError::EmptyAlphabet => {
                        if ui.get_chinese() {
                            "请至少选择一种字符类型。"
                        } else {
                            "Select at least one character type."
                        }
                    }
                }
                .into(),
            ),
        }
    }
}

pub fn run() -> Result<(), slint::PlatformError> {
    #[cfg(debug_assertions)]
    let preview = std::env::args().any(|arg| arg == "--qa-preview" || arg == "--qa-snapshots");
    #[cfg(not(debug_assertions))]
    let preview = false;
    let settings_path = if preview {
        None
    } else {
        settings::default_path()
    };
    #[cfg(all(debug_assertions, feature = "ui-qa"))]
    let settings_path = if preview
        && (crate::qa::settings_check_mode().is_some()
            || crate::qa::position_check_mode().is_some())
    {
        crate::qa::settings_test_path()
    } else {
        settings_path
    };
    let preferences = settings_path
        .as_deref()
        .map(settings::load)
        .unwrap_or_default();
    let initial_position = preferences.window_position;
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("software".into())
        .with_winit_window_attributes_hook(move |attributes| {
            crate::platform::window::attributes(attributes, initial_position)
        })
        .select()?;
    let ui = AppWindow::new()?;
    ui.set_app_version(env!("CARGO_PKG_VERSION").into());
    ui.set_qa_preview(preview);
    {
        let weak = ui.as_weak();
        ui.on_begin_resize(move |direction| {
            if let Some(ui) = weak.upgrade() {
                crate::platform::window::begin_resize(ui.window(), direction);
            }
        });
    }
    let state = Rc::new(RefCell::new(State::from_settings(
        preferences,
        settings_path,
    )));
    state.borrow().sync(&ui, true);
    crate::motion::connect(&ui);
    // Choose the initial native height before showing the restored mode.
    ui.set_initial_page_height(ui.get_page_height());
    let placement = state.borrow().placement.clone();
    crate::chrome::connect(&ui, placement.clone());
    {
        let weak = ui.as_weak();
        slint::spawn_local(async move {
            if let Some(ui) = weak.upgrade() {
                let _ = crate::platform::window::initialize(ui.window(), &placement).await;
                crate::chrome::refresh(&ui);
                ui.invoke_animate_window_size();
            }
        })
        .map_err(|e| slint::PlatformError::Other(e.to_string()))?;
    }

    {
        let (weak, state) = (ui.as_weak(), state.clone());
        ui.on_regenerate(move || {
            if let Some(ui) = weak.upgrade() {
                let mut state = state.borrow_mut();
                state.sync(&ui, true);
                state.generate(&ui);
            }
        });
    }
    {
        let (weak, state) = (ui.as_weak(), state.clone());
        ui.on_mode_changed(move |pin| {
            if let Some(ui) = weak.upgrade() {
                let mut state = state.borrow_mut();
                if state.pin != pin {
                    state.pin = pin;
                    state.sync(&ui, true);
                    state.generate(&ui);
                    state.remember(&ui);
                }
            }
        });
    }
    {
        let (weak, state) = (ui.as_weak(), state.clone());
        ui.on_toggle_type(move |index| {
            if let Some(ui) = weak.upgrade() {
                let mut state = state.borrow_mut();
                if state.types.toggle(index as usize) {
                    state.sync(&ui, false);
                    state.generate(&ui);
                    state.remember(&ui);
                } else {
                    ui.set_status(
                        if ui.get_chinese() {
                            "至少保留一种字符类型。"
                        } else {
                            "Keep at least one character type selected."
                        }
                        .into(),
                    );
                }
            }
        });
    }
    {
        let (weak, state) = (ui.as_weak(), state.clone());
        ui.on_length_changed(move |length| {
            if let Some(ui) = weak.upgrade() {
                let mut state = state.borrow_mut();
                let length = (length.max(core::MIN_LENGTH as i32) as usize).min(state.maximum());
                if state.length() != length {
                    state.set_length(length);
                    state.sync(&ui, true);
                    state.generate(&ui);
                    state.remember(&ui);
                }
            }
        });
    }
    {
        let (weak, state) = (ui.as_weak(), state.clone());
        ui.on_length_edited(move |value| {
            if let Some(ui) = weak.upgrade() {
                let mut state = state.borrow_mut();
                let parsed = value.parse::<usize>().ok();
                let valid = parsed.filter(|v| (core::MIN_LENGTH..=state.maximum()).contains(v));
                ui.set_length_invalid(valid.is_none());
                if let Some(length) = valid
                    && state.length() != length
                {
                    state.set_length(length);
                    state.sync(&ui, false);
                    state.generate(&ui);
                    state.remember(&ui);
                }
            }
        });
    }
    {
        let (weak, state) = (ui.as_weak(), state.clone());
        ui.on_length_committed(move |_| {
            if let Some(ui) = weak.upgrade() {
                // An incomplete or out-of-range edit reverts to the last valid value.
                state.borrow().sync(&ui, true);
            }
        });
    }
    {
        let (weak, state) = (ui.as_weak(), state.clone());
        ui.on_language_changed(move |chinese| {
            if let Some(ui) = weak.upgrade() {
                let mut state = state.borrow_mut();
                state.chinese = chinese;
                ui.set_status("".into());
                state.sync(&ui, true);
                state.remember(&ui);
            }
        });
    }
    {
        let (weak, state) = (ui.as_weak(), state.clone());
        ui.on_return_to_generator(move || {
            if let Some(ui) = weak.upgrade() {
                let mut state = state.borrow_mut();
                let mode_changed = state.pin;
                state.pin = false;
                state.sync(&ui, true);
                if mode_changed {
                    state.generate(&ui);
                } else {
                    ui.set_status("".into());
                }
                ui.set_about(false);
                state.remember(&ui);
            }
        });
    }
    {
        let (weak, state) = (ui.as_weak(), state.clone());
        ui.on_copy_password(move || {
            if let Some(ui) = weak.upgrade() {
                let mut current = state.borrow_mut();
                if current.secret.is_empty() {
                    return;
                }
                let State {
                    clipboard, secret, ..
                } = &mut *current;
                if clipboard.copy(secret).is_ok() {
                    ui.set_copied(true);
                    ui.set_status("".into());
                    current.copy_epoch = current.copy_epoch.wrapping_add(1);
                    let epoch = current.copy_epoch;
                    let (weak, state) = (weak.clone(), Rc::downgrade(&state));
                    slint::Timer::single_shot(Duration::from_millis(1500), move || {
                        if let (Some(ui), Some(state)) = (weak.upgrade(), state.upgrade())
                            && state.borrow().copy_epoch == epoch
                        {
                            ui.set_copied(false);
                        }
                    });
                } else {
                    ui.set_status(
                        if ui.get_chinese() {
                            "暂时无法访问剪贴板，请重试。"
                        } else {
                            "The clipboard is unavailable. Please retry."
                        }
                        .into(),
                    );
                }
            }
        });
    }

    state.borrow().sync(&ui, true);
    state.borrow_mut().generate(&ui);
    #[cfg(all(debug_assertions, feature = "ui-qa"))]
    let qa_result = if let Some(mode) = crate::qa::position_check_mode() {
        Some(crate::qa::start_position_check(&ui, mode))
    } else if let Some(write) = crate::qa::settings_check_mode() {
        Some(crate::qa::start_settings_check(&ui, write))
    } else {
        std::env::args()
            .any(|arg| arg == "--qa-snapshots")
            .then(|| crate::qa::start(&ui))
    };
    let run_result = ui.run();
    state.borrow().placement.capture_before_close(ui.window());
    ui.set_generated_password("".into());
    state.borrow_mut().secret = Zeroizing::new(String::new());
    state
        .borrow()
        .flush_settings()
        .map_err(|_| slint::PlatformError::Other("Unable to save settings.".into()))?;
    run_result?;
    #[cfg(all(debug_assertions, feature = "ui-qa"))]
    if let Some(result) = qa_result {
        match result.borrow_mut().take() {
            Some(Ok(())) if crate::qa::position_check_mode().is_some() => {
                println!("Window position QA passed.")
            }
            Some(Ok(())) if crate::qa::settings_check_mode().is_some() => {
                println!("Settings restart QA passed.")
            }
            Some(Ok(())) => println!("UI QA passed: callbacks and fourteen preview snapshots."),
            Some(Err(message)) => return Err(slint::PlatformError::Other(message)),
            None => return Err(slint::PlatformError::Other("UI QA did not complete".into())),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restoring_options_keeps_password_and_transient_state_empty() {
        let settings = Settings {
            language: Language::English,
            mode: Mode::Pin,
            random_length: 48,
            pin_length: 10,
            character_types: [false, true, false, true],
            ..Settings::default()
        };
        let state = State::from_settings(settings, None);
        assert_eq!(state.settings(), settings);
        assert!(state.secret.is_empty());
        assert_eq!(state.copy_epoch, 0);
        assert!(!state.save_timer.running());
    }

    #[test]
    fn mode_lengths_are_independent() {
        let mut state = State::default();
        state.set_length(32);
        state.pin = true;
        assert_eq!(state.length(), 6);
        assert_eq!(state.maximum(), core::MAX_PIN_LENGTH);
        state.set_length(8);
        state.pin = false;
        assert_eq!(state.length(), 32);
        assert_eq!(state.maximum(), core::MAX_RANDOM_LENGTH);
        state.pin = true;
        assert_eq!(state.length(), 8);
    }

    #[test]
    fn changing_modes_keeps_character_choices() {
        let mut state = State::default();
        state.types.toggle(0);
        state.types.toggle(3);
        let types = state.types;
        state.pin = true;
        state.set_length(10);
        state.pin = false;
        assert_eq!(state.types, types);
    }
}
