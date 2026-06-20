use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("toml parse error: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("toml serialize error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("audio error: {0}")]
    Audio(#[from] cpal::Error),

    #[error("wav error: {0}")]
    Wav(#[from] hound::Error),

    #[error("hotkey error: {0}")]
    Hotkey(#[from] keytap::Error),

    #[error("dialog error: {0}")]
    Dialog(#[from] dialoguer::Error),

    #[error("invalid config: {0}")]
    InvalidConfig(String),

    #[error("unsupported feature: {0}")]
    Unsupported(String),

    #[error("dependency missing: {0}")]
    MissingDependency(String),

    #[error("command failed: {0}")]
    CommandFailed(String),
}

pub type AppResult<T> = Result<T, AppError>;
