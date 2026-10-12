use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::thread;

use log::warn;

/// ID da ultima notificacao entregue pelo daemon (`notify-send -p`).
///
/// O `-r` so substitui uma notificacao existente quando recebe o ID que o
/// daemon atribuiu; um ID fixo inventado nao substitui nada e empilha avisos.
static LAST_ID: Mutex<Option<String>> = Mutex::new(None);

/// Notificacoes de desktop (OSD) via `notify-send`.
///
/// Cada aviso substitui o anterior usando o ID devolvido pelo daemon.
pub fn send(icon: &str, expire_ms: &str, title: &str, body: &str) {
    if !is_available() {
        return;
    }

    let mut command = Command::new("notify-send");
    command.args([
        "-p",
        "-a",
        "Linux Voice Typer",
        "-i",
        icon,
        "-e",
        "-t",
        expire_ms,
    ]);

    if let Some(id) = last_id() {
        command.args(["-r", id.as_str()]);
    }

    command.args([title, body]);
    command.stdout(Stdio::piped());

    match command.spawn() {
        Ok(child) => {
            // Le o novo ID em segundo plano para nao bloquear o chamador;
            // o proximo aviso reutiliza esse ID para substituir este.
            thread::spawn(move || {
                if let Ok(output) = child.wait_with_output() {
                    let id = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !id.is_empty() {
                        store_last_id(&id);
                    }
                }
            });
        }
        Err(err) => warn!("failed to run notify-send: {err}"),
    }
}

/// `notify-send` e verificado uma unica vez por processo; sem ele, o OSD
/// fica desativado (com aviso no log).
pub fn is_available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        let available = crate::platform::find_command("notify-send").is_some();
        if !available {
            warn!("notify-send not found; desktop notifications disabled");
        }
        available
    })
}

fn last_id() -> Option<String> {
    LAST_ID.lock().ok().and_then(|guard| guard.clone())
}

fn store_last_id(id: &str) {
    if let Ok(mut guard) = LAST_ID.lock() {
        *guard = Some(id.to_string());
    }
}
