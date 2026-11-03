// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

use std::path::PathBuf;

/// Per-user data folder, created if missing and readable only by its owner:
///
/// - Windows: `%APPDATA%\ClipDeck`
/// - macOS: `~/Library/Application Support/ClipDeck`
/// - Linux: `$XDG_DATA_HOME/clipdeck` (default `~/.local/share/clipdeck`)
pub fn data_dir() -> PathBuf {
    let dir = base_dir();
    let _ = std::fs::create_dir_all(&dir);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
    }
    dir
}

#[cfg(windows)]
fn base_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("ClipDeck")
}

#[cfg(target_os = "macos")]
fn base_dir() -> PathBuf {
    home().join("Library/Application Support/ClipDeck")
}

#[cfg(not(any(windows, target_os = "macos")))]
fn base_dir() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| home().join(".local/share"))
        .join("clipdeck")
}

#[cfg(unix)]
pub fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}
