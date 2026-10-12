use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use cpal::traits::HostTrait;
use serde::Serialize;

use crate::audio::Recorder;
use crate::config::{Config, PasteBackend, PasteShortcut};
use crate::error::{AppError, AppResult};
use crate::hotkey::{HotkeyListener, HotkeyMode, TriggerEvent};
use crate::notify;
use crate::process_recording;

const MAX_EVENTS: usize = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceState {
    Idle,
    Recording,
    Transcribing,
}

#[derive(Debug, Clone, Serialize)]
pub struct DictationFeedback {
    pub state: ServiceState,
    pub transcript: Option<String>,
    pub level: f32,
    pub elapsed_secs: u64,
}

pub type StateListener = Arc<dyn Fn(&DictationFeedback) + Send + Sync>;

#[derive(Debug, Clone, Serialize)]
pub struct VoiceTyperStatus {
    pub running: bool,
    pub state: ServiceState,
    pub hotkey: String,
    pub paste_mode: String,
    pub paste_backend: String,
    pub language: String,
    pub model_path: String,
    pub whisper_bin: String,
    pub microphone: String,
    pub last_transcript: Option<String>,
    pub recent_events: Vec<String>,
}

#[derive(Clone)]
pub struct VoiceTyperService {
    inner: Arc<Mutex<ServiceInner>>,
}

struct ServiceInner {
    config: Config,
    running: bool,
    state: ServiceState,
    shutdown: Option<Arc<AtomicBool>>,
    handle: Option<JoinHandle<()>>,
    last_transcript: Option<String>,
    events: VecDeque<String>,
    listener: Option<StateListener>,
    session: u64,
}

impl VoiceTyperService {
    pub fn new(config: Config) -> Self {
        Self {
            inner: Arc::new(Mutex::new(ServiceInner {
                config,
                running: false,
                state: ServiceState::Idle,
                shutdown: None,
                handle: None,
                last_transcript: None,
                events: VecDeque::new(),
                listener: None,
                session: 0,
            })),
        }
    }

    pub fn start(&self) -> AppResult<()> {
        self.start_inner(true)
    }

    fn start_inner(&self, notify: bool) -> AppResult<()> {
        let mut inner = self.lock_inner()?;
        if inner.running {
            push_event(&mut inner, "Serviço já estava rodando");
            return Ok(());
        }

        let config = inner.config.clone();
        let shutdown = Arc::new(AtomicBool::new(false));
        let worker_shutdown = shutdown.clone();
        let service_inner = self.inner.clone();
        let osd_enabled = inner.config.osd_enabled;

        inner.running = true;
        inner.shutdown = Some(shutdown);
        push_event(&mut inner, "Serviço iniciado");

        if notify && osd_enabled {
            notify::send(
                "media-playback-start",
                "2500",
                "▶️ Serviço iniciado",
                "O atalho global está ativo.",
            );
        }

        inner.handle = Some(
            thread::Builder::new()
                .name("voice-typer-service".into())
                .spawn(move || run_service_loop(service_inner, config, worker_shutdown))
                .map_err(|err| {
                    AppError::Unsupported(format!("failed to spawn service thread: {err}"))
                })?,
        );

        Ok(())
    }

    pub fn stop(&self) -> AppResult<()> {
        self.stop_inner(true)
    }

    fn stop_inner(&self, notify: bool) -> AppResult<()> {
        let handle = {
            let mut inner = self.lock_inner()?;
            if !inner.running {
                push_event(&mut inner, "Serviço já estava parado");
                return Ok(());
            }

            if let Some(shutdown) = &inner.shutdown {
                shutdown.store(true, Ordering::Relaxed);
            }
            // Invalida transcricoes em andamento: resultados antigos nao podem
            // sobrescrever o estado depois que o servico parar.
            inner.session = inner.session.wrapping_add(1);
            push_event(&mut inner, "Parando serviço");
            inner.handle.take()
        };

        if let Some(handle) = handle {
            let _ = handle.join();
        }

        let mut inner = self.lock_inner()?;
        inner.running = false;
        inner.shutdown = None;
        push_event(&mut inner, "Serviço parado");

        if notify && inner.config.osd_enabled {
            notify::send(
                "media-playback-pause",
                "2500",
                "⏸️ Serviço parado",
                "Use o tray ou a janela para reativar.",
            );
        }

        Ok(())
    }

    pub fn restart(&self) -> AppResult<()> {
        self.stop_inner(false)?;
        self.start_inner(false)
    }

    pub fn is_running(&self) -> bool {
        self.inner
            .lock()
            .map(|inner| inner.running)
            .unwrap_or(false)
    }

    pub fn reload_config(&self, config: Config) -> AppResult<()> {
        let mut inner = self.lock_inner()?;
        inner.config = config;
        push_event(&mut inner, "Configuração recarregada");
        Ok(())
    }

    pub fn set_paste_shortcut(&self, shortcut: PasteShortcut) -> AppResult<()> {
        let mut inner = self.lock_inner()?;
        inner.config.paste_shortcut = shortcut;
        push_event(
            &mut inner,
            match shortcut {
                PasteShortcut::CtrlV => "Modo Editor selecionado",
                PasteShortcut::CtrlShiftV => "Modo Terminal selecionado",
            },
        );
        Ok(())
    }

    pub fn set_paste_backend(&self, backend: PasteBackend) -> AppResult<()> {
        let mut inner = self.lock_inner()?;
        inner.config.paste_backend = backend;
        push_event(
            &mut inner,
            format!("Backend definido: {}", backend.as_str()),
        );
        Ok(())
    }

    pub fn set_state_listener(&self, listener: StateListener) -> AppResult<()> {
        let mut inner = self.lock_inner()?;
        inner.listener = Some(listener);
        Ok(())
    }

    pub fn last_transcript(&self) -> Option<String> {
        self.inner
            .lock()
            .ok()
            .and_then(|inner| inner.last_transcript.clone())
    }

    pub fn recent_events(&self) -> Vec<String> {
        self.inner
            .lock()
            .map(|inner| inner.events.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn status(&self) -> VoiceTyperStatus {
        let inner = self.inner.lock().ok();
        let config = inner
            .as_ref()
            .map(|inner| inner.config.clone())
            .unwrap_or_else(fallback_config);

        let paste_backend = config.effective_paste_backend().as_str().to_string();

        VoiceTyperStatus {
            running: inner.as_ref().map(|inner| inner.running).unwrap_or(false),
            state: inner
                .as_ref()
                .map(|inner| inner.state)
                .unwrap_or(ServiceState::Idle),
            hotkey: config.hotkey,
            paste_mode: paste_mode_label(config.paste_shortcut).to_string(),
            paste_backend,
            language: config.language,
            model_path: config.model_path.display().to_string(),
            whisper_bin: config.whisper_bin.display().to_string(),
            microphone: default_microphone_name(),
            last_transcript: inner
                .as_ref()
                .and_then(|inner| inner.last_transcript.clone()),
            recent_events: inner
                .as_ref()
                .map(|inner| inner.events.iter().cloned().collect())
                .unwrap_or_default(),
        }
    }

    fn lock_inner(&self) -> AppResult<std::sync::MutexGuard<'_, ServiceInner>> {
        self.inner
            .lock()
            .map_err(|_| AppError::Unsupported("voice typer service mutex poisoned".into()))
    }
}

fn run_service_loop(
    service_inner: Arc<Mutex<ServiceInner>>,
    config: Config,
    shutdown: Arc<AtomicBool>,
) {
    let listener = match HotkeyListener::start(&config.hotkey, false, shutdown.clone()) {
        Ok(listener) => listener,
        Err(err) => {
            with_inner(&service_inner, |inner| {
                inner.running = false;
                push_event(inner, format!("Falha ao iniciar hotkey: {err}"));
            });
            return;
        }
    };

    match listener.mode {
        HotkeyMode::Global => {
            with_inner(&service_inner, |inner| {
                push_event(inner, format!("Hotkey global ativa: {}", config.hotkey));
            });
        }
        HotkeyMode::Terminal => {
            with_inner(&service_inner, |inner| {
                push_event(
                    inner,
                    "Hotkey global indisponível; modo terminal ativo (sem suporte na GUI)",
                );
            });
            if config.osd_enabled {
                notify::send(
                    "dialog-warning",
                    "5000",
                    "⚠️ Hotkey global indisponível",
                    "Não foi possível registrar o atalho global. Verifique permissões de entrada ou conflito com outro aplicativo.",
                );
            }
        }
    }

    let mut recorder: Option<Recorder> = None;
    let mut record_start: Option<Instant> = None;
    let mut session_id: u64 = 0;
    let mut last_tick = Instant::now();

    while !shutdown.load(Ordering::Relaxed) {
        let event_opt = listener.recv_timeout(Duration::from_millis(50));

        // If recording, tick audio level and elapsed timer every 60ms
        if let Some(active) = &recorder
            && last_tick.elapsed() >= Duration::from_millis(60)
        {
            last_tick = Instant::now();
            let elapsed = record_start.map(|t| t.elapsed().as_secs()).unwrap_or(0);
            let lvl = active.current_level();
            notify_feedback(&service_inner, ServiceState::Recording, None, lvl, elapsed);
        }

        let Some(event) = event_opt else {
            continue;
        };

        match event {
            TriggerEvent::Start => {
                if recorder.is_some() {
                    continue;
                }

                session_id = {
                    let Ok(mut inner) = service_inner.lock() else {
                        continue;
                    };
                    inner.session = inner.session.wrapping_add(1);
                    inner.session
                };

                record_start = Some(Instant::now());
                show_osd_notification(ServiceState::Recording, None, config.osd_enabled);
                notify_feedback(&service_inner, ServiceState::Recording, None, 0.0, 0);

                with_inner(&service_inner, |inner| {
                    push_event(inner, "Gravação iniciada");
                });

                match Recorder::start() {
                    Ok(active) => {
                        recorder = Some(active);
                    }
                    Err(err) => {
                        record_start = None;
                        notify_feedback(&service_inner, ServiceState::Idle, None, 0.0, 0);
                        with_inner(&service_inner, |inner| {
                            push_event(inner, format!("Falha ao gravar: {err}"));
                        });

                        if config.osd_enabled {
                            notify::send(
                                "dialog-error",
                                "4000",
                                "⚠️ Falha ao gravar",
                                &friendly_error_message(&err),
                            );
                        }
                    }
                }
            }
            TriggerEvent::Stop => {
                let Some(active) = recorder.take() else {
                    continue;
                };
                record_start = None;

                show_osd_notification(ServiceState::Transcribing, None, config.osd_enabled);
                notify_feedback(&service_inner, ServiceState::Transcribing, None, 0.0, 0);

                with_inner(&service_inner, |inner| {
                    push_event(inner, "Gravação finalizada");
                });

                let service_inner_cloned = service_inner.clone();
                let config_cloned = config.clone();
                let session = session_id;

                thread::spawn(move || {
                    let trans_start = Instant::now();
                    let done = Arc::new(AtomicBool::new(false));
                    let done_ticker = done.clone();
                    let inner_ticker = service_inner_cloned.clone();

                    let ticker_handle = thread::spawn(move || {
                        while !done_ticker.load(Ordering::Relaxed) {
                            thread::sleep(Duration::from_millis(500));
                            if done_ticker.load(Ordering::Relaxed) {
                                break;
                            }

                            // Uma nova gravacao (ou a parada do servico) invalida
                            // este ticker; o estado atual nao pode ser sobrescrito.
                            let still_current = inner_ticker
                                .lock()
                                .map(|inner| inner.session == session)
                                .unwrap_or(false);
                            if !still_current {
                                break;
                            }

                            let elapsed = trans_start.elapsed().as_secs();
                            notify_feedback(
                                &inner_ticker,
                                ServiceState::Transcribing,
                                None,
                                0.0,
                                elapsed,
                            );
                        }
                    });

                    let result = process_recording(active, &config_cloned);
                    done.store(true, Ordering::Relaxed);
                    let _ = ticker_handle.join();

                    let still_current = service_inner_cloned
                        .lock()
                        .map(|inner| inner.session == session)
                        .unwrap_or(false);
                    if !still_current {
                        // Sessao antiga: descarta estado, OSD, transcricao e eventos.
                        return;
                    }

                    match result {
                        Ok(transcript) => {
                            let total_elapsed = trans_start.elapsed().as_secs();
                            show_osd_notification(
                                ServiceState::Idle,
                                Some(&transcript),
                                config_cloned.osd_enabled,
                            );
                            notify_feedback(
                                &service_inner_cloned,
                                ServiceState::Idle,
                                Some(&transcript),
                                0.0,
                                total_elapsed,
                            );
                            with_inner(&service_inner_cloned, |inner| {
                                inner.last_transcript = Some(transcript);
                                push_event(inner, "Transcrição concluída");
                                push_event(
                                    inner,
                                    format!(
                                        "Texto colado com {}",
                                        match config_cloned.paste_shortcut {
                                            PasteShortcut::CtrlV => "Ctrl+V",
                                            PasteShortcut::CtrlShiftV => "Ctrl+Shift+V",
                                        }
                                    ),
                                );
                            });
                        }
                        Err(err) => {
                            notify_feedback(
                                &service_inner_cloned,
                                ServiceState::Idle,
                                None,
                                0.0,
                                0,
                            );
                            with_inner(&service_inner_cloned, |inner| {
                                push_event(inner, format!("Pipeline falhou: {err}"));
                            });

                            if config_cloned.osd_enabled {
                                notify::send(
                                    "dialog-error",
                                    "4000",
                                    "⚠️ Ditado falhou",
                                    &friendly_error_message(&err),
                                );
                            }
                        }
                    }
                });
            }
        }
    }

    show_osd_notification(ServiceState::Idle, None, config.osd_enabled);
    notify_feedback(&service_inner, ServiceState::Idle, None, 0.0, 0);

    with_inner(&service_inner, |inner| {
        inner.running = false;
        inner.shutdown = None;
        push_event(inner, "Loop do serviço encerrado");
    });
}

fn notify_feedback(
    service_inner: &Arc<Mutex<ServiceInner>>,
    state: ServiceState,
    transcript: Option<&str>,
    level: f32,
    elapsed_secs: u64,
) {
    let listener = {
        let Ok(mut inner) = service_inner.lock() else {
            return;
        };
        inner.state = state;
        inner.listener.clone()
    };

    if let Some(listener) = listener {
        listener(&DictationFeedback {
            state,
            transcript: transcript.map(|s| s.to_string()),
            level,
            elapsed_secs,
        });
    }
}

fn osd_content(
    state: ServiceState,
    transcript: Option<&str>,
) -> (&'static str, &'static str, &'static str, String) {
    match state {
        ServiceState::Recording => (
            "audio-input-microphone",
            "0", // NEVER EXPIRE while recording!
            "🎙️ Escutando...",
            "Fale agora no microfone...".to_string(),
        ),
        ServiceState::Transcribing => (
            "system-run",
            "0", // NEVER EXPIRE while transcribing!
            "⏳ Transcrevendo...",
            "Processando áudio com Whisper...".to_string(),
        ),
        ServiceState::Idle => {
            let preview = transcript
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .map(|text| truncate_chars(text, 60));

            match preview {
                Some(preview) => ("emblem-default", "3500", "✓ Ditado Colado!", preview),
                None => (
                    "edit-clear",
                    "1500",
                    "🎙️ Pronto",
                    "Pode ditar novamente quando quiser.".to_string(),
                ),
            }
        }
    }
}

fn show_osd_notification(state: ServiceState, transcript: Option<&str>, osd_enabled: bool) {
    if !osd_enabled {
        return;
    }

    let (icon, expire, title, body) = osd_content(state, transcript);
    notify::send(icon, expire, title, &body);
}

fn friendly_error_message(err: &AppError) -> String {
    let raw = err.to_string();
    let friendly = if raw.contains("transcriber returned empty text")
        || raw.contains("returned empty transcription")
    {
        "Whisper retornou texto vazio. Verifique o modelo e a gravação.".to_string()
    } else if raw.contains("no audio samples captured") {
        "Nenhum áudio capturado. Verifique o microfone.".to_string()
    } else if raw.contains("wl-copy") || raw.contains("wl-clipboard") {
        "Clipboard indisponível (wl-clipboard). Verifique a sessão Wayland.".to_string()
    } else if raw.contains("whisper binary not found") || raw.contains("whisper model not found") {
        "Whisper não encontrado. Verifique os caminhos em Configurações.".to_string()
    } else {
        raw
    };

    truncate_chars(&friendly, 140)
}

fn truncate_chars(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_string()
    } else {
        let cut: String = text.chars().take(max).collect();
        format!("{cut}...")
    }
}

fn with_inner(inner: &Arc<Mutex<ServiceInner>>, action: impl FnOnce(&mut ServiceInner)) {
    if let Ok(mut guard) = inner.lock() {
        action(&mut guard);
    }
}

fn push_event(inner: &mut ServiceInner, message: impl Into<String>) {
    if inner.events.len() >= MAX_EVENTS {
        inner.events.pop_front();
    }
    inner.events.push_back(format!(
        "{} {}",
        chrono::Local::now().format("%H:%M"),
        message.into()
    ));
}

fn paste_mode_label(shortcut: PasteShortcut) -> &'static str {
    match shortcut {
        PasteShortcut::CtrlV => "Editor",
        PasteShortcut::CtrlShiftV => "Terminal",
    }
}

fn default_microphone_name() -> String {
    const TTL: Duration = Duration::from_secs(10);
    static CACHE: Mutex<Option<(Instant, String)>> = Mutex::new(None);

    // O status e consultado a cada segundo pela UI; evitar reenumerar o
    // dispositivo a cada chamada (cache curto para detectar mudancas).
    if let Ok(guard) = CACHE.lock()
        && let Some((at, name)) = guard.as_ref()
        && at.elapsed() < TTL
    {
        return name.clone();
    }

    let name = if cpal::default_host().default_input_device().is_some() {
        "Microfone padrão detectado".to_string()
    } else {
        "Microfone não detectado".to_string()
    };

    if let Ok(mut guard) = CACHE.lock() {
        *guard = Some((Instant::now(), name.clone()));
    }

    name
}

fn fallback_config() -> Config {
    Config {
        hotkey: "Control+Alt+Space".into(),
        language: "pt".into(),
        model_path: "".into(),
        whisper_bin: "".into(),
        insert_mode: "clipboard".into(),
        restore_clipboard: false,
        auto_paste: true,
        paste_backend: PasteBackend::Uinput,
        paste_shortcut: PasteShortcut::CtrlV,
        trim_text: true,
        temp_dir: std::env::temp_dir(),
        auto_start_service: true,
        start_minimized: false,
        start_with_system: false,
        close_to_tray: true,
        osd_enabled: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_restart() {
        let config = fallback_config();
        let service = VoiceTyperService::new(config);
        assert!(service.start().is_ok());
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(service.is_running());
        assert!(service.restart().is_ok());
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(service.is_running());
        assert!(service.stop().is_ok());
        assert!(!service.is_running());
    }

    #[test]
    fn test_osd_recording_never_expires() {
        let (icon, expire, _, _) = osd_content(ServiceState::Recording, None);
        assert_eq!(icon, "audio-input-microphone");
        assert_eq!(expire, "0");
    }

    #[test]
    fn test_osd_idle_without_transcript_clears_notification() {
        let (icon, expire, _, _) = osd_content(ServiceState::Idle, None);
        assert_eq!(icon, "edit-clear");
        assert_eq!(expire, "1500");

        let (icon, _, _, _) = osd_content(ServiceState::Idle, Some("   "));
        assert_eq!(icon, "edit-clear");
    }

    #[test]
    fn test_osd_done_preview_is_truncated() {
        let long = "a".repeat(80);
        let (icon, expire, _, body) = osd_content(ServiceState::Idle, Some(&long));
        assert_eq!(icon, "emblem-default");
        assert_eq!(expire, "3500");
        assert_eq!(body.chars().count(), 63);
    }

    #[test]
    fn test_friendly_error_message_maps_empty_transcript() {
        let err = AppError::CommandFailed("transcriber returned empty text".into());
        assert!(friendly_error_message(&err).contains("texto vazio"));
    }

    #[test]
    fn test_truncate_chars_limits_length() {
        let long = "x".repeat(200);
        let cut = truncate_chars(&long, 140);
        assert_eq!(cut.chars().count(), 143);
        assert!(cut.ends_with("..."));
    }
}
