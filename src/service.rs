use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use cpal::traits::HostTrait;
use serde::Serialize;

use crate::audio::Recorder;
use crate::config::{Config, PasteBackend, PasteShortcut};
use crate::error::{AppError, AppResult};
use crate::hotkey::{HotkeyListener, TriggerEvent};
use crate::process_recording;

const MAX_EVENTS: usize = 80;

#[derive(Debug, Clone, Serialize)]
pub struct VoiceTyperStatus {
    pub running: bool,
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
    shutdown: Option<Arc<AtomicBool>>,
    handle: Option<JoinHandle<()>>,
    last_transcript: Option<String>,
    events: VecDeque<String>,
}

impl VoiceTyperService {
    pub fn new(config: Config) -> Self {
        Self {
            inner: Arc::new(Mutex::new(ServiceInner {
                config,
                running: false,
                shutdown: None,
                handle: None,
                last_transcript: None,
                events: VecDeque::new(),
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

    while !shutdown.load(Ordering::Relaxed) {
        let Some(event) = listener.recv_timeout(Duration::from_millis(100)) else {
            continue;
        };

        match event {
            TriggerEvent::Start => {
                if recorder.is_some() {
                    continue;
                }

                with_inner(&service_inner, |inner| {
                    push_event(inner, "Gravação iniciada");
                });

                match Recorder::start() {
                    Ok(active) => {
                        recorder = Some(active);
                    }
                    Err(err) => with_inner(&service_inner, |inner| {
                        push_event(inner, format!("Falha ao gravar: {err}"));
                    }),
                }
            }
            TriggerEvent::Stop => {
                let Some(active) = recorder.take() else {
                    continue;
                };

                with_inner(&service_inner, |inner| {
                    push_event(inner, "Gravação finalizada");
                });

                match process_recording(active, &config) {
                    Ok(transcript) => with_inner(&service_inner, |inner| {
                        inner.last_transcript = Some(transcript);
                        push_event(inner, "Transcrição concluída");
                        push_event(
                            inner,
                            format!(
                                "Texto colado com {}",
                                match config.paste_shortcut {
                                    PasteShortcut::CtrlV => "Ctrl+V",
                                    PasteShortcut::CtrlShiftV => "Ctrl+Shift+V",
                                }
                            ),
                        );
                    }),
                    Err(err) => with_inner(&service_inner, |inner| {
                        push_event(inner, format!("Pipeline falhou: {err}"));
                    }),
                }
            }
        }
    }

    with_inner(&service_inner, |inner| {
        inner.running = false;
        inner.shutdown = None;
        push_event(inner, "Loop do serviço encerrado");
    });
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
        hotkey: "Ctrl+Alt+Space".into(),
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
    }
}
