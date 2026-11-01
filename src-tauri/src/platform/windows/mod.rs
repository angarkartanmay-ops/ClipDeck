// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! Windows implementation of the platform layer.

mod autostart;
mod clipboard;
mod core_window;
mod crypto;
mod hotkeys;
mod paste;
mod policy;
mod source_app;
pub mod window;

use std::path::Path;
use std::sync::mpsc::Sender;

use tauri::AppHandle;

use crate::clip::ClipData;
use crate::worker::Job;

pub const NAME: &str = "windows";

pub use autostart::set as set_autostart;
pub use core_window::{acquire_single_instance, signal_running_instance};
pub use crypto::{protect, unprotect};
pub use policy::read as read_policy;

/// Starts the clipboard listener and hotkeys (the hidden core window).
pub fn start(app: &AppHandle, jobs: Sender<Job>) {
    core_window::spawn(app.clone(), jobs);
}

pub fn reload_hotkeys(_app: &AppHandle) {
    core_window::reload_hotkeys();
}

pub fn snapshot_clipboard() -> Option<ClipData> {
    paste::snapshot_clipboard()
}

pub fn set_clipboard(data: &ClipData) -> Result<(), String> {
    paste::set_clipboard(data)
}

/// The window to paste into (an HWND), unless ClipDeck itself is in front.
pub fn foreground_target() -> Option<i64> {
    paste::foreground_target().map(|hwnd| hwnd as i64)
}

/// Focuses `target` and sends Ctrl+V. Returns false if it could not be focused.
pub fn paste_into(_app: &AppHandle, target: i64) -> bool {
    if paste::focus(target as isize) {
        paste::send_ctrl_v();
        true
    } else {
        false
    }
}

/// Whether choosing an item can paste into the previous app (vs. only copy).
pub fn can_auto_paste() -> bool {
    true
}

pub fn encryption_supported() -> bool {
    true
}

pub fn open_path(path: &Path) {
    let _ = std::process::Command::new("explorer.exe").arg(path).spawn();
}

/// No permission pages to open on Windows.
pub fn open_system_settings(_kind: &str) {}

/// Platform notices for the dashboard banner: none needed on Windows.
pub fn notices() -> Vec<crate::actions::Notice> {
    Vec::new()
}
