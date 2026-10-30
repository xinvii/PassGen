//! Only non-secret preferences can cross this persistence boundary.

use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use passgen::core::{MAX_PIN_LENGTH, MAX_RANDOM_LENGTH, MIN_LENGTH};
use serde::{Deserialize, Serialize};

const MAX_FILE_BYTES: u64 = 8192;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub enum Language {
    #[serde(rename = "en")]
    English,
    #[default]
    #[serde(rename = "zh-CN")]
    Chinese,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Random,
    Pin,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct WindowPosition {
    pub x: i32,
    pub y: i32,
}

fn optional_position<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<WindowPosition>, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).unwrap_or(None))
}

// Deliberately separate from app::State: never serialize passwords or UI state.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct Settings {
    pub version: u8,
    pub language: Language,
    pub mode: Mode,
    pub random_length: usize,
    pub pin_length: usize,
    pub character_types: [bool; 4],
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "optional_position"
    )]
    pub window_position: Option<WindowPosition>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            language: Language::Chinese,
            mode: Mode::Random,
            random_length: 20,
            pin_length: 6,
            character_types: [true; 4],
            window_position: None,
        }
    }
}

impl Settings {
    pub fn validated(mut self) -> Self {
        let defaults = Self::default();
        if self.version != defaults.version {
            return defaults;
        }
        if !(MIN_LENGTH..=MAX_RANDOM_LENGTH).contains(&self.random_length) {
            self.random_length = defaults.random_length;
        }
        if !(MIN_LENGTH..=MAX_PIN_LENGTH).contains(&self.pin_length) {
            self.pin_length = defaults.pin_length;
        }
        if !self.character_types.iter().any(|selected| *selected) {
            self.character_types = defaults.character_types;
        }
        self
    }
}

pub fn default_path() -> Option<PathBuf> {
    let variable = |name| std::env::var_os(name).filter(|value| !value.is_empty());
    #[cfg(target_os = "windows")]
    let root = variable("LOCALAPPDATA").map(PathBuf::from).or_else(|| {
        variable("USERPROFILE").map(|profile| PathBuf::from(profile).join("AppData/Local"))
    });
    #[cfg(target_os = "macos")]
    let root = variable("HOME").map(|home| PathBuf::from(home).join("Library/Application Support"));
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let root = variable("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| variable("HOME").map(|home| PathBuf::from(home).join(".config")));
    root.map(|root| root.join("PassGen/settings.json"))
}

pub fn load(path: &Path) -> Settings {
    let read = || -> io::Result<Settings> {
        let mut bytes = Vec::new();
        File::open(path)?
            .take(MAX_FILE_BYTES + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_FILE_BYTES {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let settings: Settings = serde_json::from_slice(&bytes)?;
        Ok(settings.validated())
    };
    read().unwrap_or_default()
}

pub fn save(path: &Path, settings: Settings) -> io::Result<()> {
    let directory = path.parent().ok_or(io::ErrorKind::InvalidInput)?;
    fs::create_dir_all(directory)?;
    let mut pending = tempfile::NamedTempFile::new_in(directory)?;
    serde_json::to_writer_pretty(pending.as_file_mut(), &settings.validated())?;
    pending.write_all(b"\n")?;
    pending.as_file().sync_all()?;
    pending.persist(path).map_err(|error| error.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_all_character_selections_and_overwrite() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("PassGen/settings.json");
        for mask in 1..16 {
            let settings = Settings {
                language: Language::English,
                mode: Mode::Pin,
                random_length: 48,
                pin_length: 10,
                character_types: std::array::from_fn(|index| mask & (1 << index) != 0),
                window_position: Some(WindowPosition { x: -1200, y: 180 }),
                ..Settings::default()
            };
            save(&path, settings).unwrap();
            assert_eq!(load(&path), settings);
        }
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
    }

    #[test]
    fn missing_corrupt_large_and_unknown_version_use_defaults() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("settings.json");
        assert_eq!(load(&path), Settings::default());
        for content in [
            "{truncated".to_owned(),
            " ".repeat(MAX_FILE_BYTES as usize + 1),
            r#"{"version":99,"random_length":32}"#.to_owned(),
            r#"{"language":"unknown"}"#.to_owned(),
        ] {
            fs::write(&path, content).unwrap();
            assert_eq!(load(&path), Settings::default());
        }
    }

    #[test]
    fn invalid_values_recover_without_discarding_valid_preferences() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("settings.json");
        fs::write(
            &path,
            r#"{"language":"en","mode":"pin","random_length":999,"pin_length":0,"character_types":[false,false,false,false]}"#,
        )
        .unwrap();
        assert_eq!(
            load(&path),
            Settings {
                language: Language::English,
                mode: Mode::Pin,
                ..Settings::default()
            }
        );
    }

    #[test]
    fn serialized_keys_are_only_non_secret_preferences() {
        let value = serde_json::to_value(Settings {
            window_position: Some(WindowPosition { x: 100, y: 200 }),
            ..Settings::default()
        })
        .unwrap();
        let mut keys: Vec<_> = value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "character_types",
                "language",
                "mode",
                "pin_length",
                "random_length",
                "version",
                "window_position"
            ]
        );
    }

    #[test]
    fn save_failure_preserves_existing_file() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("not-a-directory");
        fs::write(&path, "unchanged").unwrap();
        assert!(save(&path.join("settings.json"), Settings::default()).is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), "unchanged");
    }

    #[test]
    fn missing_or_invalid_position_preserves_existing_options() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("settings.json");
        for position in [
            None,
            Some("null"),
            Some("\"invalid\""),
            Some(r#"{"x":1}"#),
            Some(r#"{"x":999999999999,"y":2}"#),
        ] {
            let extra = position
                .map(|p| format!(",\"window_position\":{p}"))
                .unwrap_or_default();
            fs::write(
                &path,
                format!("{{\"language\":\"en\",\"random_length\":32{extra}}}"),
            )
            .unwrap();
            let settings = load(&path);
            assert_eq!(settings.language, Language::English);
            assert_eq!(settings.random_length, 32);
            assert_eq!(settings.window_position, None);
        }
    }
}
