pub mod audio;
pub mod cli;
pub mod clipboard;
pub mod config;
pub mod doctor;
pub mod downloader;
pub mod error;
pub mod fs_paths;
pub mod hotkey;
pub mod injector;
pub mod platform;
pub mod service;
pub mod setup;
pub mod transcriber;
pub mod uinput;

use crate::audio::Recorder;
use crate::config::Config;
use crate::error::{AppError, AppResult};

pub fn process_recording(recorder: Recorder, config: &Config) -> AppResult<String> {
    let wav_path = recorder.stop_and_save(&config.temp_dir)?;
    let transcript = transcriber::transcribe(config, &wav_path)?;

    if transcript.trim().is_empty() {
        return Err(AppError::CommandFailed(
            "transcriber returned empty text".into(),
        ));
    }

    injector::inject_text(config, &transcript)?;
    Ok(transcript)
}
