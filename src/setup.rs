use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use dialoguer::Confirm;

use crate::cli::{SetupArgs, WhisperModel};
use crate::config::Config;
use crate::downloader::{run_command, run_shell};
use crate::error::{AppError, AppResult};
use crate::fs_paths::AppPaths;
use crate::platform::{PackageManager, PlatformInfo, apt_package_installed, find_command};

pub fn run_setup(paths: &AppPaths, args: &SetupArgs) -> AppResult<()> {
    let platform = PlatformInfo::detect();
    paths.ensure_base_dirs()?;

    println!("linux-voice-typer setup\n");
    println!("Sistema operacional: {}", platform.os);
    println!(
        "Sessao Wayland: {}",
        if platform.is_wayland_session() {
            "detectada"
        } else {
            "nao detectada"
        }
    );

    if !platform.is_linux {
        return Err(AppError::Unsupported(
            "this MVP currently supports Linux only".into(),
        ));
    }

    if !args.skip_system_deps {
        let missing = detect_system_dependencies(&platform);
        if !missing.packages.is_empty() || !missing.notes.is_empty() {
            print_missing_dependencies(&missing);
            maybe_install_system_dependencies(&platform, &missing, args)?;
        }
    }

    if !ensure_whisper_repo(paths, args)? {
        println!("\nSetup interrompido antes do clone do whisper.cpp.");
        println!("Depois rode:");
        println!("cargo run -- setup");
        return Ok(());
    }

    if !args.skip_whisper_build {
        ensure_whisper_build(paths, args)?;
    }

    if !args.skip_model_download {
        ensure_model(paths, args.model, args)?;
    }

    update_config(paths, args.model)?;

    println!("\nSetup concluido.");
    print_ydotool_guidance();
    println!("Proximos comandos:");
    println!("cargo run -- doctor");
    println!("cargo run -- paste-test");
    println!("cargo run -- run --terminal-hotkey");

    Ok(())
}

#[derive(Debug, Default)]
struct MissingDependencies {
    packages: Vec<&'static str>,
    notes: Vec<String>,
}

fn detect_system_dependencies(platform: &PlatformInfo) -> MissingDependencies {
    let mut missing = MissingDependencies::default();

    let mut require = |command: &str, package: &'static str| {
        if find_command(command).is_none() && !missing.packages.contains(&package) {
            missing.packages.push(package);
        }
    };

    require("git", "git");
    require("cmake", "cmake");
    require("make", "build-essential");
    require("pkg-config", "pkg-config");
    require("wl-copy", "wl-clipboard");
    require("wl-paste", "wl-clipboard");
    require("wtype", "wtype");
    require("ydotool", "ydotool");

    if find_command("cc").is_none()
        && find_command("gcc").is_none()
        && find_command("clang").is_none()
    {
        missing.packages.push("build-essential");
    }

    if platform.package_manager == Some(PackageManager::Apt)
        && !apt_package_installed("libasound2-dev")
    {
        missing.packages.push("libasound2-dev");
    }

    if platform.package_manager == Some(PackageManager::Apt) && !apt_package_installed("ydotool") {
        missing
            .notes
            .push("Dependencia opcional para auto-paste alternativo: ydotool".into());
    }

    missing.packages.sort_unstable();
    missing.packages.dedup();

    if platform.package_manager.is_none() {
        missing
            .notes
            .push("Nenhum gerenciador de pacotes suportado foi detectado automaticamente.".into());
    }

    missing
}

fn print_missing_dependencies(missing: &MissingDependencies) {
    if !missing.packages.is_empty() {
        println!("Dependencias ausentes:");
        for package in &missing.packages {
            println!("- {package}");
        }
        println!();
    }

    for note in &missing.notes {
        println!("{note}");
    }
}

fn maybe_install_system_dependencies(
    platform: &PlatformInfo,
    missing: &MissingDependencies,
    args: &SetupArgs,
) -> AppResult<()> {
    if missing.packages.is_empty() {
        return Ok(());
    }

    match platform.package_manager {
        Some(PackageManager::Apt) => {
            let command = format!(
                "sudo apt update && sudo apt install -y {}",
                missing.packages.join(" ")
            );

            println!("Posso instalar usando apt?");
            println!("Comando:");
            println!("{command}\n");

            if confirm(args, "Continuar?")? {
                run_shell(&command, None)?;
            } else {
                println!("Instalacao cancelada. Rode manualmente:");
                println!("{command}");
            }
        }
        Some(other) => {
            println!(
                "Instalacao automatica ainda nao implementada para {}.",
                other.as_str()
            );
            println!("Instale manualmente os pacotes acima e rode o setup novamente.");
        }
        None => {
            println!("Instale manualmente as dependencias listadas acima.");
        }
    }

    Ok(())
}

fn print_ydotool_guidance() {
    println!("\nSe o wtype falhar no seu compositor Wayland, use o backend ydotool:");
    println!("sudo apt install -y ydotool");
    println!("sudo modprobe uinput");
    println!("sudo ydotoold");
}

fn ensure_whisper_repo(paths: &AppPaths, args: &SetupArgs) -> AppResult<bool> {
    let whisper_dir = paths.whisper_dir();
    if !whisper_dir.exists() {
        println!("\nwhisper.cpp nao encontrado em {}", whisper_dir.display());
        println!(
            "Vou clonar: git clone https://github.com/ggml-org/whisper.cpp.git {}",
            whisper_dir.display()
        );

        if !confirm(args, "Continuar com o clone?")? {
            println!("Clone cancelado. Rode manualmente:");
            println!(
                "git clone https://github.com/ggml-org/whisper.cpp.git {}",
                whisper_dir.display()
            );
            return Ok(false);
        }

        let parent = whisper_dir
            .parent()
            .ok_or_else(|| AppError::Unsupported("invalid whisper directory".into()))?;
        let clone_dest = whisper_dir.to_string_lossy().to_string();
        run_command(
            "git",
            &[
                "clone",
                "https://github.com/ggml-org/whisper.cpp.git",
                &clone_dest,
            ],
            Some(parent),
        )?;
        return Ok(true);
    }

    if args.force || args.rebuild_whisper {
        return Ok(true);
    }

    if confirm(
        args,
        "whisper.cpp ja existe. Deseja atualizar com git pull?",
    )? {
        run_command("git", &["pull", "--ff-only"], Some(whisper_dir))?;
    }

    Ok(true)
}

fn ensure_whisper_build(paths: &AppPaths, args: &SetupArgs) -> AppResult<()> {
    let whisper_bin = find_whisper_bin(paths);
    if whisper_bin.exists() && !args.rebuild_whisper && !args.force {
        println!("\nwhisper-cli ja encontrado em {}", whisper_bin.display());
        return Ok(());
    }

    println!("\nCompilando whisper.cpp...");
    run_command("cmake", &["-B", "build"], Some(paths.whisper_dir()))?;
    run_command(
        "cmake",
        &["--build", "build", "-j"],
        Some(paths.whisper_dir()),
    )?;

    let built_bin = find_whisper_bin(paths);
    if !built_bin.exists() {
        return Err(AppError::CommandFailed(format!(
            "whisper build completed but binary was not found at {}",
            built_bin.display()
        )));
    }

    Ok(())
}

fn ensure_model(paths: &AppPaths, model: WhisperModel, args: &SetupArgs) -> AppResult<()> {
    let model_path = paths.model_path(model);
    if model_path.exists() && !args.force {
        println!("\nModelo ja encontrado em {}", model_path.display());
        return Ok(());
    }

    let script = paths.whisper_dir().join("models/download-ggml-model.sh");
    if !script.exists() {
        return Err(AppError::CommandFailed(format!(
            "download script not found: {}",
            script.display()
        )));
    }

    println!("\nBaixando modelo whisper: {}", model.as_str());
    let script_path = script.to_string_lossy().to_string();
    run_command(&script_path, &[model.as_str()], Some(paths.whisper_dir()))?;

    if !model_path.exists() {
        return Err(AppError::CommandFailed(format!(
            "model download completed but file was not found at {}",
            model_path.display()
        )));
    }

    Ok(())
}

fn update_config(paths: &AppPaths, model: WhisperModel) -> AppResult<()> {
    let mut desired = Config::default_for_paths(paths, model);
    desired.whisper_bin = find_whisper_bin(paths);

    let existing = match Config::load_optional(paths.config_path()) {
        Ok(config) => config,
        Err(err) => {
            println!("config.toml existente sera refeito porque nao pode ser lido: {err}");
            None
        }
    };

    let (merged, changed) = match existing {
        Some(current) => current.merge_runtime_defaults(&desired),
        None => (desired.clone(), true),
    };

    if !changed {
        println!("\nconfig.toml ja estava consistente.");
        return Ok(());
    }

    if paths.config_path().exists() {
        let backup = paths.project_root().join("config.toml.bak");
        fs::copy(paths.config_path(), &backup)?;
        println!("Backup criado em {}", backup.display());
    }

    merged.save(paths.config_path())?;
    println!(
        "config.toml atualizado em {}",
        paths.config_path().display()
    );
    Ok(())
}

fn find_whisper_bin(paths: &AppPaths) -> PathBuf {
    let primary = paths.whisper_bin_path();
    if primary.exists() {
        primary
    } else {
        paths.whisper_build_dir().join("bin/main")
    }
}

fn confirm(args: &SetupArgs, prompt: &str) -> AppResult<bool> {
    if args.yes {
        return Ok(true);
    }

    if !std::io::stdin().is_terminal() {
        return Ok(false);
    }

    Ok(Confirm::new()
        .with_prompt(prompt)
        .default(false)
        .interact()?)
}

#[allow(dead_code)]
fn _canonical(path: &Path) -> AppResult<PathBuf> {
    Ok(path.canonicalize()?)
}
