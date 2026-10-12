use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppResult;

const MAX_ENTRIES: usize = 500;
const TRIM_AFTER_BYTES: u64 = 256 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub timestamp: String,
    pub text: String,
}

pub fn history_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|dir| dir.join("linux-voice-typer").join("history.jsonl"))
}

/// Adiciona uma transcricao ao historico local (JSON Lines).
pub fn append(text: &str) -> AppResult<()> {
    let Some(path) = history_path() else {
        return Ok(());
    };
    append_to(&path, text)
}

/// Retorna as ultimas `limit` transcricoes, da mais recente para a mais antiga.
pub fn load_recent(limit: usize) -> AppResult<Vec<HistoryEntry>> {
    let Some(path) = history_path() else {
        return Ok(Vec::new());
    };
    load_from(&path, limit)
}

pub fn clear() -> AppResult<()> {
    if let Some(path) = history_path()
        && path.exists()
    {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn append_to(path: &Path, text: &str) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    trim_if_needed(path);

    let entry = HistoryEntry {
        timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        text: text.to_string(),
    };
    let line = serde_json::to_string(&entry)?;

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{line}")?;
    Ok(())
}

pub fn load_from(path: &Path, limit: usize) -> AppResult<Vec<HistoryEntry>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let raw = fs::read_to_string(path)?;
    let mut entries: Vec<HistoryEntry> = raw
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();

    if entries.len() > limit {
        entries = entries.split_off(entries.len() - limit);
    }
    entries.reverse();
    Ok(entries)
}

/// Mantem o arquivo limitado: quando cresce demais, preserva as ultimas
/// `MAX_ENTRIES` linhas.
fn trim_if_needed(path: &Path) {
    let Ok(metadata) = fs::metadata(path) else {
        return;
    };
    if metadata.len() < TRIM_AFTER_BYTES {
        return;
    }

    let Ok(raw) = fs::read_to_string(path) else {
        return;
    };
    let lines: Vec<&str> = raw.lines().filter(|line| !line.trim().is_empty()).collect();
    if lines.len() <= MAX_ENTRIES {
        return;
    }

    let kept = lines[lines.len() - MAX_ENTRIES..].join("\n");
    let _ = fs::write(path, format!("{kept}\n"));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("lvt-history-test-{name}.jsonl"))
    }

    #[test]
    fn test_append_and_load_recent() {
        let path = temp_path("append");
        let _ = fs::remove_file(&path);

        append_to(&path, "primeira").unwrap();
        append_to(&path, "segunda").unwrap();

        let entries = load_from(&path, 10).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "segunda");
        assert_eq!(entries[1].text, "primeira");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_load_recent_respects_limit() {
        let path = temp_path("limit");
        let _ = fs::remove_file(&path);

        for index in 0..5 {
            append_to(&path, &format!("item {index}")).unwrap();
        }

        let entries = load_from(&path, 2).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "item 4");
        assert_eq!(entries[1].text, "item 3");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_load_missing_file_is_empty() {
        let path = temp_path("missing");
        let _ = fs::remove_file(&path);
        assert!(load_from(&path, 10).unwrap().is_empty());
    }
}
