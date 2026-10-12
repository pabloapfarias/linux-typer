//! Injecao de input via libei (protocolo EI) com sessao do portal XDG
//! RemoteDesktop.
//!
//! Este backend usa a "porta oficial" do Wayland: o usuario autoriza uma
//! sessao de controle remoto no portal e o compositor entrega um socket EIS,
//! por onde enviamos o atalho de colar. Diferente de uinput/ydotool, nao exige
//! `/dev/uinput` nem o grupo `input`.
//!
//! A sessao do portal e criada uma unica vez por processo (o dialogo de
//! consentimento aparece so na primeira ditada) e reutilizada depois.
//!
//! Notas de protocolo (importantes):
//! - um dispositivo novo nasce no estado `paused`; so e permitido chamar
//!   `start_emulating` depois do evento `resumed` (caso contrario o compositor
//!   pode descartar os eventos silenciosamente);
//! - `frame` fecha um grupo logico de eventos: pressionar e soltar a mesma
//!   tecla na mesma frame e um noop; por isso cada transicao vai em sua
//!   propria frame.

use std::collections::HashMap;
use std::io;
use std::os::unix::io::{AsRawFd, RawFd};
use std::os::unix::net::UnixStream;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use ashpd::desktop::remote_desktop::{
    ConnectToEISOptions, DeviceType, RemoteDesktop, SelectDevicesOptions, StartOptions,
};
use ashpd::desktop::{CreateSessionOptions, PersistMode};
use enumflags2::BitFlags;
use log::{info, warn};
use reis::PendingRequestResult;
use reis::ei;
use reis::handshake::ei_handshake_blocking;

use crate::config::PasteShortcut;
use crate::error::{AppError, AppResult};

/// Keycodes evdev (nao XKB) usados no Ctrl+V / Ctrl+Shift+V.
const KEY_LEFTCTRL: u32 = 29;
const KEY_LEFTSHIFT: u32 = 42;
const KEY_V: u32 = 47;

/// Tempo maximo para o compositor liberar o teclado (evento `resumed`).
const DEVICE_TIMEOUT: Duration = Duration::from_secs(10);

struct LibeiSession {
    context: ei::Context,
    device: ei::Device,
    keyboard: ei::Keyboard,
    serial: u32,
    sequence: u32,
    emulating: bool,
    started: Instant,
}

static SESSION: Mutex<Option<LibeiSession>> = Mutex::new(None);

/// Envia o atalho de colagem via portal/libei.
pub fn send_paste_shortcut(shortcut: PasteShortcut) -> AppResult<()> {
    let mut guard = SESSION
        .lock()
        .map_err(|_| AppError::Unsupported("libei session mutex poisoned".into()))?;

    if guard.is_none() {
        *guard = Some(create_session()?);
    }

    let session = guard.as_mut().expect("session created above");
    if let Err(err) = session.send(shortcut) {
        // Sessao invalida (portal revogado, compositor desistiu, etc.):
        // sera recriada na proxima tentativa.
        warn!("libei session failed; will recreate on next use: {err}");
        *guard = None;
        return Err(err);
    }

    Ok(())
}

fn create_session() -> AppResult<LibeiSession> {
    let stream = futures_executor::block_on(open_eis_stream())?;
    let context = ei::Context::new(stream)
        .map_err(|err| AppError::Unsupported(format!("failed to create libei context: {err}")))?;

    let response = ei_handshake_blocking(
        &context,
        "linux-voice-typer",
        ei::handshake::ContextType::Sender,
    )
    .map_err(|err| AppError::Unsupported(format!("libei handshake failed: {err}")))?;

    info!("libei handshake completed (serial {})", response.serial);
    let (device, keyboard, serial) = discover_keyboard(&context)?;

    // O dispositivo ja veio `resumed`: agora e valido iniciar a emulacao.
    device.start_emulating(serial, 0);
    context
        .flush()
        .map_err(|err| AppError::Unsupported(format!("libei flush failed: {err}")))?;
    info!("libei keyboard device ready (serial {serial})");

    Ok(LibeiSession {
        context,
        device,
        keyboard,
        serial,
        sequence: 1,
        emulating: true,
        started: Instant::now(),
    })
}

async fn open_eis_stream() -> AppResult<UnixStream> {
    let remote_desktop = RemoteDesktop::new().await.map_err(|err| {
        AppError::Unsupported(format!(
            "XDG RemoteDesktop portal unavailable: {err}. Is a portal backend running?"
        ))
    })?;

    let session = remote_desktop
        .create_session(CreateSessionOptions::default())
        .await
        .map_err(|err| AppError::Unsupported(format!("portal create_session failed: {err}")))?;

    let options = SelectDevicesOptions::default()
        .set_devices(BitFlags::from(DeviceType::Keyboard))
        .set_persist_mode(PersistMode::DoNot);

    remote_desktop
        .select_devices(&session, options)
        .await
        .map_err(|err| AppError::Unsupported(format!("portal select_devices failed: {err}")))?;

    // Aqui aparece o dialogo de consentimento do desktop.
    remote_desktop
        .start(&session, None, StartOptions::default())
        .await
        .map_err(|err| {
            AppError::Unsupported(format!("portal start failed (consent denied?): {err}"))
        })?;

    let fd = remote_desktop
        .connect_to_eis(&session, ConnectToEISOptions::default())
        .await
        .map_err(|err| AppError::Unsupported(format!("portal connect_to_eis failed: {err}")))?;

    Ok(UnixStream::from(fd))
}

#[derive(Default)]
struct DeviceState {
    keyboard: Option<ei::Keyboard>,
    resumed: Option<u32>,
    done: bool,
}

/// Acompanha os eventos ate o compositor liberar o teclado (evento `resumed`).
fn discover_keyboard(context: &ei::Context) -> AppResult<(ei::Device, ei::Keyboard, u32)> {
    let mut seats: HashMap<ei::Seat, HashMap<String, u64>> = HashMap::new();
    let mut devices: HashMap<ei::Device, DeviceState> = HashMap::new();
    let deadline = Instant::now() + DEVICE_TIMEOUT;

    loop {
        if Instant::now() > deadline {
            return Err(AppError::Unsupported(
                "timed out waiting for the libei keyboard device to be resumed".into(),
            ));
        }

        wait_readable(context.as_raw_fd(), 200)?;
        context
            .read()
            .map_err(|err| AppError::Unsupported(format!("libei read failed: {err}")))?;

        while let Some(result) = context.pending_event() {
            let event = match result {
                PendingRequestResult::Request(event) => event,
                PendingRequestResult::ParseError(err) => {
                    return Err(AppError::Unsupported(format!("libei parse error: {err:?}")));
                }
                PendingRequestResult::InvalidObject(id) => {
                    warn!("libei: ignoring invalid object {id}");
                    continue;
                }
            };

            match event {
                ei::Event::Connection(_connection, request) => match request {
                    ei::connection::Event::Ping { ping } => ping.done(0),
                    ei::connection::Event::Disconnected {
                        reason,
                        explanation,
                        ..
                    } => {
                        return Err(AppError::Unsupported(format!(
                            "libei session disconnected (reason {reason:?}): {}",
                            explanation.unwrap_or_else(|| "no explanation".into())
                        )));
                    }
                    _ => {}
                },
                ei::Event::Seat(seat, request) => match request {
                    ei::seat::Event::Capability { mask, interface } => {
                        seats.entry(seat).or_default().insert(interface, mask);
                    }
                    ei::seat::Event::Done => {
                        if let Some(mask) = seats
                            .get(&seat)
                            .and_then(|capabilities| capabilities.get("ei_keyboard"))
                        {
                            seat.bind(*mask);
                        }
                    }
                    ei::seat::Event::Device { device } => {
                        devices.entry(device).or_default();
                    }
                    ei::seat::Event::Destroyed { .. } => {
                        seats.remove(&seat);
                    }
                    _ => {}
                },
                ei::Event::Device(device, request) => {
                    let state = devices.entry(device.clone()).or_default();
                    match request {
                        ei::device::Event::Interface { object } => {
                            if let Some(keyboard) = object.downcast::<ei::Keyboard>() {
                                state.keyboard = Some(keyboard);
                            }
                        }
                        ei::device::Event::Done => {
                            state.done = true;
                        }
                        ei::device::Event::Resumed { serial } => {
                            state.resumed = Some(serial);
                        }
                        ei::device::Event::Destroyed { .. } => {
                            devices.remove(&device);
                        }
                        _ => {}
                    }
                }
                ei::Event::Keyboard(_keyboard, _event) => {
                    // O keymap do compositor nao e necessario: usamos
                    // keycodes evdev fixos.
                }
                ei::Event::Handshake(_handshake, _event) => {
                    // Handshake ja concluido por `ei_handshake_blocking`.
                }
                _ => {}
            }

            if let Some((device, state)) = devices.iter().find(|(_, state)| {
                state.done && state.keyboard.is_some() && state.resumed.is_some()
            }) {
                let keyboard = state.keyboard.clone().expect("checked above");
                let serial = state.resumed.expect("checked above");
                return Ok((device.clone(), keyboard, serial));
            }
        }

        context
            .flush()
            .map_err(|err| AppError::Unsupported(format!("libei flush failed: {err}")))?;
    }
}

impl LibeiSession {
    fn send(&mut self, shortcut: PasteShortcut) -> AppResult<()> {
        self.pump();
        self.wait_resumed()?;

        self.key(KEY_LEFTCTRL, true);
        if shortcut == PasteShortcut::CtrlShiftV {
            self.key(KEY_LEFTSHIFT, true);
        }
        self.key(KEY_V, true);
        self.key(KEY_V, false);
        if shortcut == PasteShortcut::CtrlShiftV {
            self.key(KEY_LEFTSHIFT, false);
        }
        self.key(KEY_LEFTCTRL, false);

        self.context
            .flush()
            .map_err(|err| AppError::Unsupported(format!("libei flush failed: {err}")))?;
        Ok(())
    }

    /// Cada transicao de tecla vai em sua propria frame (press+release na
    /// mesma frame seria um noop).
    fn key(&self, code: u32, pressed: bool) {
        let state = if pressed {
            ei::keyboard::KeyState::Press
        } else {
            ei::keyboard::KeyState::Released
        };

        self.keyboard.key(code, state);
        self.device
            .frame(self.serial, self.started.elapsed().as_micros() as u64);
    }

    /// Processa eventos pendentes (pings, suspensao/retomada do dispositivo).
    fn pump(&mut self) {
        if context_read(&self.context).is_err() {
            return;
        }

        while let Some(result) = self.context.pending_event() {
            let event = match result {
                PendingRequestResult::Request(event) => event,
                _ => continue,
            };

            match event {
                ei::Event::Connection(_connection, ei::connection::Event::Ping { ping }) => {
                    ping.done(0);
                }
                ei::Event::Device(device, ei::device::Event::Resumed { serial })
                    if device == self.device =>
                {
                    // Dispositivo liberado: reemula com a proxima sequencia.
                    self.serial = serial;
                    self.device.start_emulating(self.serial, self.sequence);
                    self.sequence = self.sequence.wrapping_add(1);
                    self.emulating = true;
                }
                ei::Event::Device(device, ei::device::Event::Paused { .. })
                    if device == self.device =>
                {
                    self.emulating = false;
                }
                _ => {}
            }
        }

        let _ = self.context.flush();
    }

    fn wait_resumed(&mut self) -> AppResult<()> {
        if self.emulating {
            return Ok(());
        }

        let deadline = Instant::now() + Duration::from_millis(1500);
        while Instant::now() < deadline {
            wait_readable(self.context.as_raw_fd(), 50)?;
            self.pump();
            if self.emulating {
                return Ok(());
            }
        }

        Err(AppError::Unsupported(
            "libei keyboard device is paused by the compositor".into(),
        ))
    }
}

fn context_read(context: &ei::Context) -> AppResult<()> {
    context
        .read()
        .map_err(|err| AppError::Unsupported(format!("libei read failed: {err}")))?;
    Ok(())
}

fn wait_readable(fd: RawFd, timeout_ms: i32) -> AppResult<()> {
    let mut poll_fd = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };

    loop {
        let result = unsafe { libc::poll(&mut poll_fd, 1, timeout_ms) };
        if result >= 0 {
            return Ok(());
        }

        let err = io::Error::last_os_error();
        if err.kind() != io::ErrorKind::Interrupted {
            return Err(AppError::Io(err));
        }
    }
}
