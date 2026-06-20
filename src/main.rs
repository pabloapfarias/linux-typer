mod audio;
mod cli;
mod clipboard;
mod config;
mod doctor;
mod downloader;
mod error;
mod fs_paths;
mod hotkey;
mod injector;
mod platform;
mod setup;
mod transcriber;
mod uinput;

use std::io::IsTerminal;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use clap::Parser;
use dialoguer::Confirm;
use log::{error, info, warn};

use crate::audio::Recorder;
use crate::cli::{Cli, Commands};
use crate::config::{Config, PasteShortcut};
use crate::doctor::RuntimeStatus;
use crate::error::{AppError, AppResult};
use crate::fs_paths::AppPaths;
use crate::hotkey::{HotkeyListener, TriggerEvent};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    if let Err(err) = run_main() {
        error!("{err}");
        std::process::exit(1);
    }
}

fn run_main() -> AppResult<()> {
    let cli = Cli::parse();
    let paths = AppPaths::discover()?;
    let default_run = cli.run_args();

    match cli.command.unwrap_or(Commands::Run(default_run)) {
        Commands::Setup(args) => setup::run_setup(&paths, &args),
        Commands::Doctor => doctor::run_doctor(&paths),
        Commands::PasteTest => run_paste_test(&paths),
        Commands::Run(args) => run_command(&paths, args),
    }
}

fn run_command(paths: &AppPaths, args: cli::RunArgs) -> AppResult<()> {
    let status = doctor::runtime_status(paths)?;
    if !status.ready {
        print_runtime_issues(&status);

        if should_run_setup()? {
            setup::run_setup(paths, &cli::SetupArgs::default())?;
        } else {
            return Ok(());
        }
    }

    let mut config = Config::load(paths.config_path())?;
    apply_run_overrides(&mut config, &args);
    run_voice_loop(&config, args.terminal_hotkey)
}

fn apply_run_overrides(config: &mut Config, args: &cli::RunArgs) {
    if args.terminal_paste {
        config.paste_shortcut = PasteShortcut::CtrlShiftV;
    } else if args.editor_paste {
        config.paste_shortcut = PasteShortcut::CtrlV;
    }
}

fn should_run_setup() -> AppResult<bool> {
    if !std::io::stdin().is_terminal() {
        return Ok(false);
    }

    Ok(Confirm::new()
        .with_prompt("Deseja executar o setup agora?")
        .default(false)
        .interact()?)
}

fn print_runtime_issues(status: &RuntimeStatus) {
    println!("Ambiente incompleto.\n");
    println!("Faltando:");
    for item in &status.issues {
        println!("- {item}");
    }
    println!("\nRode:");
    println!("linux-voice-typer setup");
    println!("ou durante desenvolvimento:");
    println!("cargo run -- setup");
}

fn run_voice_loop(config: &Config, force_terminal_hotkey: bool) -> AppResult<()> {
    info!("linux-voice-typer starting");
    info!("language={}", config.language);
    info!("insert_mode={}", config.insert_mode);
    info!(
        "paste_backend={} paste_shortcut={}",
        config.effective_paste_backend().as_str(),
        config.effective_paste_shortcut().as_str()
    );

    let shutdown = Arc::new(AtomicBool::new(false));
    ctrlc::set_handler({
        let shutdown = shutdown.clone();
        move || {
            shutdown.store(true, Ordering::Relaxed);
        }
    })
    .map_err(|err| AppError::Unsupported(format!("failed to install Ctrl+C handler: {err}")))?;

    let listener = HotkeyListener::start(&config.hotkey, force_terminal_hotkey, shutdown.clone())?;
    info!("hotkey mode: {:?}", listener.mode);

    let mut recorder: Option<Recorder> = None;

    while !shutdown.load(Ordering::Relaxed) {
        let Some(event) = listener.recv_timeout(Duration::from_millis(100)) else {
            continue;
        };

        match event {
            TriggerEvent::Start => {
                if recorder.is_some() {
                    warn!("recording already in progress; ignoring duplicate start");
                    continue;
                }

                info!("start recording");
                match Recorder::start() {
                    Ok(active) => {
                        recorder = Some(active);
                    }
                    Err(err) => {
                        error!("failed to start recording: {err}");
                    }
                }
            }
            TriggerEvent::Stop => {
                let Some(active) = recorder.take() else {
                    warn!("stop received without active recording");
                    continue;
                };

                info!("stop recording");
                match process_recording(active, config) {
                    Ok(()) => info!("ready for next capture"),
                    Err(err) => error!("recording pipeline failed: {err}"),
                }
            }
        }
    }

    info!("shutdown requested");
    Ok(())
}

fn run_paste_test(paths: &AppPaths) -> AppResult<()> {
    let config = Config::load_unvalidated(paths.config_path())?;
    injector::run_paste_test(&config)
}

fn process_recording(recorder: Recorder, config: &Config) -> AppResult<()> {
    let wav_path = recorder.stop_and_save(&config.temp_dir)?;
    let transcript = transcriber::transcribe(config, &wav_path)?;

    if transcript.trim().is_empty() {
        return Err(AppError::CommandFailed(
            "transcriber returned empty text".into(),
        ));
    }

    info!("transcript: {}", transcript);
    injector::inject_text(config, &transcript)?;
    Ok(())
}
