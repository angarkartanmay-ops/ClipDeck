// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Mutex, RwLock};

use crate::settings::Settings;
use crate::storage::Store;

/// Paste queue: while active, every new capture is appended, and the
/// dashboard hotkey pastes the oldest queued item instead of opening the UI.
#[derive(Default)]
pub struct Queue {
    pub active: bool,
    pub items: VecDeque<i64>,
}

pub struct AppState {
    pub store: Store,
    pub settings: RwLock<Settings>,
    pub data_dir: PathBuf,
    pub paused: AtomicBool,
    pub queue: Mutex<Queue>,
    /// The window that was in the foreground when the dashboard opened
    /// (raw HWND), i.e. where a chosen item will be pasted.
    pub target: Mutex<Option<isize>>,
    pub hotkey_errors: Mutex<Vec<String>>,
}

impl AppState {
    pub fn new(store: Store, settings: Settings, data_dir: PathBuf) -> Self {
        Self {
            store,
            settings: RwLock::new(settings),
            data_dir,
            paused: AtomicBool::new(false),
            queue: Mutex::new(Queue::default()),
            target: Mutex::new(None),
            hotkey_errors: Mutex::new(Vec::new()),
        }
    }

    pub fn settings(&self) -> Settings {
        self.settings.read().unwrap().clone()
    }

    pub fn forget_items(&self, ids: &[i64]) {
        if ids.is_empty() {
            return;
        }
        self.queue
            .lock()
            .unwrap()
            .items
            .retain(|id| !ids.contains(id));
    }
}
