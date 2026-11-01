// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! High-level operations shared by hotkeys, the tray menu and the dashboard.

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::settings::Settings;
use crate::state::AppState;
use crate::tray::{TrayItems, TRAY_ID};
use crate::{dashboard, platform};

fn state(app: &AppHandle) -> Arc<AppState> {
    app.state::<Arc<AppState>>().inner().clone()
}

/// A message for the dashboard banner (a missing permission, a policy).
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Notice {
    pub id: String,
    pub text: String,
    /// Passed to `open_system_settings` by the banner's button.
    pub action: Option<String>,
    pub action_label: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// "windows", "macos" or "linux".
    pub platform: &'static str,
    pub paused: bool,
    pub queue_active: bool,
    pub queue: Vec<i64>,
    pub hotkey_errors: Vec<String>,
    pub item_count: i64,
    pub disk_bytes: i64,
    /// Whether choosing an item will paste into a window (vs. only copy).
    pub can_paste: bool,
    /// Whether this platform can paste into other apps at all.
    pub can_auto_paste: bool,
    pub encryption_supported: bool,
    /// The dashboard window has a native blurred backdrop.
    pub glass: bool,
    /// Settings fixed by the administrator.
    pub managed: Vec<String>,
    pub notices: Vec<Notice>,
    pub version: String,
}

pub fn status(app: &AppHandle) -> Status {
    let state = state(app);
    let (queue_active, queue) = {
        let queue = state.queue.lock().unwrap();
        (queue.active, queue.items.iter().copied().collect())
    };
    let (item_count, disk_bytes) = state.store.stats();
    let hotkey_errors = state.hotkey_errors.lock().unwrap().clone();
    let has_target = state.target.lock().unwrap().is_some();
    let mut notices: Vec<Notice> = state
        .policy
        .notice
        .iter()
        .map(|text| Notice {
            id: "policy".into(),
            text: text.clone(),
            action: None,
            action_label: None,
        })
        .collect();
    notices.extend(platform::notices());
    let status = Status {
        platform: platform::NAME,
        paused: state.paused.load(Ordering::Relaxed),
        queue_active,
        queue,
        hotkey_errors,
        item_count,
        disk_bytes,
        can_paste: state.settings().paste_on_select && has_target && platform::can_auto_paste(),
        can_auto_paste: platform::can_auto_paste(),
        encryption_supported: platform::encryption_supported(),
        glass: platform::window::glass_supported(),
        managed: state.policy.managed_keys(),
        notices,
        version: app.package_info().version.to_string(),
    };
    status
}

pub fn emit_history(app: &AppHandle) {
    let _ = app.emit("history-changed", ());
}

pub fn emit_status(app: &AppHandle) {
    let status = status(app);
    if let Some(items) = app.try_state::<TrayItems>() {
        let _ = items.pause.set_checked(status.paused);
        let _ = items.queue.set_checked(status.queue_active);
    }
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let mut tip = format!("ClipDeck — {} items", status.item_count);
        if status.paused {
            tip.push_str(" (capture paused)");
        }
        if status.queue_active {
            tip.push_str(&format!(" · queue: {}", status.queue.len()));
        }
        let _ = tray.set_tooltip(Some(tip));
    }
    let _ = app.emit("status-changed", status);
}

pub fn set_paused(app: &AppHandle, paused: bool) {
    state(app).paused.store(paused, Ordering::Relaxed);
    emit_status(app);
}

pub fn set_queue_active(app: &AppHandle, active: bool) {
    {
        let state = state(app);
        let mut queue = state.queue.lock().unwrap();
        queue.active = active;
        if !active {
            queue.items.clear();
        }
    }
    emit_status(app);
}

pub fn toggle_queue(app: &AppHandle) {
    let active = state(app).queue.lock().unwrap().active;
    set_queue_active(app, !active);
}

/// Adds the item to the paste queue, or removes it if already queued.
pub fn toggle_queued(app: &AppHandle, id: i64) {
    {
        let state = state(app);
        let mut queue = state.queue.lock().unwrap();
        if let Some(pos) = queue.items.iter().position(|q| *q == id) {
            queue.items.remove(pos);
        } else {
            queue.active = true;
            queue.items.push_back(id);
        }
    }
    emit_status(app);
}

pub fn toggle_autostart(app: &AppHandle) {
    let state = state(app);
    let mut settings = state.settings();
    settings.autostart = !settings.autostart;
    let _ = apply_settings(app, settings);
}

pub fn open_data_dir(app: &AppHandle) {
    platform::open_path(&state(app).data_dir);
}

/// Dashboard hotkey: pastes the next queued item while the queue has
/// items, otherwise opens/closes the dashboard.
pub fn dashboard_hotkey(app: &AppHandle, target: Option<i64>) {
    let state = state(app);
    let next = {
        let mut queue = state.queue.lock().unwrap();
        if queue.active {
            queue.items.pop_front()
        } else {
            None
        }
    };
    match next {
        Some(id) if target.is_some() => {
            emit_status(app);
            if let Err(err) = paste_item(app, id, false, target, false) {
                eprintln!("ClipDeck: queue paste: {err}");
            }
        }
        Some(id) => {
            state.queue.lock().unwrap().items.push_front(id);
            dashboard::toggle(app, target, "");
        }
        None => dashboard::toggle(app, target, ""),
    }
}

/// Slot hotkey (e.g. Ctrl+Alt+N): pastes the N-th most recent item without
/// opening the UI.
pub fn paste_slot(app: &AppHandle, slot: usize, target: Option<i64>) {
    let ids = state(app).store.ids().unwrap_or_default();
    if let (Some(id), Some(_)) = (ids.get(slot - 1), target) {
        if let Err(err) = paste_item(app, *id, false, target, false) {
            eprintln!("ClipDeck: slot paste: {err}");
        }
    }
}

/// Puts an item on the clipboard and, when there is a target app and
/// pasting is enabled and possible, pastes it there. Blocks; call off the
/// main thread.
pub fn paste_item(
    app: &AppHandle,
    id: i64,
    plain: bool,
    target: Option<i64>,
    from_dashboard: bool,
) -> Result<(), String> {
    let state = state(app);
    let settings = state.settings();
    let mut data = state.store.load_data(id)?;
    if plain {
        if let Some(text) = data.plain_text_only() {
            data = text;
        }
    }
    let target = target.filter(|_| settings.paste_on_select && platform::can_auto_paste());
    let previous = match target {
        Some(_) if settings.restore_clipboard => platform::snapshot_clipboard(),
        _ => None,
    };
    platform::set_clipboard(&data)?;
    drop(data);

    if from_dashboard {
        dashboard::close(app);
    }
    if let Some(target) = target {
        let pasted = platform::paste_into(app, target);
        if let (true, Some(previous)) = (pasted, previous) {
            // Give the target app time to read the clipboard first.
            std::thread::sleep(Duration::from_millis(500));
            platform::set_clipboard(&previous)?;
        }
        if !pasted {
            // Still on the clipboard; tell the dashboard why it only copied.
            emit_status(app);
        }
    }
    Ok(())
}

/// Validates, saves and applies new settings. Returns what was stored.
pub fn apply_settings(app: &AppHandle, new: Settings) -> Result<Settings, String> {
    let state = state(app);
    // The administrator's values always win over the user's.
    let mut new = state.policy.apply(new.sanitized());
    let old = state.settings();
    // Window bounds are owned by the core (saved when the window closes);
    // the page's copy may be stale.
    new.window_bounds = old.window_bounds;
    if new == old {
        return Ok(new);
    }
    new.save(&state.data_dir)
        .map_err(|e| format!("save settings: {e}"))?;
    *state.settings.write().unwrap() = new.clone();

    if new.display_mode != old.display_mode {
        dashboard::apply_mode(app);
    }
    if new.hotkey_dashboard != old.hotkey_dashboard
        || new.hotkey_queue != old.hotkey_queue
        || new.slot_modifiers != old.slot_modifiers
    {
        platform::reload_hotkeys(app);
    }
    if new.autostart != old.autostart {
        if !cfg!(debug_assertions) {
            platform::set_autostart(new.autostart)?;
        }
        if let Some(items) = app.try_state::<TrayItems>() {
            let _ = items.autostart.set_checked(new.autostart);
        }
    }
    if new.encrypt != old.encrypt {
        let state = state.clone();
        let app = app.clone();
        let encrypt = new.encrypt;
        std::thread::spawn(move || {
            if let Err(err) = state.store.set_encryption(encrypt) {
                eprintln!("ClipDeck: encryption migration: {err}");
            }
            emit_history(&app);
        });
    }
    if new.history_limit < old.history_limit || new.max_disk_mb < old.max_disk_mb {
        let removed = state
            .store
            .enforce_limits(new.history_limit, new.max_disk_bytes())?;
        state.forget_items(&removed);
        emit_history(app);
    }
    if new.retention_days != old.retention_days && !state.enforce_retention().is_empty() {
        emit_history(app);
    }
    if !new.keep_dashboard_warm && old.keep_dashboard_warm {
        if let Some(window) = app.get_webview_window(dashboard::LABEL) {
            if !window.is_visible().unwrap_or(false) {
                let _ = window.destroy();
            }
        }
    }
    let _ = app.emit("settings-changed", new.clone());
    emit_status(app);
    Ok(new)
}

pub fn clear_if_configured(state: &AppState) {
    if state.settings().clear_on_exit {
        if let Ok(removed) = state.store.clear(true) {
            state.forget_items(&removed);
        }
    }
}

pub fn on_exit(app: &AppHandle) {
    clear_if_configured(&state(app));
}
