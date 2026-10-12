use std::io::Write;
use std::process::{Command, Stdio};

use log::{info, warn};

use crate::error::{AppError, AppResult};
use crate::platform::find_command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClipboardTool {
    WlClipboard,
    Xclip,
    Xsel,
}

impl ClipboardTool {
    fn name(self) -> &'static str {
        match self {
            Self::WlClipboard => "wl-clipboard",
            Self::Xclip => "xclip",
            Self::Xsel => "xsel",
        }
    }
}

/// No Wayland usa wl-clipboard; no X11 cai para xclip/xsel automaticamente.
fn detect_tool() -> Option<ClipboardTool> {
    if find_command("wl-copy").is_some() && find_command("wl-paste").is_some() {
        Some(ClipboardTool::WlClipboard)
    } else if find_command("xclip").is_some() {
        Some(ClipboardTool::Xclip)
    } else if find_command("xsel").is_some() {
        Some(ClipboardTool::Xsel)
    } else {
        None
    }
}

fn missing_tool_error() -> AppError {
    AppError::MissingDependency(
        "no clipboard tool found. Install wl-clipboard (Wayland) or xclip/xsel (X11)".into(),
    )
}

pub fn read_clipboard() -> AppResult<Option<String>> {
    let tool = detect_tool().ok_or_else(missing_tool_error)?;

    let output = match tool {
        ClipboardTool::WlClipboard => Command::new("wl-paste").arg("--no-newline").output()?,
        ClipboardTool::Xclip => Command::new("xclip")
            .args(["-selection", "clipboard", "-o"])
            .output()?,
        ClipboardTool::Xsel => Command::new("xsel")
            .args(["--clipboard", "--output"])
            .output()?,
    };

    if !output.status.success() {
        return Ok(None);
    }

    Ok(Some(String::from_utf8_lossy(&output.stdout).to_string()))
}

pub fn write_clipboard(text: &str) -> AppResult<()> {
    let tool = detect_tool().ok_or_else(missing_tool_error)?;

    let mut command = match tool {
        ClipboardTool::WlClipboard => {
            let mut command = Command::new("wl-copy");
            command.arg("--type").arg("text/plain;charset=utf-8");
            command
        }
        ClipboardTool::Xclip => {
            let mut command = Command::new("xclip");
            command.args(["-selection", "clipboard"]);
            command
        }
        ClipboardTool::Xsel => {
            let mut command = Command::new("xsel");
            command.args(["--clipboard", "--input"]);
            command
        }
    };

    let mut child = command.stdin(Stdio::piped()).spawn()?;

    {
        let stdin = child.stdin.as_mut().ok_or_else(|| {
            AppError::CommandFailed("clipboard tool: failed to open stdin".into())
        })?;
        stdin.write_all(text.as_bytes())?;
    }

    let status = child.wait()?;
    if !status.success() {
        return Err(AppError::CommandFailed(format!(
            "{} exited with status {status}",
            tool.name()
        )));
    }

    info!("transcript copied to clipboard ({})", tool.name());
    Ok(())
}

pub fn verify_clipboard(expected: &str) -> bool {
    let clipboard_text = match read_clipboard() {
        Ok(Some(text)) => text,
        Ok(None) => {
            warn!("clipboard verification failed: clipboard is empty or unreadable");
            return false;
        }
        Err(err) => {
            warn!("clipboard verification failed: {err}");
            return false;
        }
    };

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
