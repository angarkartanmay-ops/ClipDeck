// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! Operating-system integration. Each platform module provides the same
//! functions; the rest of ClipDeck is platform-neutral.
//!
//! | Function | Purpose |
//! | --- | --- |
//! | `NAME` | "windows", "macos" or "linux" |
//! | `acquire_single_instance`, `signal_running_instance` | one ClipDeck per user |
//! | `start`, `reload_hotkeys` | clipboard watching and global hotkeys |
//! | `snapshot_clipboard`, `set_clipboard` | reading/writing the clipboard |
//! | `foreground_target`, `paste_into`, `can_auto_paste` | pasting into the previous app |
//! | `set_autostart`, `open_path` | login item, file browser |
//! | `protect`, `unprotect`, `encryption_supported` | encryption at rest with the OS key store |
//! | `read_policy` | administrator-managed settings |
//! | `notices` | permission prompts for the dashboard banner |
//! | `window::{configure, style, placement, glass_supported}` | dashboard window |

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::*;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use self::macos::*;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use self::linux::*;

/// Global hotkeys through the Tauri plugin (macOS and Linux; Windows uses
/// RegisterHotKey on its core window).
#[cfg(not(windows))]
pub mod shortcuts;
