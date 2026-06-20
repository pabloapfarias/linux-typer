use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;

use linux_voice_typer::cli::WhisperModel;
use linux_voice_typer::config::{Config, PasteBackend, PasteShortcut};
use linux_voice_typer::doctor;
use linux_voice_typer::error::{AppError, AppResult};
use linux_voice_typer::fs_paths::AppPaths;
use linux_voice_typer::injector;
use linux_voice_typer::service::{VoiceTyperService, VoiceTyperStatus};
use serde::{Deserialize, Serialize};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, State, WindowEvent};

struct DesktopState {
    paths: AppPaths,
    service: VoiceTyperService,
    last_doctor: Mutex<Option<String>>,
    last_paste_test: Mutex<Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
struct DesktopConfig {
    hotkey: String,
    language: String,
    model_path: String,
    whisper_bin: String,
    insert_mode: String,
    restore_clipboard: bool,
    auto_paste: bool,
    paste_backend: PasteBackend,
    paste_shortcut: PasteShortcut,
    trim_text: bool,
    temp_dir: String,
    start_minimized: bool,
    start_with_system: bool,
}

#[derive(Debug, Clone, Serialize)]
struct CommandOutput {
    ok: bool,
    message: String,
}

#[tauri::command]
fn get_status(state: State<'_, DesktopState>) -> VoiceTyperStatus {
    state.service.status()
}

#[tauri::command]
fn start_service(state: State<'_, DesktopState>) -> Result<VoiceTyperStatus, String> {
    let config = load_validated_config(&state.paths).map_err(to_message)?;
    state.service.reload_config(config).map_err(to_message)?;
    state.service.start().map_err(to_message)?;
    Ok(state.service.status())
}

#[tauri::command]
fn stop_service(state: State<'_, DesktopState>) -> Result<VoiceTyperStatus, String> {
    state.service.stop().map_err(to_message)?;
    Ok(state.service.status())
}

#[tauri::command]
fn restart_service(state: State<'_, DesktopState>) -> Result<VoiceTyperStatus, String> {
    let config = load_validated_config(&state.paths).map_err(to_message)?;
    state.service.reload_config(config).map_err(to_message)?;
    state.service.restart().map_err(to_message)?;
    Ok(state.service.status())
}

#[tauri::command]
fn get_config(state: State<'_, DesktopState>) -> Result<DesktopConfig, String> {
    let config = load_config_or_default(&state.paths).map_err(to_message)?;
    Ok(config_to_desktop(config))
}

#[tauri::command]
fn save_config(
    state: State<'_, DesktopState>,
    config: DesktopConfig,
) -> Result<VoiceTyperStatus, String> {
    let config = desktop_to_config(config);
    config.save(state.paths.config_path()).map_err(to_message)?;
    state.service.reload_config(config).map_err(to_message)?;
    Ok(state.service.status())
}

#[tauri::command]
fn run_doctor(state: State<'_, DesktopState>) -> Result<CommandOutput, String> {
    let output = doctor::doctor_report_text(&state.paths).map_err(to_message)?;
    if let Ok(mut last) = state.last_doctor.lock() {
        *last = Some(output.clone());
    }
    Ok(CommandOutput {
        ok: !output.contains("[ERROR]"),
        message: output,
    })
}

#[tauri::command]
fn run_paste_test(state: State<'_, DesktopState>) -> Result<CommandOutput, String> {
    let config = load_config_or_default(&state.paths).map_err(to_message)?;
    let output = injector::paste_test_report(&config).map_err(to_message)?;
    if let Ok(mut last) = state.last_paste_test.lock() {
        *last = Some(output.clone());
    }
    Ok(CommandOutput {
        ok: !output.contains("FAILED"),
        message: output,
    })
}

#[tauri::command]
fn open_config_file(state: State<'_, DesktopState>) -> Result<(), String> {
    Command::new("xdg-open")
        .arg(state.paths.config_path())
        .spawn()
        .map_err(|err| format!("failed to open config file: {err}"))?;
    Ok(())
}

#[tauri::command]
fn select_model_file() -> Option<String> {
    None
}

#[tauri::command]
fn select_whisper_bin() -> Option<String> {
    None
}

#[tauri::command]
fn get_recent_logs(state: State<'_, DesktopState>) -> Vec<String> {
    state.service.recent_events()
}

#[tauri::command]
fn set_mode_editor(state: State<'_, DesktopState>) -> Result<VoiceTyperStatus, String> {
    let mut config = load_config_or_default(&state.paths).map_err(to_message)?;
    config.paste_shortcut = PasteShortcut::CtrlV;
    config.save(state.paths.config_path()).map_err(to_message)?;
    state
        .service
        .set_paste_shortcut(PasteShortcut::CtrlV)
        .map_err(to_message)?;
    state.service.reload_config(config).map_err(to_message)?;
    Ok(state.service.status())
}

#[tauri::command]
fn set_mode_terminal(state: State<'_, DesktopState>) -> Result<VoiceTyperStatus, String> {
    let mut config = load_config_or_default(&state.paths).map_err(to_message)?;
    config.paste_shortcut = PasteShortcut::CtrlShiftV;
    config.save(state.paths.config_path()).map_err(to_message)?;
    state
        .service
        .set_paste_shortcut(PasteShortcut::CtrlShiftV)
        .map_err(to_message)?;
    state.service.reload_config(config).map_err(to_message)?;
    Ok(state.service.status())
}

pub fn run() {
    let paths = AppPaths::discover().expect("failed to discover app paths");
    let config = load_config_or_default(&paths).expect("failed to load config");
    let state = DesktopState {
        paths,
        service: VoiceTyperService::new(config),
        last_doctor: Mutex::new(None),
        last_paste_test: Mutex::new(None),
    };

    tauri::Builder::default()
        .manage(state)
        .setup(|app| {
            install_tray(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            start_service,
            stop_service,
            restart_service,
            get_config,
            save_config,
            run_doctor,
            run_paste_test,
            open_config_file,
            select_model_file,
            select_whisper_bin,
            get_recent_logs,
            set_mode_editor,
            set_mode_terminal
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn install_tray(app: &AppHandle) -> tauri::Result<()> {
    let title = MenuItem::with_id(app, "title", "Linux Voice Typer", false, None::<&str>)?;
    let start = MenuItem::with_id(app, "start", "Iniciar", true, None::<&str>)?;
    let stop = MenuItem::with_id(app, "stop", "Pausar", true, None::<&str>)?;
    let editor = MenuItem::with_id(app, "editor", "Modo Editor", true, None::<&str>)?;
    let terminal = MenuItem::with_id(app, "terminal", "Modo Terminal", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Abrir Configurações", true, None::<&str>)?;
    let doctor = MenuItem::with_id(app, "doctor", "Rodar Doctor", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &title, &start, &stop, &editor, &terminal, &settings, &doctor, &quit,
        ],
    )?;
    let icon = Image::from_bytes(include_bytes!("../icons/icon.png"))?;

    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "start" => {
                let state = app.state::<DesktopState>();
                if let Ok(config) = load_validated_config(&state.paths) {
                    let _ = state.service.reload_config(config);
                    let _ = state.service.start();
                }
            }
            "stop" => {
                let state = app.state::<DesktopState>();
                let _ = state.service.stop();
            }
            "editor" => {
                let state = app.state::<DesktopState>();
                let _ = set_shortcut_from_tray(&state, PasteShortcut::CtrlV);
            }
            "terminal" => {
                let state = app.state::<DesktopState>();
                let _ = set_shortcut_from_tray(&state, PasteShortcut::CtrlShiftV);
            }
            "settings" => show_main_window(app),
            "doctor" => {
                let state = app.state::<DesktopState>();
                if let Ok(output) = doctor::doctor_report_text(&state.paths) {
                    if let Ok(mut last) = state.last_doctor.lock() {
                        *last = Some(output);
                    }
                }
                show_main_window(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn set_shortcut_from_tray(
    state: &State<'_, DesktopState>,
    shortcut: PasteShortcut,
) -> AppResult<()> {
    let mut config = load_config_or_default(&state.paths)?;
    config.paste_shortcut = shortcut;
    config.save(state.paths.config_path())?;
    state.service.set_paste_shortcut(shortcut)?;
    state.service.reload_config(config)
}

fn load_validated_config(paths: &AppPaths) -> AppResult<Config> {
    Config::load(paths.config_path())
}

fn load_config_or_default(paths: &AppPaths) -> AppResult<Config> {
    if paths.config_path().exists() {
        Config::load_unvalidated(paths.config_path())
    } else {
        Ok(Config::default_for_paths(paths, WhisperModel::Small))
    }
}

fn config_to_desktop(config: Config) -> DesktopConfig {
    DesktopConfig {
        hotkey: config.hotkey,
        language: config.language,
        model_path: config.model_path.display().to_string(),
        whisper_bin: config.whisper_bin.display().to_string(),
        insert_mode: config.insert_mode,
        restore_clipboard: config.restore_clipboard,
        auto_paste: config.auto_paste,
        paste_backend: config.paste_backend,
        paste_shortcut: config.paste_shortcut,
        trim_text: config.trim_text,
        temp_dir: config.temp_dir.display().to_string(),
        start_minimized: false,
        start_with_system: false,
    }
}

fn desktop_to_config(config: DesktopConfig) -> Config {
    Config {
        hotkey: config.hotkey,
        language: config.language,
        model_path: PathBuf::from(config.model_path),
        whisper_bin: PathBuf::from(config.whisper_bin),
        insert_mode: if config.insert_mode.trim().is_empty() {
            "clipboard".into()
        } else {
            config.insert_mode
        },
        restore_clipboard: config.restore_clipboard,
        auto_paste: config.auto_paste,
        paste_backend: config.paste_backend,
        paste_shortcut: config.paste_shortcut,
        trim_text: config.trim_text,
        temp_dir: PathBuf::from(config.temp_dir),
    }
}

fn to_message(error: AppError) -> String {
    error.to_string()
}
