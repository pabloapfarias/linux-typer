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
use crate::hotkey::{HotkeyListener, TriggerEvent};
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
            })),
        }
    }

    pub fn start(&self) -> AppResult<()> {
        let mut inner = self.lock_inner()?;
        if inner.running {
            push_event(&mut inner, "Serviço já estava rodando");
            return Ok(());
        }

        let config = inner.config.clone();
        let shutdown = Arc::new(AtomicBool::new(false));
        let worker_shutdown = shutdown.clone();
        let service_inner = self.inner.clone();

        inner.running = true;
        inner.shutdown = Some(shutdown);
        push_event(&mut inner, "Serviço iniciado");

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
        let handle = {
            let mut inner = self.lock_inner()?;
            if !inner.running {
                push_event(&mut inner, "Serviço já estava parado");
                return Ok(());
            }

            if let Some(shutdown) = &inner.shutdown {
                shutdown.store(true, Ordering::Relaxed);
            }
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
        Ok(())
    }

    pub fn restart(&self) -> AppResult<()> {
        self.stop()?;
        self.start()
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

    with_inner(&service_inner, |inner| {
        push_event(inner, "Hotkey global ativa");
    });

    let mut recorder: Option<Recorder> = None;
    let mut record_start: Option<Instant> = None;
    let mut last_tick = Instant::now();

    while !shutdown.load(Ordering::Relaxed) {
        let event_opt = listener.recv_timeout(Duration::from_millis(50));

        // If recording, tick audio level and elapsed timer every 60ms
        if let Some(active) = &recorder {
            if last_tick.elapsed() >= Duration::from_millis(60) {
                last_tick = Instant::now();
                let elapsed = record_start.map(|t| t.elapsed().as_secs()).unwrap_or(0);
                let lvl = active.current_level();
                notify_feedback(&service_inner, ServiceState::Recording, None, lvl, elapsed);
            }
        }

        let Some(event) = event_opt else {
            continue;
        };

        match event {
            TriggerEvent::Start => {
                if recorder.is_some() {
                    continue;
                }

                record_start = Some(Instant::now());
                show_osd_notification(ServiceState::Recording, None);
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
                    }
                }
            }
            TriggerEvent::Stop => {
                let Some(active) = recorder.take() else {
                    continue;
                };
                record_start = None;

                show_osd_notification(ServiceState::Transcribing, None);
                notify_feedback(&service_inner, ServiceState::Transcribing, None, 0.0, 0);

                with_inner(&service_inner, |inner| {
                    push_event(inner, "Gravação finalizada");
                });

                let service_inner_cloned = service_inner.clone();
                let config_cloned = config.clone();

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

                    match result {
                        Ok(transcript) => {
                            let total_elapsed = trans_start.elapsed().as_secs();
                            show_osd_notification(ServiceState::Idle, Some(&transcript));
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
                            show_osd_notification(ServiceState::Idle, None);
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
                        }
                    }
                });
            }
        }
    }

    show_osd_notification(ServiceState::Idle, None);
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

fn show_osd_notification(state: ServiceState, transcript: Option<&str>) {
    let (icon, expire, title, body) = match state {
        ServiceState::Recording => (
            "audio-input-microphone",
            "0", // NEVER EXPIRE while recording!
            "🎙️ Escutando...",
            "Fale agora no microfone...",
        ),
        ServiceState::Transcribing => (
            "system-run",
            "0", // NEVER EXPIRE while transcribing!
            "⏳ Transcrevendo...",
            "Processando áudio com Whisper...",
        ),
        ServiceState::Idle => {
            if let Some(text) = transcript {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    return;
                }
                let preview = if trimmed.chars().count() > 60 {
                    let cut: String = trimmed.chars().take(60).collect();
                    format!("{cut}...")
                } else {
                    trimmed.to_string()
                };
                (
                    "emblem-default",
                    "3500", // 3.5s for done
                    "✓ Ditado Colado!",
                    preview.leak() as &str,
                )
            } else {
                return;
            }
        }
    };

    let _ = std::process::Command::new("notify-send")
        .args([
            "-r",
            "4242",
            "-a",
            "Linux Voice Typer",
            "-i",
            icon,
            "-e",
            "-t",
            expire,
            title,
            body,
        ])
        .spawn();
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
    if cpal::default_host().default_input_device().is_some() {
        "Microfone padrão detectado".to_string()
    } else {
        "Microfone não detectado".to_string()
    }
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
    }
}
