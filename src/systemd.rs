use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::cli::{ServiceArgs, ServiceCommand};
use crate::error::{AppError, AppResult};

const UNIT_NAME: &str = "linux-voice-typer.service";

pub fn run_service_command(args: &ServiceArgs) -> AppResult<()> {
    match &args.command {
        ServiceCommand::Install { no_enable } => install(!no_enable),
        ServiceCommand::Uninstall => uninstall(),
        ServiceCommand::Status => status(),
    }
}

pub fn unit_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("systemd").join("user").join(UNIT_NAME))
}

fn install(enable: bool) -> AppResult<()> {
    let path = unit_path()
        .ok_or_else(|| AppError::Unsupported("could not resolve config directory".into()))?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let exe = std::env::current_exe()?;
    let content = format!(
        "[Unit]\n\
         Description=Linux Voice Typer (ditado por voz local com Whisper)\n\
         After=graphical-session.target\n\
         \n\
         [Service]\n\
         Type=simple\n\
         ExecStart=\"{}\" run\n\
         Restart=on-failure\n\
         RestartSec=2\n\
         \n\
         [Install]\n\
         WantedBy=default.target\n",
        exe.display()
    );

    fs::write(&path, content)?;
    println!("Unit instalada em {}", path.display());

    run_systemctl(&["daemon-reload"], true)?;

    if enable {
        run_systemctl(&["enable", "--now", UNIT_NAME], true)?;
        println!("Serviço habilitado e iniciado.");
    } else {
        println!("Para ativar:");
        println!("  systemctl --user enable --now {UNIT_NAME}");
    }

    println!("\nSe o Wayland não for herdado pelo gerenciador de serviços, rode:");
    println!("  systemctl --user import-environment WAYLAND_DISPLAY DISPLAY XDG_SESSION_TYPE");
    println!("  systemctl --user restart {UNIT_NAME}");

    Ok(())
}

fn uninstall() -> AppResult<()> {
    let _ = run_systemctl(&["disable", "--now", UNIT_NAME], false);

    if let Some(path) = unit_path()
        && path.exists()
    {
        fs::remove_file(&path)?;
        println!("Unit removida: {}", path.display());
    } else {
        println!("Nenhuma unit instalada.");
    }

    let _ = run_systemctl(&["daemon-reload"], false);
    Ok(())
}

fn status() -> AppResult<()> {
    run_systemctl(&["status", UNIT_NAME, "--no-pager"], false)?;
    Ok(())
}

fn run_systemctl(args: &[&str], required: bool) -> AppResult<()> {
    let status = Command::new("systemctl")
        .arg("--user")
        .args(args)
        .status()
        .map_err(|err| {
            AppError::MissingDependency(format!("failed to execute systemctl: {err}"))
        })?;

    if required && !status.success() {
        return Err(AppError::CommandFailed(format!(
            "systemctl --user {} exited with status {status}",
            args.join(" ")
        )));
    }

    Ok(())
}
