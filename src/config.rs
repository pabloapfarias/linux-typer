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
            hotkey: "Ctrl+Alt+Space".into(),
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
            || merged.temp_dir == PathBuf::from("/tmp/linux-voice-typer")
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
