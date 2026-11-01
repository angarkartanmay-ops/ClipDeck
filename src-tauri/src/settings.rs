// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

use std::path::Path;

use serde::{Deserialize, Serialize};

const FILE_NAME: &str = "settings.json";

/// User settings, stored as JSON in the data directory. Unknown or missing
/// fields fall back to defaults so older files keep loading.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// Maximum number of unpinned items kept.
    pub history_limit: u32,
    /// Total disk budget for stored items, in MB.
    pub max_disk_mb: u64,
    /// Largest single item captured, in MB.
    pub max_item_mb: u64,

    pub hotkey_dashboard: String,
    pub hotkey_queue: String,
    /// Modifiers for the 1..9 slot hotkeys, e.g. "Ctrl+Alt". Empty disables them.
    pub slot_modifiers: String,

    /// Paste into the previous window after choosing an item (otherwise only copy).
    pub paste_on_select: bool,
    /// Put the previous clipboard content back after pasting.
    pub restore_clipboard: bool,
    /// "fullscreen" (overlay covering the monitor) or "window" (movable,
    /// resizable panel).
    pub display_mode: String,
    /// Last position and size of the window mode, in physical pixels.
    pub window_bounds: Option<WindowBounds>,
    /// Hide instead of destroying the dashboard: faster to open, more memory.
    pub keep_dashboard_warm: bool,

    /// "system", "on" or "off".
    pub reduce_motion: String,

    pub capture_images: bool,
    pub capture_files: bool,
    pub capture_rich: bool,

    /// Apps whose copies are never captured: executable names on Windows
    /// and Linux ("keepass.exe", "keepassxc"), bundle IDs on macOS.
    pub blocklist: Vec<String>,
    /// Encrypt stored history with the OS key store (DPAPI on Windows,
    /// Keychain on macOS).
    pub encrypt: bool,
    /// Delete clips older than this many days (0 keeps them until evicted).
    pub retention_days: u32,
    /// Remove unpinned history when ClipDeck exits or the user signs out.
    pub clear_on_exit: bool,
    pub autostart: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            history_limit: 50,
            max_disk_mb: 500,
            max_item_mb: 64,
            hotkey_dashboard: DEFAULT_HOTKEYS.0.into(),
            hotkey_queue: DEFAULT_HOTKEYS.1.into(),
            slot_modifiers: DEFAULT_HOTKEYS.2.into(),
            paste_on_select: true,
            restore_clipboard: false,
            display_mode: "fullscreen".into(),
            window_bounds: None,
            keep_dashboard_warm: false,
            reduce_motion: "system".into(),
            capture_images: true,
            capture_files: true,
            capture_rich: true,
            blocklist: default_blocklist(),
            encrypt: false,
            retention_days: 0,
            clear_on_exit: false,
            autostart: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WindowBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Default (dashboard, queue, slot modifiers) shortcuts per platform. They
/// avoid each OS's own bindings: Shift+Cmd+V is "Paste and Match Style" on
/// macOS and Ctrl+Shift+V pastes in Linux terminals.
#[cfg(windows)]
const DEFAULT_HOTKEYS: (&str, &str, &str) = ("Ctrl+Shift+V", "Ctrl+Shift+Q", "Ctrl+Alt");
#[cfg(target_os = "macos")]
const DEFAULT_HOTKEYS: (&str, &str, &str) = ("Ctrl+Cmd+V", "Ctrl+Cmd+B", "Ctrl+Cmd");
#[cfg(not(any(windows, target_os = "macos")))]
const DEFAULT_HOTKEYS: (&str, &str, &str) = ("Ctrl+Alt+V", "Ctrl+Alt+B", "");

/// Password managers on every platform: Windows executables, macOS bundle
/// IDs and Linux program names. Copies they mark as private are skipped
/// regardless; this also covers apps that don't mark them.
fn default_blocklist() -> Vec<String> {
    [
        // Windows
        "1password.exe",
        "bitwarden.exe",
        "dashlane.exe",
        "enpass.exe",
        "keepass.exe",
        "keepassxc.exe",
        "keeper.exe",
        "keeperpasswordmanager.exe",
        "lastpass.exe",
        "nordpass.exe",
        "proton pass.exe",
        "pwsafe.exe",
        "roboform.exe",
        // macOS
        "com.1password.1password",
        "com.agilebits.onepassword7",
        "com.bitwarden.desktop",
        "com.apple.passwords",
        "com.apple.keychainaccess",
        "org.keepassxc.keepassxc",
        "com.lastpass.lastpassmacdesktop",
        "com.dashlane.dashlanephonefinal",
        "in.sinew.enpass-desktop",
        "com.nordpass.macos.nordpass",
        "me.proton.pass.electron",
        // Linux
        "keepassxc",
        "bitwarden",
        "1password",
        "enpass",
        "seahorse",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

impl Settings {
    /// Loads settings, returning `(settings, first_run)`.
    pub fn load(dir: &Path) -> (Self, bool) {
        let path = dir.join(FILE_NAME);
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                // Editors such as Notepad and PowerShell may add a UTF-8 BOM.
                let text = text.trim_start_matches('\u{feff}');
                let settings = match serde_json::from_str::<Settings>(text) {
                    Ok(settings) => settings,
                    Err(err) => {
                        eprintln!(
                            "ClipDeck: {} is invalid ({err}); using defaults",
                            path.display()
                        );
                        Settings::default()
                    }
                };
                (settings.sanitized(), false)
            }
            Err(_) => {
                let settings = Settings::default();
                let _ = settings.save(dir);
                (settings, true)
            }
        }
    }

    pub fn save(&self, dir: &Path) -> std::io::Result<()> {
        let text = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        let tmp = dir.join(format!("{FILE_NAME}.tmp"));
        std::fs::write(&tmp, text)?;
        std::fs::rename(tmp, dir.join(FILE_NAME))
    }

    pub fn sanitized(mut self) -> Self {
        self.history_limit = self.history_limit.clamp(10, 200);
        self.max_disk_mb = self.max_disk_mb.clamp(50, 10_000);
        self.max_item_mb = self.max_item_mb.clamp(1, 512);
        self.retention_days = self.retention_days.min(3650);
        if !crate::platform::encryption_supported() {
            self.encrypt = false;
        }
        if !matches!(self.display_mode.as_str(), "fullscreen" | "window") {
            self.display_mode = "fullscreen".into();
        }
        if !matches!(self.reduce_motion.as_str(), "system" | "on" | "off") {
            self.reduce_motion = "system".into();
        }
        self.blocklist = self
            .blocklist
            .iter()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        self
    }

    pub fn is_window_mode(&self) -> bool {
        self.display_mode == "window"
    }

    /// Oldest creation time (ms since the epoch) clips may have, if limited.
    pub fn retention_cutoff(&self, now_ms: i64) -> Option<i64> {
        (self.retention_days > 0).then(|| now_ms - self.retention_days as i64 * 86_400_000)
    }

    pub fn max_disk_bytes(&self) -> u64 {
        self.max_disk_mb * 1024 * 1024
    }

    pub fn max_item_bytes(&self) -> usize {
        (self.max_item_mb * 1024 * 1024) as usize
    }

    pub fn is_blocked(&self, exe: &str) -> bool {
        let exe = exe.to_lowercase();
        !exe.is_empty() && self.blocklist.contains(&exe)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_file_with_bom() {
        let dir = std::env::temp_dir().join(format!("clipdeck-settings-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(FILE_NAME),
            "\u{feff}{\"autostart\": false, \"historyLimit\": 999}",
        )
        .unwrap();
        let (settings, first_run) = Settings::load(&dir);
        assert!(!first_run);
        assert!(!settings.autostart);
        assert_eq!(settings.history_limit, 200);
        let _ = std::fs::remove_dir_all(dir);
    }
}
