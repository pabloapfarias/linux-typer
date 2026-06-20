use std::path::Path;
use std::process::{Command, Stdio};

use crate::error::{AppError, AppResult};

pub fn run_command(program: &str, args: &[&str], cwd: Option<&Path>) -> AppResult<()> {
    let mut command = Command::new(program);
    command.args(args);
    if let Some(path) = cwd {
        command.current_dir(path);
    }
    command.stdin(Stdio::inherit());
    command.stdout(Stdio::inherit());
    command.stderr(Stdio::inherit());

    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(AppError::CommandFailed(format!(
            "{} {} exited with status {}",
            program,
            args.join(" "),
            status
        )))
    }
}

pub fn run_shell(command: &str, cwd: Option<&Path>) -> AppResult<()> {
    let mut shell = Command::new("sh");
    shell.arg("-lc").arg(command);
    if let Some(path) = cwd {
        shell.current_dir(path);
    }
    shell.stdin(Stdio::inherit());
    shell.stdout(Stdio::inherit());
    shell.stderr(Stdio::inherit());

    let status = shell.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(AppError::CommandFailed(format!(
            "shell command failed with status {}: {}",
            status, command
        )))
    }
}
