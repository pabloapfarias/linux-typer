use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::cli::WhisperModel;
use crate::error::{AppError, AppResult};
use crate::fs_paths::AppPaths;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PasteBackend {
    Auto,
    Wtype,
    Ydotool,
    Uinput,
    None,
}

impl PasteBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Wtype => "wtype",
            Self::Ydotool => "ydotool",
            Self::Uinput => "uinput",
            Self::None => "none",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PasteShortcut {
    CtrlV,
    CtrlShiftV,
}

impl PasteShortcut {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CtrlV => "ctrl_v",
            Self::CtrlShiftV => "ctrl_shift_v",
        }
    }
}

fn default_paste_backend() -> PasteBackend {
    PasteBackend::Auto
}

fn default_paste_shortcut() -> PasteShortcut {
    PasteShortcut::CtrlV
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub hotkey: String,
    pub language: String,
    pub model_path: PathBuf,
    pub whisper_bin: PathBuf,
    pub insert_mode: String,
    pub restore_clipboard: bool,
    pub auto_paste: bool,
    #[serde(default = "default_paste_backend")]
    pub paste_backend: PasteBackend,
    #[serde(default = "default_paste_shortcut")]
    pub paste_shortcut: PasteShortcut,
    pub trim_text: bool,
    pub temp_dir: PathBuf,
    #[serde(default = "default_true")]
    pub auto_start_service: bool,
    #[serde(default)]
    pub start_minimized: bool,
    #[serde(default)]
    pub start_with_system: bool,
    #[serde(default = "default_true")]
    pub close_to_tray: bool,
    #[serde(default = "default_true")]
    pub osd_enabled: bool,
    #[serde(default)]
    pub input_device: Option<String>,
}

impl Config {
    pub fn load(config_path: &Path) -> AppResult<Self> {
        let config = Self::load_unvalidated(config_path)?;
        config.validate()?;
        Ok(config)
    }

    pub fn load_unvalidated(config_path: &Path) -> AppResult<Self> {
        let raw = fs::read_to_string(config_path)?;
        let config: Self = toml::from_str(&raw)?;
        Ok(config)
    }

    pub fn load_optional(config_path: &Path) -> AppResult<Option<Self>> {
        if !config_path.exists() {
            return Ok(None);
        }

        Ok(Some(Self::load_unvalidated(config_path)?))
    }

    pub fn save(&self, config_path: &Path) -> AppResult<()> {
        let data = toml::to_string_pretty(self)?;
        fs::write(config_path, data)?;
        Ok(())
    }

    pub fn default_for_paths(paths: &AppPaths, model: WhisperModel) -> Self {
        Self {
            hotkey: "Control+Alt+Space".into(),
            language: "pt".into(),
            model_path: paths.model_path(model),
            whisper_bin: paths.whisper_bin_path(),
            insert_mode: "clipboard".into(),
            restore_clipboard: false,
            auto_paste: true,
            paste_backend: PasteBackend::Auto,
            paste_shortcut: PasteShortcut::CtrlV,
            trim_text: true,
            temp_dir: paths.temp_dir().to_path_buf(),
            auto_start_service: true,
            start_minimized: false,
            start_with_system: false,
            close_to_tray: true,
            osd_enabled: true,
            input_device: None,
        }
    }

    pub fn effective_paste_backend(&self) -> PasteBackend {
        if !self.auto_paste && self.paste_backend == PasteBackend::Auto {
            PasteBackend::None
        } else {
            self.paste_backend
        }
    }

    pub fn effective_paste_shortcut(&self) -> PasteShortcut {
        self.paste_shortcut
    }

    pub fn merge_runtime_defaults(&self, desired: &Self) -> (Self, bool) {
        let mut merged = self.clone();
        let mut changed = false;

        if merged.hotkey.trim().is_empty() {
            merged.hotkey = desired.hotkey.clone();
            changed = true;
        }

        if merged.language.trim().is_empty() {
            merged.language = desired.language.clone();
            changed = true;
        }

        if merged.insert_mode.trim().is_empty() || merged.insert_mode != "clipboard" {
            merged.insert_mode = desired.insert_mode.clone();
            changed = true;
        }

        if merged.model_path.as_os_str().is_empty() || !merged.model_path.exists() {
            merged.model_path = desired.model_path.clone();
            changed = true;
        }

        if merged.whisper_bin.as_os_str().is_empty() || !merged.whisper_bin.exists() {
            merged.whisper_bin = desired.whisper_bin.clone();
            changed = true;
        }

        if merged.temp_dir.as_os_str().is_empty()
            || !merged.temp_dir.exists()
            || merged.temp_dir == Path::new("/tmp/linux-voice-typer")
        {
            merged.temp_dir = desired.temp_dir.clone();
            changed = true;
        }

        (merged, changed)
    }

    pub fn validate(&self) -> AppResult<()> {
        if self.hotkey.trim().is_empty() {
            return Err(AppError::InvalidConfig("hotkey cannot be empty".into()));
        }

        if self.language.trim().is_empty() {
            return Err(AppError::InvalidConfig("language cannot be empty".into()));
        }

        if self.insert_mode.trim() != "clipboard" {
            return Err(AppError::InvalidConfig(
                "only insert_mode = \"clipboard\" is supported in this MVP".into(),
            ));
        }

        if !self.model_path.exists() {
            return Err(AppError::InvalidConfig(format!(
                "model_path does not exist: {}",
                self.model_path.display()
            )));
        }

        if !self.whisper_bin.exists() {
            return Err(AppError::InvalidConfig(format!(
                "whisper_bin does not exist: {}",
                self.whisper_bin.display()
            )));
        }

        if !self.whisper_bin.is_file() {
            return Err(AppError::InvalidConfig(format!(
                "whisper_bin is not a file: {}",
                self.whisper_bin.display()
            )));
        }

        fs::create_dir_all(&self.temp_dir)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_for_tests() -> Config {
        Config {
            hotkey: "F7".into(),
            language: "pt".into(),
            model_path: PathBuf::from("/nonexistent/model.bin"),
            whisper_bin: PathBuf::from("/nonexistent/whisper-cli"),
            insert_mode: "clipboard".into(),
            restore_clipboard: false,
            auto_paste: true,
            paste_backend: PasteBackend::Auto,
            paste_shortcut: PasteShortcut::CtrlV,
            trim_text: true,
            temp_dir: PathBuf::from("/tmp/lvt-tests"),
            auto_start_service: true,
            start_minimized: false,
            start_with_system: false,
            close_to_tray: true,
            osd_enabled: true,
            input_device: None,
        }
    }

    #[test]
    fn test_effective_paste_backend_respects_auto_paste() {
        let mut config = config_for_tests();
        assert_eq!(config.effective_paste_backend(), PasteBackend::Auto);

        config.auto_paste = false;
        assert_eq!(config.effective_paste_backend(), PasteBackend::None);

        config.paste_backend = PasteBackend::Uinput;
        assert_eq!(config.effective_paste_backend(), PasteBackend::Uinput);
    }

    #[test]
    fn test_merge_runtime_defaults_fills_missing_values() {
        let mut current = config_for_tests();
        current.hotkey = "".into();
        current.language = "".into();
        current.insert_mode = "weird".into();

        let mut desired = config_for_tests();
        desired.hotkey = "F9".into();
        desired.language = "en".into();

        let (merged, changed) = current.merge_runtime_defaults(&desired);
        assert!(changed);
        assert_eq!(merged.hotkey, "F9");
        assert_eq!(merged.language, "en");
        assert_eq!(merged.insert_mode, "clipboard");
    }

    #[test]
    fn test_merge_runtime_defaults_keeps_non_empty_fields() {
        let mut desired = config_for_tests();
        desired.hotkey = "F9".into();
        desired.language = "en".into();

        let (merged, _) = config_for_tests().merge_runtime_defaults(&desired);
        assert_eq!(merged.hotkey, "F7");
        assert_eq!(merged.language, "pt");
    }
}
