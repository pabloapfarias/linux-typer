use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use keytap::Key;
use keytap::chord::{Chord, ChordEvent, ChordMatcher};
use log::{info, warn};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerEvent {
    Start,
    Stop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyMode {
    Global,
    Terminal,
}

pub struct HotkeyListener {
    rx: Receiver<TriggerEvent>,
    _guard: ListenerGuard,
    pub mode: HotkeyMode,
}

enum ListenerGuard {
    Global {
        _matcher: Arc<ChordMatcher<&'static str>>,
        _worker: thread::JoinHandle<()>,
    },
    Terminal {
        _worker: thread::JoinHandle<()>,
    },
}

impl HotkeyListener {
    pub fn start(
        hotkey: &str,
        force_terminal: bool,
        stop_flag: Arc<AtomicBool>,
    ) -> AppResult<Self> {
        let (tx, rx) = mpsc::channel();

        if force_terminal {
            let worker = spawn_terminal_listener(tx, stop_flag);
            return Ok(Self {
                rx,
                _guard: ListenerGuard::Terminal { _worker: worker },
                mode: HotkeyMode::Terminal,
            });
        }

        match spawn_global_listener(hotkey, tx.clone(), stop_flag.clone()) {
            Ok((matcher, worker)) => Ok(Self {
                rx,
                _guard: ListenerGuard::Global {
                    _matcher: matcher,
                    _worker: worker,
                },
                mode: HotkeyMode::Global,
            }),
            Err(err) => {
                warn!("global hotkey unavailable: {err}");
                warn!("falling back to terminal hotkey mode");
                let worker = spawn_terminal_listener(tx, stop_flag);
                Ok(Self {
                    rx,
                    _guard: ListenerGuard::Terminal { _worker: worker },
                    mode: HotkeyMode::Terminal,
                })
            }
        }
    }

    pub fn recv_timeout(&self, duration: Duration) -> Option<TriggerEvent> {
        self.rx.recv_timeout(duration).ok()
    }
}

fn spawn_global_listener(
    hotkey: &str,
    tx: mpsc::Sender<TriggerEvent>,
    stop_flag: Arc<AtomicBool>,
) -> AppResult<(Arc<ChordMatcher<&'static str>>, thread::JoinHandle<()>)> {
    let chord = parse_hotkey(hotkey)?;
    let matcher = Arc::new(ChordMatcher::builder().add("ptt", chord).build()?);

    info!("global hotkey active: {hotkey}");

    let worker = thread::Builder::new()
        .name("hotkey-global-worker".into())
        .spawn({
            let stop_flag = stop_flag.clone();
            let matcher = matcher.clone();
            move || {
                while !stop_flag.load(Ordering::Relaxed) {
                    match matcher.recv_timeout(Duration::from_millis(50)) {
                        Ok(ChordEvent::Start { .. }) => {
                            let _ = tx.send(TriggerEvent::Start);
                        }
                        Ok(ChordEvent::End { .. }) => {
                            let _ = tx.send(TriggerEvent::Stop);
                        }
                        Err(_) => continue,
                    }
                }
            }
        })
        .map_err(|err| AppError::Unsupported(format!("failed to spawn hotkey thread: {err}")))?;

    Ok((matcher, worker))
}

fn spawn_terminal_listener(
    tx: mpsc::Sender<TriggerEvent>,
    stop_flag: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    info!("terminal hotkey mode active");
    info!("press Enter to start recording, then Enter again to stop");

    thread::Builder::new()
        .name("hotkey-terminal-worker".into())
        .spawn(move || {
            let stdin = std::io::stdin();
            let mut line = String::new();
            let mut recording = false;

            while !stop_flag.load(Ordering::Relaxed) {
                line.clear();
                if stdin.read_line(&mut line).is_err() {
                    continue;
                }

                let event = if recording {
                    TriggerEvent::Stop
                } else {
                    TriggerEvent::Start
                };
                recording = !recording;
                let _ = tx.send(event);
            }
        })
        .expect("failed to spawn terminal hotkey worker")
}

fn parse_hotkey(value: &str) -> AppResult<Chord> {
    let mut keys = Vec::new();

    for part in value.split('+') {
        let token = part.trim().to_ascii_lowercase();
        let key = match token.as_str() {
            "ctrl" | "control" => Key::ControlLeft,
            "alt" => Key::AltLeft,
            "shift" => Key::ShiftLeft,
            "super" | "meta" | "win" => Key::MetaLeft,
            "space" => Key::Space,
            "enter" | "return" => Key::Enter,
            "tab" => Key::Tab,
            "esc" | "escape" => Key::Escape,
            "backspace" => Key::Backspace,
            "up" => Key::ArrowUp,
            "down" => Key::ArrowDown,
            "left" => Key::ArrowLeft,
            "right" => Key::ArrowRight,
            "f1" => Key::F1,
            "f2" => Key::F2,
            "f3" => Key::F3,
            "f4" => Key::F4,
            "f5" => Key::F5,
            "f6" => Key::F6,
            "f7" => Key::F7,
            "f8" => Key::F8,
            "f9" => Key::F9,
            "f10" => Key::F10,
            "f11" => Key::F11,
            "f12" => Key::F12,
            "a" => Key::A,
            "b" => Key::B,
            "c" => Key::C,
            "d" => Key::D,
            "e" => Key::E,
            "f" => Key::F,
            "g" => Key::G,
            "h" => Key::H,
            "i" => Key::I,
            "j" => Key::J,
            "k" => Key::K,
            "l" => Key::L,
            "m" => Key::M,
            "n" => Key::N,
            "o" => Key::O,
            "p" => Key::P,
            "q" => Key::Q,
            "r" => Key::R,
            "s" => Key::S,
            "t" => Key::T,
            "u" => Key::U,
            "v" => Key::V,
            "w" => Key::W,
            "x" => Key::X,
            "y" => Key::Y,
            "z" => Key::Z,
            "0" => Key::Digit0,
            "1" => Key::Digit1,
            "2" => Key::Digit2,
            "3" => Key::Digit3,
            "4" => Key::Digit4,
            "5" => Key::Digit5,
            "6" => Key::Digit6,
            "7" => Key::Digit7,
            "8" => Key::Digit8,
            "9" => Key::Digit9,
            _ => {
                return Err(AppError::InvalidConfig(format!(
                    "unsupported hotkey token: {part}"
                )));
            }
        };
        keys.push(key);
    }

    if keys.is_empty() {
        return Err(AppError::InvalidConfig("hotkey cannot be empty".into()));
    }

    Ok(Chord::of(keys))
}

#[allow(dead_code)]
fn _debug_path(path: PathBuf) -> PathBuf {
    path
}
