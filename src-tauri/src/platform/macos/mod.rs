// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! macOS implementation of the platform layer.

mod autostart;
mod crypto;
mod paste;
mod pasteboard;
mod policy;
pub mod window;

use std::path::Path;
use std::sync::mpsc::Sender;

use tauri::AppHandle;

use crate::actions::Notice;
use crate::clip::{ClipData, ReadOptions, ReadOutcome};
use crate::worker::Job;

pub const NAME: &str = "macos";

pub use autostart::set as set_autostart;
pub use crypto::{protect, unprotect};
pub use paste::{can_auto_paste, foreground_target, paste_into};
pub use policy::read as read_policy;

/// The single-instance plugin handles this on macOS.
pub fn acquire_single_instance() -> Option<()> {
    Some(())
}

pub fn signal_running_instance() {}

/// Starts the pasteboard watcher and registers the hotkeys.
pub fn start(app: &AppHandle, jobs: Sender<Job>) {
    pasteboard::watch(app.clone(), jobs);
    reload_hotkeys(app);
}

pub fn reload_hotkeys(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || super::shortcuts::reload(&app));
}

pub fn snapshot_clipboard() -> Option<ClipData> {
    // Restoring is a nicety; never make macOS ask the user mid-paste.
    if pasteboard::access_restricted() {
        return None;
    }
    match pasteboard::read(&ReadOptions::everything()) {
        Ok(ReadOutcome::Data(data)) => Some(data),
        _ => None,
    }
}

pub fn set_clipboard(data: &ClipData) -> Result<(), String> {
    pasteboard::write(data)
}

pub fn encryption_supported() -> bool {
    true
}

pub fn open_path(path: &Path) {
    let _ = std::process::Command::new("open").arg(path).spawn();
}

/// Opens the Privacy & Security page a notice refers to.
pub fn open_system_settings(kind: &str) {
    let url = match kind {
        "accessibility" => {
            // Adds ClipDeck to the list so the user only has to switch it on.
            paste::request_accessibility();
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
        }
        "pasteboard" => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Pasteboard"
        }
        _ => return,
    };
    let _ = std::process::Command::new("open").arg(url).spawn();
}

pub fn notices() -> Vec<Notice> {
    let mut notices = Vec::new();
    if pasteboard::access_restricted() {
        notices.push(Notice {
            id: "pasteboard".into(),
            text:
                "macOS asks before ClipDeck can read the clipboard, so new copies aren't recorded. \
                   Set ClipDeck to Allow under Privacy & Security › Paste from Other Apps."
                    .into(),
            action: Some("pasteboard".into()),
            action_label: Some("Open Settings".into()),
        });
    }
    if !paste::can_auto_paste() {
        notices.push(Notice {
            id: "accessibility".into(),
            text: "To paste straight into apps, allow ClipDeck under Privacy & Security › \
                   Accessibility. Until then, choosing a clip copies it and you press ⌘V."
                .into(),
            action: Some("accessibility".into()),
            action_label: Some("Allow".into()),
        });
    }
    notices
}
