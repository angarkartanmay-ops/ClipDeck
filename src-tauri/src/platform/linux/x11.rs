// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! The X11 clipboard (also XWayland). Changes arrive as XFixes selection
//! events, so there is no polling. Before any content is read, the offered
//! targets are checked for the password-manager hint KeePassXC and KDE apps
//! set (`x-kde-passwordManagerHint`).

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager};
use x11_clipboard::Clipboard;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _};

use crate::clip::{ClipData, ClipFormat, ReadOptions, ReadOutcome, X_FILES, X_HTML, X_PNG, X_TEXT};
use crate::state::AppState;
use crate::worker::Job;

/// Set once the watcher has a display connection.
pub static WATCHING: AtomicBool = AtomicBool::new(false);

const LOAD_TIMEOUT: Duration = Duration::from_millis(1500);
const PASSWORD_HINT: &str = "x-kde-passwordManagerHint";

/// A separate connection that owns the clipboard when ClipDeck writes it.
fn writer() -> Option<&'static Clipboard> {
    static WRITER: OnceLock<Option<Clipboard>> = OnceLock::new();
    WRITER.get_or_init(|| Clipboard::new().ok()).as_ref()
}

fn atom_names(clip: &Clipboard, raw: &[u8]) -> HashMap<String, u32> {
    raw.chunks_exact(4)
        .map(|c| u32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
        .filter_map(|atom| {
            let reply = clip
                .getter
                .connection
                .get_atom_name(atom)
                .ok()?
                .reply()
                .ok()?;
            Some((String::from_utf8_lossy(&reply.name).into_owned(), atom))
        })
        .collect()
}

fn owner(clip: &Clipboard) -> Option<u32> {
    clip.getter
        .connection
        .get_selection_owner(clip.getter.atoms.clipboard)
        .ok()?
        .reply()
        .ok()
        .map(|r| r.owner)
}

/// Program name of the focused window's process (`_NET_ACTIVE_WINDOW` →
/// `_NET_WM_PID` → `/proc/<pid>/comm`), for the blocklist and labels.
fn active_program(clip: &Clipboard) -> Option<String> {
    let conn = &clip.getter.connection;
    let root = conn.setup().roots.get(clip.getter.screen)?.root;
    let active = clip.getter.get_atom("_NET_ACTIVE_WINDOW").ok()?;
    let pid_atom = clip.getter.get_atom("_NET_WM_PID").ok()?;
    let window = conn
        .get_property(false, root, active, AtomEnum::WINDOW, 0, 1)
        .ok()?
        .reply()
        .ok()?
        .value32()?
        .next()?;
    let pid = conn
        .get_property(false, window, pid_atom, AtomEnum::CARDINAL, 0, 1)
        .ok()?
        .reply()
        .ok()?
        .value32()?
        .next()?;
    let comm = std::fs::read_to_string(format!("/proc/{pid}/comm")).ok()?;
    Some(comm.trim().to_string())
}

fn read_targets(clip: &Clipboard, names: &HashMap<String, u32>, opts: &ReadOptions) -> ReadOutcome {
    if names.contains_key(PASSWORD_HINT) {
        return ReadOutcome::Excluded;
    }
    let atoms = &clip.getter.atoms;
    let mut budget = opts.max_bytes;
    let mut formats = Vec::new();
    let mut push = |stored_as: &str, offered: &[&str], formats: &mut Vec<ClipFormat>| {
        let Some(atom) = offered.iter().find_map(|n| names.get(*n)) else {
            return;
        };
        let Ok(data) = clip.load(atoms.clipboard, *atom, atoms.property, LOAD_TIMEOUT) else {
            return;
        };
        if data.is_empty() || data.len() > budget {
            return;
        }
        budget = budget.saturating_sub(data.len());
        formats.push(ClipFormat {
            name: stored_as.into(),
            data,
        });
    };

    if names.contains_key(X_FILES) {
        if !opts.files {
            return ReadOutcome::Empty;
        }
        push(X_FILES, &[X_FILES], &mut formats);
    } else {
        push(X_TEXT, &[X_TEXT, "text/plain;charset=utf-8"], &mut formats);
        if opts.rich {
            push(X_HTML, &[X_HTML], &mut formats);
        }
        if opts.images {
            push(X_PNG, &[X_PNG], &mut formats);
        }
    }
    let has_text = formats
        .iter()
        .any(|f| f.name == X_TEXT && !String::from_utf8_lossy(&f.data).trim().is_empty());
    let has_payload = has_text
        || formats
            .iter()
            .any(|f| matches!(f.name.as_str(), X_PNG | X_FILES));
    if has_payload {
        ReadOutcome::Data(ClipData { formats })
    } else {
        ReadOutcome::Empty
    }
}

/// Starts the clipboard watcher on its own thread.
pub fn watch(app: AppHandle, jobs: Sender<Job>) {
    std::thread::Builder::new()
        .name("clipdeck-x11".into())
        .spawn(move || {
            let Ok(clip) = Clipboard::new() else {
                eprintln!("ClipDeck: no X11 display; clipboard capture is off");
                return;
            };
            WATCHING.store(true, Ordering::Release);
            let atoms = (
                clip.getter.atoms.clipboard,
                clip.getter.atoms.targets,
                clip.getter.atoms.property,
            );
            loop {
                let targets = match clip.load_wait(atoms.0, atoms.1, atoms.2) {
                    Ok(targets) => targets,
                    Err(_) => {
                        std::thread::sleep(Duration::from_millis(500));
                        continue;
                    }
                };
                let ours = writer().map(|w| w.setter.window);
                if ours.is_some() && owner(&clip) == ours {
                    continue; // ClipDeck's own write
                }
                let state = app.state::<Arc<AppState>>().inner().clone();
                if !state.capture_allowed() {
                    continue;
                }
                let program = active_program(&clip).unwrap_or_default();
                let settings = state.settings();
                if settings.is_blocked(&program) {
                    continue;
                }
                let opts = ReadOptions {
                    images: settings.capture_images,
                    files: settings.capture_files,
                    rich: settings.capture_rich,
                    max_bytes: settings.max_item_bytes(),
                };
                let names = atom_names(&clip, &targets);
                if let ReadOutcome::Data(data) = read_targets(&clip, &names, &opts) {
                    let created_at = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_millis() as i64)
                        .unwrap_or_default();
                    let _ = jobs.send(Job::Capture {
                        data,
                        created_at,
                        source_exe: program.to_lowercase(),
                        source_name: program,
                    });
                }
            }
        })
        .expect("spawn X11 clipboard watcher");
}

/// Puts an item on the clipboard. X11 serves one representation per
/// selection here: files, else image, else text.
pub fn write(data: &ClipData) -> Result<(), String> {
    let clip = writer().ok_or("no X11 display")?;
    let (target, bytes) = if let Some(files) = data.get(X_FILES) {
        (X_FILES, files.to_vec())
    } else if let Some(png) = data.png() {
        (X_PNG, png.to_vec())
    } else if let Some(text) = data.text() {
        (X_TEXT, text.into_bytes())
    } else {
        return Err("nothing to paste".into());
    };
    let atom = clip
        .getter
        .get_atom(target)
        .map_err(|e| format!("X11 atom: {e}"))?;
    clip.store(clip.getter.atoms.clipboard, atom, bytes)
        .map_err(|e| format!("X11 clipboard: {e}"))
}
