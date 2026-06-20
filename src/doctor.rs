use cpal::traits::HostTrait;
use std::fs::{self, OpenOptions};
use std::io::Write;

use keytap::Tap;

use crate::clipboard;
use crate::config::Config;
use crate::error::AppResult;
use crate::fs_paths::AppPaths;
use crate::injector::detect_paste_backend_status;
use crate::platform::PlatformInfo;
use crate::uinput;

const CLIPBOARD_TEST_TEXT: &str = "linux-voice-typer-doctor-test";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CheckLevel {
    Ok,
    Warn,
    Error,
    Suggest,
}

#[derive(Debug)]
struct Check {
    level: CheckLevel,
    message: String,
}

#[derive(Debug)]
pub struct RuntimeStatus {
    pub ready: bool,
    pub issues: Vec<String>,
}

pub fn run_doctor(paths: &AppPaths) -> AppResult<()> {
    let report = collect_report(paths)?;

    println!("linux-voice-typer doctor\n");
    println!("Sistema:");
    for check in &report.system_checks {
        println!("[{}] {}", label(check.level), check.message);
    }

    println!("\nPaste backend:");
    for check in &report.paste_checks {
        println!("[{}] {}", label(check.level), check.message);
    }

    println!("\nArquivos e audio:");
    for check in &report.runtime_checks {
        println!("[{}] {}", label(check.level), check.message);
    }

    if report.has_errors() {
        println!("\nPara corrigir, rode:");
        println!("linux-voice-typer setup");
        println!("ou durante desenvolvimento:");
        println!("cargo run -- setup");
    }

    Ok(())
}

pub fn runtime_status(paths: &AppPaths) -> AppResult<RuntimeStatus> {
    let report = collect_report(paths)?;
    Ok(RuntimeStatus {
        ready: report.fatal_issues.is_empty(),
        issues: report.fatal_issues,
    })
}

#[derive(Debug)]
struct DoctorReport {
    system_checks: Vec<Check>,
    paste_checks: Vec<Check>,
    runtime_checks: Vec<Check>,
    fatal_issues: Vec<String>,
}

impl DoctorReport {
    fn has_errors(&self) -> bool {
        self.system_checks
            .iter()
            .chain(self.paste_checks.iter())
            .chain(self.runtime_checks.iter())
            .any(|check| check.level == CheckLevel::Error)
    }
}

fn collect_report(paths: &AppPaths) -> AppResult<DoctorReport> {
    let mut system_checks = Vec::new();
    let mut paste_checks = Vec::new();
    let mut runtime_checks = Vec::new();
    let mut fatal_issues = Vec::new();

    let platform = PlatformInfo::detect();
    let paste_status = detect_paste_backend_status();

    if platform.is_linux {
        system_checks.push(ok("Linux detected"));
    } else {
        let message = format!("Unsupported OS: {}", platform.os);
        system_checks.push(err(message.clone()));
        fatal_issues.push(message);
    }

    if platform.is_wayland_session() {
        system_checks.push(ok("Wayland session detected"));
    } else {
        system_checks.push(warn(format!(
            "Wayland session not clearly detected (XDG_SESSION_TYPE={})",
            platform.xdg_session_type.as_deref().unwrap_or("unset")
        )));
    }

    if let Some(path) = &paste_status.wl_copy {
        paste_checks.push(ok(format!("wl-copy found: {path}")));
    } else {
        let message = "wl-copy not found".to_string();
        paste_checks.push(err(message.clone()));
        paste_checks.push(suggest("sudo apt install wl-clipboard"));
        fatal_issues.push(message);
    }

    if let Some(path) = &paste_status.wl_paste {
        paste_checks.push(ok(format!("wl-paste found: {path}")));
    } else {
        paste_checks.push(warn("wl-paste not found"));
        paste_checks.push(suggest("sudo apt install wl-clipboard"));
    }

    // Teste real de clipboard: escrever e ler de volta
    if paste_status.wl_copy.is_some() && paste_status.wl_paste.is_some() {
        match clipboard::read_clipboard() {
            Ok(previous) => {
                match clipboard::write_and_verify_clipboard(CLIPBOARD_TEST_TEXT) {
                    Ok(true) => {
                        paste_checks.push(ok("clipboard write/read test passed"));
                    }
                    Ok(false) => {
                        paste_checks.push(err(
                            "clipboard write/read test failed: wl-paste did not return expected text",
                        ));
                        paste_checks.push(suggest(
                            "verifique se esta em sessao Wayland e se wl-clipboard funciona: echo \"teste\" | wl-copy && wl-paste",
                        ));
                    }
                    Err(test_err) => {
                        paste_checks.push(err(format!("clipboard test error: {test_err}")));
                    }
                }
                // Restaurar clipboard anterior
                let _ = clipboard::restore_clipboard(previous);
            }
            Err(err) => {
                paste_checks.push(warn(format!(
                    "clipboard test skipped: could not read current clipboard ({err})"
                )));
            }
        }
    }

    if let Some(path) = &paste_status.wtype {
        paste_checks.push(ok(format!("wtype found: {path}")));
        paste_checks.push(warn(
            "wtype installed but compositor may not support virtual-keyboard protocol",
        ));
        paste_checks.push(suggest(
            "if wtype fails, use paste_backend = \"uinput\" or paste_backend = \"ydotool\"",
        ));
    } else {
        paste_checks.push(warn("wtype not found"));
    }

    if let Some(path) = &paste_status.ydotool {
        paste_checks.push(ok(format!("ydotool found: {path}")));
    } else {
        paste_checks.push(warn("ydotool not found"));
        paste_checks.push(suggest("sudo apt install ydotool"));
    }

    if let Some(path) = &paste_status.ydotoold {
        paste_checks.push(ok(format!("ydotoold found: {path}")));
    } else {
        paste_checks.push(warn("ydotoold not found"));
    }

    if paste_status.uinput_exists {
        if paste_status.uinput_accessible {
            paste_checks.push(ok("/dev/uinput exists and is accessible"));
            match uinput::probe_virtual_keyboard() {
                Ok(()) => paste_checks.push(ok("native uinput virtual keyboard probe passed")),
                Err(err) => paste_checks.push(warn(format!(
                    "native uinput virtual keyboard probe failed: {err}"
                ))),
            }
        } else {
            paste_checks.push(warn(
                "/dev/uinput exists, but permission may require daemon/root setup",
            ));
        }
    } else {
        paste_checks.push(warn("/dev/uinput not found"));
        paste_checks.push(suggest("sudo modprobe uinput"));
    }

    if paste_status.ydotool.is_some() && paste_status.ydotoold.is_some() {
        if paste_status.ydotoold_running {
            paste_checks.push(ok("ydotoold appears to be running"));
        } else {
            paste_checks.push(warn(
                "ydotoold found, but daemon is not running; ydotool requires ydotoold",
            ));
            paste_checks.push(suggest("sudo modprobe uinput && sudo ydotoold"));
        }
    }

    paste_checks.push(ok("clipboard fallback available"));

    let config = match Config::load_optional(paths.config_path()) {
        Ok(config) => {
            if paths.config_path().exists() {
                runtime_checks.push(ok(format!(
                    "config.toml found: {}",
                    paths.config_path().display()
                )));
            } else {
                let message = format!("config.toml missing: {}", paths.config_path().display());
                runtime_checks.push(err(message.clone()));
                fatal_issues.push(message);
            }
            config
        }
        Err(load_err) => {
            let message = format!(
                "config.toml invalid: {} ({})",
                paths.config_path().display(),
                load_err
            );
            runtime_checks.push(err(message.clone()));
            fatal_issues.push(message);
            None
        }
    };

    match config {
        Some(config) => {
            runtime_checks.push(ok(format!(
                "Configured paste_backend: {}",
                config.effective_paste_backend().as_str()
            )));
            runtime_checks.push(ok(format!(
                "Configured paste_shortcut: {}",
                config.effective_paste_shortcut().as_str()
            )));

            if config.whisper_bin.exists() {
                runtime_checks.push(ok(format!(
                    "whisper-cli found: {}",
                    config.whisper_bin.display()
                )));
            } else {
                let message = format!("whisper-cli missing: {}", config.whisper_bin.display());
                runtime_checks.push(err(message.clone()));
                fatal_issues.push(message);
            }

            if config.model_path.exists() {
                runtime_checks.push(ok(format!("model found: {}", config.model_path.display())));
            } else {
                let message = format!("model missing: {}", config.model_path.display());
                runtime_checks.push(err(message.clone()));
                fatal_issues.push(message);
            }

            fs::create_dir_all(&config.temp_dir)?;
            match OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(config.temp_dir.join(".doctor-write-test"))
            {
                Ok(mut file) => {
                    let _ = file.write_all(b"ok");
                    let _ = fs::remove_file(config.temp_dir.join(".doctor-write-test"));
                    runtime_checks.push(ok(format!(
                        "temp_dir writable: {}",
                        config.temp_dir.display()
                    )));
                }
                Err(write_err) => {
                    let message = format!(
                        "temp_dir not writable: {} ({})",
                        config.temp_dir.display(),
                        write_err
                    );
                    runtime_checks.push(err(message.clone()));
                    fatal_issues.push(message);
                }
            }
        }
        None => {
            if !paths.config_path().exists() {
                let message = "config.toml could not be loaded".to_string();
                runtime_checks.push(err(message.clone()));
                fatal_issues.push(message);
            }
        }
    }

    if cpal::default_host().default_input_device().is_some() {
        runtime_checks.push(ok("microphone detected via cpal"));
    } else {
        let message = "no default microphone detected via cpal".to_string();
        runtime_checks.push(err(message.clone()));
        fatal_issues.push(message);
    }

    match Tap::new() {
        Ok(tap) => {
            drop(tap);
            runtime_checks.push(ok("keytap initialized"));
        }
        Err(err) => {
            runtime_checks.push(warn(format!(
                "Global hotkey may require input permissions ({})",
                err
            )));
        }
    }

    runtime_checks.push(ok("Fallback terminal hotkey available"));

    Ok(DoctorReport {
        system_checks,
        paste_checks,
        runtime_checks,
        fatal_issues,
    })
}

fn label(level: CheckLevel) -> &'static str {
    match level {
        CheckLevel::Ok => "OK",
        CheckLevel::Warn => "WARN",
        CheckLevel::Error => "ERROR",
        CheckLevel::Suggest => "SUGGEST",
    }
}

fn ok(message: impl Into<String>) -> Check {
    Check {
        level: CheckLevel::Ok,
        message: message.into(),
    }
}

fn warn(message: impl Into<String>) -> Check {
    Check {
        level: CheckLevel::Warn,
        message: message.into(),
    }
}

fn err(message: impl Into<String>) -> Check {
    Check {
        level: CheckLevel::Error,
        message: message.into(),
    }
}

fn suggest(message: impl Into<String>) -> Check {
    Check {
        level: CheckLevel::Suggest,
        message: message.into(),
    }
}
