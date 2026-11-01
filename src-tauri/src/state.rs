// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, RwLock};

use crate::policy::Policy;
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
    /// Administrator policy, read once at startup.
    pub policy: Policy,
    pub data_dir: PathBuf,
    pub paused: AtomicBool,
    pub queue: Mutex<Queue>,
    /// Where a chosen item will be pasted: the app that was in front when
    /// the dashboard opened (an HWND on Windows, a process id on macOS).
    pub target: Mutex<Option<i64>>,
    pub hotkey_errors: Mutex<Vec<String>>,
}

impl AppState {
    pub fn new(store: Store, settings: Settings, policy: Policy, data_dir: PathBuf) -> Self {
        Self {
            store,
            settings: RwLock::new(policy.apply(settings)),
            policy,
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

    /// Whether new copies may be recorded right now (not paused by the user,
    /// not disabled by policy). Checked before the clipboard is even read.
    pub fn capture_allowed(&self) -> bool {
        !self.paused.load(Ordering::Relaxed) && !self.policy.capture_disabled
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

    /// Applies the retention period (and, when an administrator set it,
    /// applies it to pinned clips too). Returns the removed ids.
    pub fn enforce_retention(&self) -> Vec<i64> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or_default();
        let Some(cutoff) = self.settings().retention_cutoff(now) else {
            return Vec::new();
        };
        let removed = self
            .store
            .delete_older_than(cutoff, self.policy.manages("retentionDays"))
            .unwrap_or_default();
        self.forget_items(&removed);
        removed
    }
}
