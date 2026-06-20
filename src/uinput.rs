use std::fs::{File, OpenOptions};
use std::io::Write;
use std::mem::size_of;
use std::os::fd::AsRawFd;
use std::thread;
use std::time::Duration;

use crate::config::PasteShortcut;
use crate::error::{AppError, AppResult};

const UINPUT_PATH: &str = "/dev/uinput";

const UI_DEV_CREATE: libc::c_ulong = 0x5501;
const UI_DEV_DESTROY: libc::c_ulong = 0x5502;
const UI_DEV_SETUP: libc::c_ulong = 0x405c5503;
const UI_SET_EVBIT: libc::c_ulong = 0x40045564;
const UI_SET_KEYBIT: libc::c_ulong = 0x40045565;

const BUS_USB: u16 = 0x03;
const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const SYN_REPORT: u16 = 0;
const KEY_LEFTCTRL: u16 = 29;
const KEY_LEFTSHIFT: u16 = 42;
const KEY_V: u16 = 47;

#[repr(C)]
#[derive(Clone, Copy)]
struct InputId {
    bustype: u16,
    vendor: u16,
    product: u16,
    version: u16,
}

#[repr(C)]
struct UinputSetup {
    id: InputId,
    name: [u8; 80],
    ff_effects_max: u32,
}

#[repr(C)]
struct InputEvent {
    time: libc::timeval,
    type_: u16,
    code: u16,
    value: i32,
}

struct VirtualKeyboard {
    file: File,
}

impl VirtualKeyboard {
    fn create() -> AppResult<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(UINPUT_PATH)
            .map_err(|err| {
                AppError::Unsupported(format!(
                    "failed to open {UINPUT_PATH}: {err}. Try: sudo modprobe uinput and grant access to /dev/uinput"
                ))
            })?;

        ioctl_int(&file, UI_SET_EVBIT, EV_KEY)?;
        ioctl_int(&file, UI_SET_KEYBIT, KEY_LEFTCTRL)?;
        ioctl_int(&file, UI_SET_KEYBIT, KEY_LEFTSHIFT)?;
        ioctl_int(&file, UI_SET_KEYBIT, KEY_V)?;

        let mut setup = UinputSetup {
            id: InputId {
                bustype: BUS_USB,
                vendor: 0x1209,
                product: 0x7674,
                version: 1,
            },
            name: [0; 80],
            ff_effects_max: 0,
        };

        let name = b"linux-voice-typer virtual keyboard";
        setup.name[..name.len()].copy_from_slice(name);

        ioctl_ptr(&file, UI_DEV_SETUP, &setup)?;
        ioctl_no_arg(&file, UI_DEV_CREATE)?;
        thread::sleep(Duration::from_millis(120));

        Ok(Self { file })
    }

    fn send_shortcut(&mut self, shortcut: PasteShortcut) -> AppResult<()> {
        match shortcut {
            PasteShortcut::CtrlV => {
                self.key(KEY_LEFTCTRL, true)?;
                self.key(KEY_V, true)?;
                self.key(KEY_V, false)?;
                self.key(KEY_LEFTCTRL, false)?;
            }
            PasteShortcut::CtrlShiftV => {
                self.key(KEY_LEFTCTRL, true)?;
                self.key(KEY_LEFTSHIFT, true)?;
                self.key(KEY_V, true)?;
                self.key(KEY_V, false)?;
                self.key(KEY_LEFTSHIFT, false)?;
                self.key(KEY_LEFTCTRL, false)?;
            }
        }

        self.file.flush()?;
        thread::sleep(Duration::from_millis(50));
        Ok(())
    }

    fn key(&mut self, code: u16, pressed: bool) -> AppResult<()> {
        self.event(EV_KEY, code, i32::from(pressed))?;
        self.event(EV_SYN, SYN_REPORT, 0)
    }

    fn event(&mut self, type_: u16, code: u16, value: i32) -> AppResult<()> {
        let event = InputEvent {
            time: libc::timeval {
                tv_sec: 0,
                tv_usec: 0,
            },
            type_,
            code,
            value,
        };

        let bytes = unsafe {
            std::slice::from_raw_parts(
                (&event as *const InputEvent).cast::<u8>(),
                size_of::<InputEvent>(),
            )
        };
        self.file.write_all(bytes)?;
        Ok(())
    }
}

impl Drop for VirtualKeyboard {
    fn drop(&mut self) {
        let _ = unsafe { libc::ioctl(self.file.as_raw_fd(), UI_DEV_DESTROY) };
    }
}

pub fn send_paste_shortcut(shortcut: PasteShortcut) -> AppResult<()> {
    let mut keyboard = VirtualKeyboard::create()?;
    keyboard.send_shortcut(shortcut)
}

pub fn probe_virtual_keyboard() -> AppResult<()> {
    let _keyboard = VirtualKeyboard::create()?;
    Ok(())
}

fn ioctl_int(file: &File, request: libc::c_ulong, value: u16) -> AppResult<()> {
    let result = unsafe { libc::ioctl(file.as_raw_fd(), request, i32::from(value)) };
    if result == -1 {
        Err(AppError::Unsupported(format!(
            "uinput ioctl failed: {}",
            std::io::Error::last_os_error()
        )))
    } else {
        Ok(())
    }
}

fn ioctl_ptr<T>(file: &File, request: libc::c_ulong, value: &T) -> AppResult<()> {
    let result = unsafe { libc::ioctl(file.as_raw_fd(), request, value as *const T) };
    if result == -1 {
        Err(AppError::Unsupported(format!(
            "uinput setup failed: {}",
            std::io::Error::last_os_error()
        )))
    } else {
        Ok(())
    }
}

fn ioctl_no_arg(file: &File, request: libc::c_ulong) -> AppResult<()> {
    let result = unsafe { libc::ioctl(file.as_raw_fd(), request) };
    if result == -1 {
        Err(AppError::Unsupported(format!(
            "uinput device create/destroy failed: {}",
            std::io::Error::last_os_error()
        )))
    } else {
        Ok(())
    }
}
