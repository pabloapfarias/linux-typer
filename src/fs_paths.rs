use std::env;
use std::path::{Path, PathBuf};

use crate::cli::WhisperModel;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct AppPaths {
    project_root: PathBuf,
    config_path: PathBuf,
    data_dir: PathBuf,
    whisper_dir: PathBuf,
    temp_dir: PathBuf,
}

impl AppPaths {
    pub fn discover() -> AppResult<Self> {
        let project_root = discover_project_root(env::current_dir()?);
        let config_path = project_root.join("config.toml");

        let data_root = dirs::data_local_dir().ok_or_else(|| {
            AppError::Unsupported("could not resolve local data directory".into())
        })?;
        let data_dir = data_root.join("linux-voice-typer");
        let whisper_dir = data_dir.join("whisper.cpp");
        let temp_dir = data_dir.join("temp");

        Ok(Self {
            project_root,
            config_path,
            data_dir,
            whisper_dir,
            temp_dir,
        })
    }

    pub fn ensure_base_dirs(&self) -> AppResult<()> {
        std::fs::create_dir_all(&self.data_dir)?;
        std::fs::create_dir_all(&self.temp_dir)?;
        Ok(())
    }

    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    pub fn config_path(&self) -> &Path {
        &self.config_path
    }
    pub fn whisper_dir(&self) -> &Path {
        &self.whisper_dir
    }

    pub fn whisper_build_dir(&self) -> PathBuf {
        self.whisper_dir.join("build")
    }

    pub fn whisper_bin_path(&self) -> PathBuf {
        self.whisper_dir.join("build/bin/whisper-cli")
    }

    pub fn whisper_models_dir(&self) -> PathBuf {
        self.whisper_dir.join("models")
    }

    pub fn model_path(&self, model: WhisperModel) -> PathBuf {
        self.whisper_models_dir()
            .join(format!("ggml-{}.bin", model.as_str()))
    }

    pub fn temp_dir(&self) -> &Path {
        &self.temp_dir
    }
}

fn discover_project_root(start: PathBuf) -> PathBuf {
    for candidate in start.ancestors() {
        if candidate.join("config.example.toml").is_file()
            && candidate.join("src").join("main.rs").is_file()
        {
            return candidate.to_path_buf();
        }
    }

    start
}
