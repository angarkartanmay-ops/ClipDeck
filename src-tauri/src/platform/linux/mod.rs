// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! Linux implementation of the platform layer (X11 and XWayland).
//!
//! Linux desktops do not let one app type into another in a portable way
//! (Wayland forbids it), so choosing a clip copies it and the user pastes
//! with Ctrl+V. There is no system key store ClipDeck can rely on, so
//! encryption at rest is not offered; the data folder is private to the
//! user (mode 0700).

mod autostart;
mod policy;
pub mod window;
mod x11;

use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Sender;

use tauri::AppHandle;

use crate::actions::Notice;
use crate::clip::ClipData;
use crate::worker::Job;

pub const NAME: &str = "linux";

pub use autostart::set as set_autostart;
pub use policy::read as read_policy;

/// The single-instance plugin handles this on Linux.
pub fn acquire_single_instance() -> Option<()> {
    Some(())
}

pub fn signal_running_instance() {}

pub fn start(app: &AppHandle, jobs: Sender<Job>) {
    x11::watch(app.clone(), jobs);
    reload_hotkeys(app);
}

pub fn reload_hotkeys(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || super::shortcuts::reload(&app));
}

/// Not needed: ClipDeck never pastes for the user on Linux.
pub fn snapshot_clipboard() -> Option<ClipData> {
    None
}

pub fn set_clipboard(data: &ClipData) -> Result<(), String> {
    x11::write(data)
}

/// There is no window to return to; queue and slot hotkeys still copy.
pub fn foreground_target() -> Option<i64> {
    Some(0)
}

pub fn paste_into(_app: &AppHandle, _target: i64) -> bool {
    false
}

pub fn can_auto_paste() -> bool {
    false
}

pub fn encryption_supported() -> bool {
    false
}

pub fn protect(_data: &[u8]) -> Result<Vec<u8>, String> {
    Err("encryption at rest is not available on Linux".into())
}

pub fn unprotect(_data: &[u8]) -> Result<Vec<u8>, String> {
    Err("encryption at rest is not available on Linux".into())
}

pub fn open_path(path: &Path) {
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}

pub fn open_system_settings(_kind: &str) {}

fn is_wayland() -> bool {
    std::env::var("XDG_SESSION_TYPE").is_ok_and(|v| v.eq_ignore_ascii_case("wayland"))
        || std::env::var_os("WAYLAND_DISPLAY").is_some()
}

pub fn notices() -> Vec<Notice> {
    let mut notices = Vec::new();
    if !x11::WATCHING.load(Ordering::Acquire) {
        notices.push(Notice {
            id: "x11".into(),
            text: "ClipDeck needs an X11 or XWayland display to watch the clipboard, \
                   and none is available, so new copies aren't recorded."
                .into(),
            action: None,
            action_label: None,
        });
    }
    if is_wayland() {
        notices.push(Notice {
            id: "wayland".into(),
            text: "On Wayland, global shortcuts don't reach apps. Add a keyboard shortcut \
                   in your desktop settings that runs: clipdeck --toggle"
                .into(),
            action: None,
            action_label: None,
        });
    }
    notices
}
