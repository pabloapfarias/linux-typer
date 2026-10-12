pub mod audio;
pub mod autostart;
pub mod cli;
pub mod clipboard;
pub mod config;
pub mod doctor;
pub mod downloader;
pub mod error;
pub mod fs_paths;
pub mod hotkey;
pub mod injector;
pub mod notify;
pub mod platform;
pub mod service;
pub mod setup;
pub mod transcriber;
pub mod uinput;

use std::fs;
use std::path::Path;

use log::warn;

use crate::audio::Recorder;
use crate::config::Config;
use crate::error::{AppError, AppResult};

pub fn process_recording(recorder: Recorder, config: &Config) -> AppResult<String> {
    let wav_path = recorder.stop_and_save(&config.temp_dir)?;
    let result = transcribe_and_inject(config, &wav_path);

    // O WAV nao e mais necessario depois da transcricao; remover tambem em
    // caso de erro para o temp_dir nao crescer indefinidamente.
    remove_temp_file(&wav_path);

    result
}

fn transcribe_and_inject(config: &Config, wav_path: &Path) -> AppResult<String> {
    let transcript = transcriber::transcribe(config, wav_path)?;

    if transcript.trim().is_empty() {
        return Err(AppError::CommandFailed(
            "transcriber returned empty text".into(),
        ));
    }

    injector::inject_text(config, &transcript)?;
    Ok(transcript)
}

fn remove_temp_file(path: &Path) {
    if let Err(err) = fs::remove_file(path)
        && err.kind() != std::io::ErrorKind::NotFound
    {
        warn!("failed to remove temp file {}: {err}", path.display());
    }
}
