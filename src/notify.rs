use std::process::Command;
use std::sync::OnceLock;

use log::warn;

/// Notificacoes de desktop (OSD) via `notify-send`.
///
/// Usa ID fixo (`-r 4242`) para que cada aviso substitua o anterior.
pub fn send(icon: &str, expire_ms: &str, title: &str, body: &str) {
    if !is_available() {
        return;
    }

    let _ = Command::new("notify-send")
        .args([
            "-r",
            "4242",
            "-a",
            "Linux Voice Typer",
            "-i",
            icon,
            "-e",
            "-t",
            expire_ms,
            title,
            body,
        ])
        .spawn();
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
