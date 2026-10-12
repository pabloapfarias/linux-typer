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
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};

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
    auto_start_service: bool,
    start_minimized: bool,
    start_with_system: bool,
    #[serde(default = "default_close_to_tray")]
    close_to_tray: bool,
}

fn default_close_to_tray() -> bool {
    true
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
async fn start_service(state: State<'_, DesktopState>, app: AppHandle) -> Result<VoiceTyperStatus, String> {
    if state.service.is_running() {
        return Ok(state.service.status());
    }
    let config = load_validated_config(&state.paths).map_err(to_message)?;
    state.service.reload_config(config).map_err(to_message)?;
    state.service.start().map_err(to_message)?;
    update_tray_icon(&app, "idle");
    Ok(state.service.status())
}

#[tauri::command]
async fn stop_service(state: State<'_, DesktopState>, app: AppHandle) -> Result<VoiceTyperStatus, String> {
    state.service.stop().map_err(to_message)?;
    update_tray_icon(&app, "stopped");
    Ok(state.service.status())
}

#[tauri::command]
async fn restart_service(state: State<'_, DesktopState>, app: AppHandle) -> Result<VoiceTyperStatus, String> {
    let config = load_validated_config(&state.paths).map_err(to_message)?;
    state.service.reload_config(config).map_err(to_message)?;
    state.service.restart().map_err(to_message)?;
    update_tray_icon(&app, "idle");
    Ok(state.service.status())
}

#[tauri::command]
fn get_config(state: State<'_, DesktopState>) -> Result<DesktopConfig, String> {
    let config = load_config_or_default(&state.paths).map_err(to_message)?;
    Ok(config_to_desktop(config))
}

#[tauri::command]
async fn save_config(
    state: State<'_, DesktopState>,
    app: AppHandle,
    config: DesktopConfig,
) -> Result<VoiceTyperStatus, String> {
    let was_running = state.service.is_running();
    let start_with_system = config.start_with_system;
    let config = desktop_to_config(config);
    config.save(state.paths.config_path()).map_err(to_message)?;
    let _ = linux_voice_typer::autostart::set_autostart_enabled(start_with_system);
    state.service.reload_config(config).map_err(to_message)?;
    if was_running {
        state.service.restart().map_err(to_message)?;
        update_tray_icon(&app, "idle");
    }
    let mut status = state.service.status();
    if was_running {
        status.recent_events.push(
            "Configuração salva e serviço reiniciado com o novo atalho.".into(),
        );
    } else {
        status
            .recent_events
            .push("Configuração salva.".into());
    }
    Ok(status)
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
async fn set_mode_editor(state: State<'_, DesktopState>) -> Result<VoiceTyperStatus, String> {
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
async fn set_mode_terminal(state: State<'_, DesktopState>) -> Result<VoiceTyperStatus, String> {
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
            let app_handle = app.handle().clone();
            let state = app.state::<DesktopState>();
            let app_h = app_handle.clone();
            let _ = state.service.set_state_listener(std::sync::Arc::new(move |feedback| {
                let state_str = match feedback.state {
                    linux_voice_typer::service::ServiceState::Idle => "idle",
                    linux_voice_typer::service::ServiceState::Recording => "recording",
                    linux_voice_typer::service::ServiceState::Transcribing => "transcribing",
                };
                update_tray_icon(&app_h, state_str);
                let _ = app_h.emit(
                    "dictation-state",
                    serde_json::json!({
                        "state": state_str,
                        "transcript": feedback.transcript,
                        "level": feedback.level,
                        "elapsed_secs": feedback.elapsed_secs,
                    }),
                );
            }));
            install_tray(app.handle())?;

            let config = load_config_or_default(&state.paths)
                .unwrap_or_else(|_| Config::default_for_paths(&state.paths, WhisperModel::Small));
            if config.auto_start_service {
                if let Err(err) = state.service.start() {
                    eprintln!("failed to auto-start service: {err}");
                }
                update_tray_icon(app.handle(), "idle");
            } else {
                update_tray_icon(app.handle(), "stopped");
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                let close_to_tray = app
                    .try_state::<DesktopState>()
                    .and_then(|state| load_config_or_default(&state.paths).ok())
                    .map(|config| config.close_to_tray)
                    .unwrap_or(true);

                if close_to_tray && tray_host_available(app) {
                    let _ = window.hide();
                    api.prevent_close();
                }
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

fn update_tray_icon(app: &AppHandle, state: &str) {
    static CURRENT_STATE: Mutex<Option<String>> = Mutex::new(None);
    {
        let mut curr = match CURRENT_STATE.lock() {
            Ok(c) => c,
            Err(_) => return,
        };
        if curr.as_deref() == Some(state) {
            return;
        }
        *curr = Some(state.to_string());
    }

    let app = app.clone();
    let state_string = state.to_string();
    std::thread::spawn(move || {
        if let Some(tray) = app.tray_by_id("main-tray") {
            let bytes: &[u8] = match state_string.as_str() {
                "recording" => include_bytes!("../icons/icon_red.png"),
                "stopped" => include_bytes!("../icons/icon_gray.png"),
                _ => include_bytes!("../icons/icon_green.png"),
            };
            if let Ok(icon) = Image::from_bytes(bytes) {
                let _ = tray.set_icon(Some(icon));
            }
        }
    });
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
    let icon = Image::from_bytes(include_bytes!("../icons/icon_gray.png"))?;

    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "start" => {
                let state = app.state::<DesktopState>();
                let paths = state.paths.clone();
                let service = state.service.clone();
                let app_h = app.clone();
                std::thread::spawn(move || {
                    if let Ok(config) = load_validated_config(&paths) {
                        let _ = service.reload_config(config);
                        let _ = service.start();
                        update_tray_icon(&app_h, "idle");
                    }
                });
            }
            "stop" => {
                let state = app.state::<DesktopState>();
                let service = state.service.clone();
                let app_h = app.clone();
                std::thread::spawn(move || {
                    let _ = service.stop();
                    update_tray_icon(&app_h, "stopped");
                });
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

/// O icone de bandeja so aparece se houver um host StatusNotifier no barramento
/// (KDE, waybar, extensao AppIndicator no GNOME, etc.). Sem host, esconder a
/// janela deixaria o app inacessivel.
fn tray_host_available(app: &AppHandle) -> bool {
    if app.tray_by_id("main-tray").is_none() {
        return false;
    }

    let output = Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            "org.freedesktop.DBus",
            "--object-path",
            "/org/freedesktop/DBus",
            "--method",
            "org.freedesktop.DBus.NameHasOwner",
            "org.kde.StatusNotifierWatcher",
        ])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).contains("true")
        }
        // Sem gdbus ou erro na consulta: mantem o comportamento atual.
        _ => true,
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
        auto_start_service: config.auto_start_service,
        start_minimized: config.start_minimized,
        start_with_system: linux_voice_typer::autostart::is_autostart_enabled(),
        close_to_tray: config.close_to_tray,
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
        auto_start_service: config.auto_start_service,
        start_minimized: config.start_minimized,
        start_with_system: config.start_with_system,
        close_to_tray: config.close_to_tray,
    }
}

fn to_message(error: AppError) -> String {
    match &error {
        AppError::InvalidConfig(msg) => {
            if msg.contains("model_path") {
                "Modelo Whisper não encontrado. Rode `cargo run -- setup` ou verifique o caminho em Configurações.".into()
            } else if msg.contains("whisper_bin") {
                "Whisper CLI não encontrado. Rode `cargo run -- setup` para instalar.".into()
            } else if msg.contains("hotkey") {
                "Hotkey inválida. Verifique o campo Hotkey em Configurações.".into()
            } else {
                format!("Configuração inválida: {msg}")
            }
        }
        AppError::MissingDependency(name) => {
            format!("Dependência ausente: {name}. Rode `cargo run -- doctor` para diagnóstico.")
        }
        AppError::CommandFailed(msg) => {
            if msg.contains("transcriber returned empty text") {
                "Whisper retornou texto vazio. Verifique o modelo e o áudio gravado.".into()
            } else {
                format!("Comando falhou: {msg}")
            }
        }
        AppError::Audio(_) => {
            "Erro ao acessar o microfone. Verifique se há um microfone disponível.".into()
        }
        AppError::Hotkey(_) => {
            "Erro ao registrar hotkey global. Verifique se há outro programa usando a mesma combinação.".into()
        }
        AppError::Unsupported(msg) => {
            format!("Recurso não disponível: {msg}")
        }
        _ => error.to_string(),
    }
}
