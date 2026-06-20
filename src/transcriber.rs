use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use log::info;

use crate::config::Config;
use crate::error::{AppError, AppResult};

pub fn transcribe(config: &Config, wav_path: &Path) -> AppResult<String> {
    if !config.whisper_bin.exists() {
        return Err(AppError::InvalidConfig(format!(
            "whisper binary not found: {}",
            config.whisper_bin.display()
        )));
    }

    if !config.model_path.exists() {
        return Err(AppError::InvalidConfig(format!(
            "whisper model not found: {}",
            config.model_path.display()
        )));
    }

    let output_prefix = temp_output_prefix(&config.temp_dir)?;
    let output = Command::new(&config.whisper_bin)
        .arg("-m")
        .arg(&config.model_path)
        .arg("-f")
        .arg(wav_path)
        .arg("-l")
        .arg(&config.language)
        .arg("-otxt")
        .arg("-of")
        .arg(&output_prefix)
        .arg("-nt")
        .arg("-np")
        .output()?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::CommandFailed(format!(
            "whisper.cpp failed with status {} | stdout: {} | stderr: {}",
            output.status,
            stdout.trim(),
            stderr.trim()
        )));
    }

    let txt_path = output_prefix.with_extension("txt");
    let raw = if txt_path.exists() {
        fs::read_to_string(&txt_path)?
    } else {
        String::from_utf8_lossy(&output.stdout).to_string()
    };

    let cleaned = cleanup_transcript(&raw, config.trim_text);
    if cleaned.is_empty() {
        return Err(AppError::CommandFailed(
            "whisper.cpp returned empty transcription".into(),
        ));
    }

    info!("transcription completed");
    Ok(cleaned)
}

fn cleanup_transcript(raw: &str, trim: bool) -> String {
    let mut text = raw
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty()
                && !trimmed.starts_with("[")
                && !trimmed.starts_with("whisper_")
                && !trimmed.starts_with("main:")
                && !trimmed.starts_with("system_info:")
        })
        .collect::<Vec<_>>()
        .join("\n");

    if trim {
        text = text.trim().to_string();
    }

    text
}

fn temp_output_prefix(temp_dir: &Path) -> AppResult<PathBuf> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| AppError::Unsupported(format!("system clock error: {err}")))?
        .as_millis();
    Ok(temp_dir.join(format!("whisper-output-{millis}")))
}
