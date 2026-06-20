use std::env;
use std::process::Command;

use which::which;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
    Apt,
    Dnf,
    Pacman,
    Zypper,
}

impl PackageManager {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Apt => "apt",
            Self::Dnf => "dnf",
            Self::Pacman => "pacman",
            Self::Zypper => "zypper",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlatformInfo {
    pub os: String,
    pub is_linux: bool,
    pub wayland_display: Option<String>,
    pub xdg_session_type: Option<String>,
    pub package_manager: Option<PackageManager>,
}

impl PlatformInfo {
    pub fn detect() -> Self {
        Self {
            os: env::consts::OS.to_string(),
            is_linux: env::consts::OS == "linux",
            wayland_display: env::var("WAYLAND_DISPLAY").ok().filter(|v| !v.is_empty()),
            xdg_session_type: env::var("XDG_SESSION_TYPE").ok().filter(|v| !v.is_empty()),
            package_manager: detect_package_manager(),
        }
    }

    pub fn is_wayland_session(&self) -> bool {
        self.wayland_display.is_some() || self.xdg_session_type.as_deref() == Some("wayland")
    }
}

pub fn find_command(name: &str) -> Option<String> {
    which(name)
        .ok()
        .map(|path| path.to_string_lossy().to_string())
}

pub fn has_command(name: &str) -> bool {
    find_command(name).is_some()
}

pub fn apt_package_installed(name: &str) -> bool {
    Command::new("dpkg")
        .args(["-s", name])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn detect_package_manager() -> Option<PackageManager> {
    if has_command("apt") {
        Some(PackageManager::Apt)
    } else if has_command("dnf") {
        Some(PackageManager::Dnf)
    } else if has_command("pacman") {
        Some(PackageManager::Pacman)
    } else if has_command("zypper") {
        Some(PackageManager::Zypper)
    } else {
        None
    }
}
