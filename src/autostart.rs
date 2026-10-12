use std::fs;
use std::path::PathBuf;

use crate::error::AppResult;

pub fn autostart_desktop_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("autostart").join("linux-voice-typer.desktop"))
}

pub fn is_autostart_enabled() -> bool {
    autostart_desktop_path()
        .map(|p| p.exists())
        .unwrap_or(false)
}

pub fn set_autostart_enabled(enabled: bool) -> AppResult<()> {
    let Some(path) = autostart_desktop_path() else {
        return Ok(());
    };

    if enabled {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let current_exe =
            std::env::current_exe().unwrap_or_else(|_| PathBuf::from("linux-voice-typer-desktop"));

        let content = format!(
            "[Desktop Entry]\n\
            Type=Application\n\
            Name=Linux Voice Typer\n\
            Comment=Ditado de voz inteligente local com Whisper\n\
            Exec=\"{}\"\n\
            Terminal=false\n\
            Categories=Utility;Audio;Accessibility;\n\
            X-GNOME-Autostart-enabled=true\n",
            current_exe.display()
        );
        fs::write(&path, content)?;
    } else if path.exists() {
        let _ = fs::remove_file(&path);
    }

    Ok(())
}
