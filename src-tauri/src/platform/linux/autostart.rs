// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! Start at login via an XDG autostart entry
//! (`~/.config/autostart/clipdeck.desktop`).

use std::path::PathBuf;

fn entry_path() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| crate::paths::home().join(".config"))
        .join("autostart/clipdeck.desktop")
}

pub fn set(enabled: bool) -> Result<(), String> {
    let path = entry_path();
    if !enabled {
        return match std::fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                Err(format!("remove autostart entry: {e}"))
            }
            _ => Ok(()),
        };
    }
    // An AppImage runs from a temporary mount; start the AppImage itself.
    let exe = std::env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .or_else(|| std::env::current_exe().ok())
        .ok_or("cannot find the ClipDeck executable")?;
    let exe = exe
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName=ClipDeck\nComment=Clipboard history\n\
         Exec=\"{exe}\" --background\nIcon=clipdeck\nTerminal=false\nNoDisplay=true\n\
         X-GNOME-Autostart-enabled=true\n"
    );
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("create autostart folder: {e}"))?;
    }
    std::fs::write(&path, entry).map_err(|e| format!("write autostart entry: {e}"))
}
