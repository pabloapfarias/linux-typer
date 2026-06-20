use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

use log::{error, info, warn};

use crate::clipboard;
use crate::config::{Config, PasteBackend, PasteShortcut};
use crate::error::AppResult;
use crate::platform::find_command;
use crate::uinput;

const PASTE_TEST_TEXT: &str = "teste do linux-voice-typer";

#[derive(Debug, Clone)]
pub struct PasteBackendStatus {
    pub wl_copy: Option<String>,
    pub wl_paste: Option<String>,
    pub wtype: Option<String>,
    pub ydotool: Option<String>,
    pub ydotoold: Option<String>,
    pub uinput_exists: bool,
    pub uinput_accessible: bool,
    pub ydotoold_running: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasteOutcome {
    Pasted,
    ClipboardOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PasteAction {
    Normal,
    Test,
}

#[derive(Debug)]
struct BackendFailure {
    message: String,
    hint: Option<String>,
    unsupported_virtual_keyboard: bool,
    daemon_or_permission_issue: bool,
}

pub fn inject_text(config: &Config, text: &str) -> AppResult<()> {
    let _ = write_and_paste(config, text, PasteAction::Normal)?;
    Ok(())
}

pub fn run_paste_test(config: &Config) -> AppResult<()> {
    print!("{}", paste_test_report(config)?);
    Ok(())
}

pub fn paste_test_report(config: &Config) -> AppResult<String> {
    let mut report = String::new();
    report.push_str("Copied test text to clipboard.\n");

    let verified = clipboard::write_and_verify_clipboard(PASTE_TEST_TEXT)?;
    if verified {
        report.push_str("Clipboard verification: OK\n");
    } else {
        error!("Clipboard verification: FAILED");
        report.push_str("Clipboard verification: FAILED — wl-paste did not return expected text\n");
        report.push_str("Check if you are in a Wayland session and wl-clipboard is working:\n");
        report.push_str("  echo \"teste\" | wl-copy && wl-paste\n");
        return Ok(report);
    }

    report.push_str(&format!(
        "Trying paste backend: {}\n",
        config.effective_paste_backend().as_str()
    ));
    let outcome = write_and_paste(config, PASTE_TEST_TEXT, PasteAction::Test)?;
    report.push_str(&format!("Paste outcome: {:?}\n", outcome));
    Ok(report)
}

pub fn detect_paste_backend_status() -> PasteBackendStatus {
    let wl_copy = find_command("wl-copy");
    let wl_paste = find_command("wl-paste");
    let wtype = find_command("wtype");
    let ydotool = find_command("ydotool");
    let ydotoold = find_command("ydotoold");
    let uinput_path = Path::new("/dev/uinput");
    let uinput_exists = uinput_path.exists();
    let uinput_accessible = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(uinput_path)
        .is_ok();
    let ydotoold_running = Command::new("sh")
        .arg("-lc")
        .arg("pgrep -x ydotoold >/dev/null 2>&1")
        .status()
        .map(|status| status.success())
        .unwrap_or(false);

    PasteBackendStatus {
        wl_copy,
        wl_paste,
        wtype,
        ydotool,
        ydotoold,
        uinput_exists,
        uinput_accessible,
        ydotoold_running,
    }
}

fn write_and_paste(config: &Config, text: &str, action: PasteAction) -> AppResult<PasteOutcome> {
    let previous = if config.restore_clipboard && action == PasteAction::Normal {
        clipboard::read_clipboard()?
    } else {
        None
    };

    let verified = clipboard::write_and_verify_clipboard(text)?;
    if !verified {
        if action == PasteAction::Normal {
            error!("clipboard verification failed: wl-paste did not return transcript");
        } else {
            println!("Clipboard verification: FAILED");
        }
        return Ok(PasteOutcome::ClipboardOnly);
    }

    let backend = config.effective_paste_backend();
    let shortcut = config.effective_paste_shortcut();

    let outcome = match backend {
        PasteBackend::None => {
            clipboard_only_disabled_notice(action);
            PasteOutcome::ClipboardOnly
        }
        PasteBackend::Auto => attempt_auto_paste(shortcut, action),
        PasteBackend::Wtype => attempt_wtype(shortcut, action),
        PasteBackend::Ydotool => attempt_ydotool(shortcut, action),
        PasteBackend::Uinput => attempt_uinput(shortcut, action),
    };

    // Só restaurar clipboard anterior se o auto-paste confirmou sucesso.
    // Se falhou, a transcrição deve permanecer disponível para Ctrl+V.
    if outcome == PasteOutcome::Pasted && config.restore_clipboard && action == PasteAction::Normal
    {
        thread::sleep(Duration::from_millis(150));
        clipboard::restore_clipboard(previous)?;
    }

    Ok(outcome)
}

fn attempt_auto_paste(shortcut: PasteShortcut, action: PasteAction) -> PasteOutcome {
    let mut had_wtype_attempt = false;

    if has_binary("wtype") {
        had_wtype_attempt = true;
        match try_wtype(shortcut) {
            Ok(()) => return pasted(action, "wtype"),
            Err(failure) => {
                if failure.unsupported_virtual_keyboard {
                    if action == PasteAction::Normal {
                        warn!("wtype: compositor does not support virtual-keyboard protocol");
                    } else {
                        println!("wtype failed: compositor does not support virtual-keyboard");
                    }
                } else {
                    emit_failure("wtype", &failure, action);
                }
            }
        }
    }

    if had_wtype_attempt {
        if action == PasteAction::Normal {
            warn!("wtype failed; trying ydotool as fallback");
        } else {
            println!("trying ydotool...");
        }
    }

    if has_binary("ydotool") {
        match try_ydotool(shortcut) {
            Ok(()) => return pasted(action, "ydotool"),
            Err(failure) => {
                emit_failure("ydotool", &failure, action);
            }
        }
    } else if action == PasteAction::Normal {
        warn!("ydotool not found; trying native uinput fallback");
    } else {
        println!("ydotool not found; trying uinput...");
    }

    match try_uinput(shortcut) {
        Ok(()) => return pasted(action, "uinput"),
        Err(failure) => emit_failure("uinput", &failure, action),
    }

    clipboard_only_notice(action);
    PasteOutcome::ClipboardOnly
}

fn attempt_wtype(shortcut: PasteShortcut, action: PasteAction) -> PasteOutcome {
    if !has_binary("wtype") {
        if action == PasteAction::Normal {
            warn!("wtype not found; transcript copied to clipboard only");
            info!("install wtype or change paste_backend in config.toml");
        } else {
            println!("wtype failed: command not found");
        }
        clipboard_only_notice(action);
        return PasteOutcome::ClipboardOnly;
    }

    match try_wtype(shortcut) {
        Ok(()) => pasted(action, "wtype"),
        Err(failure) => {
            if failure.unsupported_virtual_keyboard {
                if action == PasteAction::Normal {
                    warn!("wtype: compositor does not support virtual-keyboard protocol");
                } else {
                    println!("wtype failed: compositor does not support virtual-keyboard");
                }
            } else {
                emit_failure("wtype", &failure, action);
            }
            clipboard_only_notice(action);
            PasteOutcome::ClipboardOnly
        }
    }
}

fn attempt_ydotool(shortcut: PasteShortcut, action: PasteAction) -> PasteOutcome {
    if !has_binary("ydotool") {
        if action == PasteAction::Normal {
            warn!("ydotool not found; transcript copied to clipboard only");
            info!("install with: sudo apt install ydotool");
        } else {
            println!("ydotool failed: command not found");
        }
        clipboard_only_notice(action);
        return PasteOutcome::ClipboardOnly;
    }

    match try_ydotool(shortcut) {
        Ok(()) => pasted(action, "ydotool"),
        Err(failure) => {
            emit_failure("ydotool", &failure, action);
            clipboard_only_notice(action);
            PasteOutcome::ClipboardOnly
        }
    }
}

fn attempt_uinput(shortcut: PasteShortcut, action: PasteAction) -> PasteOutcome {
    match try_uinput(shortcut) {
        Ok(()) => pasted(action, "uinput"),
        Err(failure) => {
            emit_failure("uinput", &failure, action);
            clipboard_only_notice(action);
            PasteOutcome::ClipboardOnly
        }
    }
}

fn try_wtype(shortcut: PasteShortcut) -> Result<(), BackendFailure> {
    let args = match shortcut {
        PasteShortcut::CtrlV => vec!["-M", "ctrl", "v", "-m", "ctrl"],
        PasteShortcut::CtrlShiftV => {
            vec![
                "-M", "ctrl", "-M", "shift", "v", "-m", "shift", "-m", "ctrl",
            ]
        }
    };

    let output = Command::new("wtype")
        .args(&args)
        .output()
        .map_err(|err| BackendFailure {
            message: format!("failed to execute wtype: {err}"),
            hint: Some("install wtype or switch to paste_backend = \"ydotool\"".into()),
            unsupported_virtual_keyboard: false,
            daemon_or_permission_issue: false,
        })?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{} {}", stdout.trim(), stderr.trim());
    let unsupported = combined.contains("virtual keyboard")
        || combined.contains("virtual-keyboard")
        || combined.contains("zwp_virtual_keyboard_manager_v1");

    Err(BackendFailure {
        message: if combined.trim().is_empty() {
            format!("wtype exited with status {}", output.status)
        } else {
            combined.trim().to_string()
        },
        hint: Some("use paste_backend = \"uinput\" or paste_backend = \"ydotool\"".into()),
        unsupported_virtual_keyboard: unsupported,
        daemon_or_permission_issue: false,
    })
}

fn try_ydotool(shortcut: PasteShortcut) -> Result<(), BackendFailure> {
    let args = match shortcut {
        PasteShortcut::CtrlV => vec!["key", "29:1", "47:1", "47:0", "29:0"],
        PasteShortcut::CtrlShiftV => {
            vec!["key", "29:1", "42:1", "47:1", "47:0", "42:0", "29:0"]
        }
    };

    let output = Command::new("ydotool")
        .args(&args)
        .output()
        .map_err(|err| BackendFailure {
            message: format!("failed to execute ydotool: {err}"),
            hint: Some("install ydotool and start ydotoold".into()),
            unsupported_virtual_keyboard: false,
            daemon_or_permission_issue: true,
        })?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{} {}", stdout.trim(), stderr.trim());
    let daemon_or_permission_issue = combined.contains("uinput")
        || combined.contains("ydotoold")
        || combined.contains("socket")
        || combined.contains("permission")
        || combined.contains("No such file")
        || combined.contains("Connection refused")
        || combined.contains("failed to connect");

    Err(BackendFailure {
        message: if combined.trim().is_empty() {
            format!("ydotool exited with status {}", output.status)
        } else {
            combined.trim().to_string()
        },
        hint: Some("try: sudo modprobe uinput && sudo ydotoold".into()),
        unsupported_virtual_keyboard: false,
        daemon_or_permission_issue,
    })
}

fn try_uinput(shortcut: PasteShortcut) -> Result<(), BackendFailure> {
    uinput::send_paste_shortcut(shortcut).map_err(|err| BackendFailure {
        message: err.to_string(),
        hint: Some("try: sudo modprobe uinput and ensure /dev/uinput is writable".into()),
        unsupported_virtual_keyboard: false,
        daemon_or_permission_issue: true,
    })
}

fn emit_failure(backend: &str, failure: &BackendFailure, action: PasteAction) {
    let message = if failure.daemon_or_permission_issue {
        format!("{} (daemon or permission issue)", failure.message)
    } else {
        failure.message.clone()
    };

    if action == PasteAction::Normal {
        warn!("{backend} failed: {message}");
        if let Some(hint) = &failure.hint {
            warn!("{hint}");
        }
    } else {
        println!("{backend} failed: {message}");
        if let Some(hint) = &failure.hint {
            println!("{hint}");
        }
    }
}

fn clipboard_only_notice(action: PasteAction) {
    if action == PasteAction::Normal {
        warn!("auto-paste failed; keeping transcript in clipboard");
        info!("paste manually with Ctrl+V or Ctrl+Shift+V");
    } else {
        println!("Keeping text in clipboard.");
    }
}

fn clipboard_only_disabled_notice(action: PasteAction) {
    if action == PasteAction::Normal {
        info!("transcript copied to clipboard; auto-paste disabled");
        info!("paste manually with Ctrl+V or Ctrl+Shift+V");
    } else {
        println!("Configured paste backend is none. Clipboard-only mode.");
        println!("Keeping text in clipboard.");
    }
}

fn pasted(action: PasteAction, backend: &str) -> PasteOutcome {
    if action == PasteAction::Normal {
        info!("paste sent using {backend}");
    } else {
        println!("paste succeeded using {backend}");
    }
    PasteOutcome::Pasted
}

fn has_binary(name: &str) -> bool {
    find_command(name).is_some()
}
