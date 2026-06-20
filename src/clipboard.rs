use std::ffi::OsStr;
use std::io::Write;
use std::process::{Command, Stdio};

use log::{info, warn};

use crate::error::{AppError, AppResult};

pub fn read_clipboard() -> AppResult<Option<String>> {
    ensure_binary("wl-paste", "sudo apt install wl-clipboard")?;

    let output = Command::new("wl-paste").arg("--no-newline").output()?;

    if !output.status.success() {
        return Ok(None);
    }

    let text = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(Some(text))
}

pub fn write_clipboard(text: &str) -> AppResult<()> {
    ensure_binary("wl-copy", "sudo apt install wl-clipboard")?;

    let mut child = Command::new("wl-copy")
        .arg("--type")
        .arg("text/plain;charset=utf-8")
        .stdin(Stdio::piped())
        .spawn()?;

    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| AppError::CommandFailed("wl-copy: failed to open stdin".into()))?;
        stdin.write_all(text.as_bytes())?;
    }

    let status = child.wait()?;
    if !status.success() {
        return Err(AppError::CommandFailed(format!(
            "wl-copy exited with status {status}"
        )));
    }

    info!("transcript copied to Wayland clipboard");
    Ok(())
}

pub fn verify_clipboard(expected: &str) -> bool {
    let output = match Command::new("wl-paste").arg("--no-newline").output() {
        Ok(output) => output,
        Err(err) => {
            warn!("clipboard verification failed: wl-paste error: {err}");
            return false;
        }
    };

    if !output.status.success() {
        warn!("clipboard verification failed: wl-paste exited with error");
        return false;
    }

    let clipboard_text = String::from_utf8_lossy(&output.stdout).to_string();

    if clipboard_text.trim() == expected.trim() {
        info!("clipboard verification: OK");
        true
    } else {
        warn!(
            "clipboard verification failed: expected {:?}, got {:?}",
            expected.trim(),
            clipboard_text.trim()
        );
        false
    }
}

pub fn write_and_verify_clipboard(text: &str) -> AppResult<bool> {
    write_clipboard(text)?;
    Ok(verify_clipboard(text))
}

pub fn restore_clipboard(previous: Option<String>) -> AppResult<()> {
    if let Some(text) = previous {
        write_clipboard(&text)?;
    }
    Ok(())
}

fn ensure_binary(bin: &str, install_hint: &str) -> AppResult<()> {
    let status = Command::new("sh")
        .arg("-lc")
        .arg(format!("command -v {}", shell_escape(bin)))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(AppError::MissingDependency(format!(
            "{bin} not found. Install it with: {install_hint}"
        )))
    }
}

fn shell_escape(value: &str) -> String {
    let mut escaped = String::from("'");
    for ch in value.chars() {
        if ch == '\'' {
            escaped.push_str("'\"'\"'");
        } else {
            escaped.push(ch);
        }
    }
    escaped.push('\'');
    escaped
}

#[allow(dead_code)]
fn _as_os_str(value: &str) -> &OsStr {
    OsStr::new(value)
}
