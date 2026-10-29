// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

use std::path::PathBuf;

/// `%APPDATA%\ClipDeck`, created if missing.
pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let dir = base.join("ClipDeck");
    let _ = std::fs::create_dir_all(&dir);
    dir
}
