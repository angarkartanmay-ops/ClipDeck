// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! Capture worker: hashing, storage, thumbnails and eviction happen here so
//! the clipboard thread is never blocked by disk or image work.

use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;

use tauri::{AppHandle, Manager};

use crate::clip::{self, ClipData};
use crate::state::AppState;
use crate::storage::NewItem;
use crate::{actions, thumbs};

pub enum Job {
    Capture {
        data: ClipData,
        created_at: i64,
        source_exe: String,
        source_name: String,
    },
}

pub fn spawn(app: AppHandle) -> Sender<Job> {
    let (tx, rx) = channel::<Job>();
    std::thread::Builder::new()
        .name("clipdeck-worker".into())
        .spawn(move || {
            for job in rx {
                match job {
                    Job::Capture {
                        data,
                        created_at,
                        source_exe,
                        source_name,
                    } => {
                        if let Err(err) = capture(&app, data, created_at, &source_exe, &source_name)
                        {
                            eprintln!("ClipDeck: store capture: {err}");
                        }
                    }
                }
            }
        })
        .expect("spawn worker thread");
    tx
}

fn capture(
    app: &AppHandle,
    data: ClipData,
    created_at: i64,
    source_exe: &str,
    source_name: &str,
) -> Result<(), String> {
    let state = app.state::<Arc<AppState>>().inner().clone();
    let settings = state.settings();
    let hash = data.hash();

    // Apps often fire several clipboard updates for one copy; a repeat of
    // the newest item is ignored.
    if state.store.latest_hash().as_deref() == Some(hash.as_str()) {
        return Ok(());
    }

    // Copying something already in history moves it to the top (with the
    // newest formats) instead of creating a duplicate. Pins are preserved.
    let mut pinned = false;
    if let Some((old_id, was_pinned)) = state.store.find_by_hash(&hash) {
        pinned = was_pinned;
        state.store.delete(old_id)?;
        state.forget_items(&[old_id]);
    }

    let analysis = clip::analyze(&data);
    let item = NewItem {
        kind: analysis.kind,
        created_at,
        source_app: source_name,
        source_exe,
        pinned,
        hash: &hash,
        preview: &analysis.preview,
        meta: &analysis.meta,
    };
    let id = state.store.insert(&item, &data, settings.encrypt)?;
    if analysis.has_image {
        if let Some(png) = thumbs::make(&data) {
            state.store.save_thumb(id, &png)?;
        }
    }
    drop(data);

    {
        let mut queue = state.queue.lock().unwrap();
        if queue.active {
            queue.items.push_back(id);
        }
    }
    let removed = state
        .store
        .enforce_limits(settings.history_limit, settings.max_disk_bytes())?;
    state.forget_items(&removed);

    actions::emit_history(app);
    actions::emit_status(app);
    Ok(())
}
