// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! Commands invoked by the dashboard frontend.

use std::sync::Arc;

use base64::Engine;
use tauri::{AppHandle, State};

use crate::actions::{self, Status};
use crate::dashboard;
use crate::settings::Settings;
use crate::state::AppState;
use crate::storage::ItemDto;

type AppStateRef<'a> = State<'a, Arc<AppState>>;

#[tauri::command]
pub async fn list_items(state: AppStateRef<'_>) -> Result<Vec<ItemDto>, String> {
    state.store.list()
}

#[tauri::command]
pub async fn get_thumb(state: AppStateRef<'_>, id: i64) -> Result<Option<String>, String> {
    Ok(state.store.load_thumb(id)?.map(|png| {
        format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(png)
        )
    }))
}

/// Full-resolution image for Quick Look.
#[tauri::command]
pub async fn get_preview(state: AppStateRef<'_>, id: i64) -> Result<Option<String>, String> {
    let data = state.store.load_data(id)?;
    Ok(crate::thumbs::preview(&data).map(|png| {
        format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(png)
        )
    }))
}

#[tauri::command]
pub fn paste_item(app: AppHandle, state: AppStateRef<'_>, id: i64, plain: bool) {
    let target = state.target.lock().unwrap().take();
    std::thread::spawn(move || {
        if let Err(err) = actions::paste_item(&app, id, plain, target, true) {
            eprintln!("ClipDeck: paste: {err}");
        }
    });
}

#[tauri::command]
pub fn copy_item(app: AppHandle, state: AppStateRef<'_>, id: i64) {
    state.target.lock().unwrap().take();
    std::thread::spawn(move || {
        if let Err(err) = actions::paste_item(&app, id, false, None, true) {
            eprintln!("ClipDeck: copy: {err}");
        }
    });
}

#[tauri::command]
pub async fn set_pinned(
    app: AppHandle,
    state: AppStateRef<'_>,
    id: i64,
    pinned: bool,
) -> Result<(), String> {
    state.store.set_pinned(id, pinned)?;
    actions::emit_history(&app);
    Ok(())
}

#[tauri::command]
pub async fn delete_item(app: AppHandle, state: AppStateRef<'_>, id: i64) -> Result<(), String> {
    state.store.delete(id)?;
    state.forget_items(&[id]);
    actions::emit_history(&app);
    actions::emit_status(&app);
    Ok(())
}

#[tauri::command]
pub async fn clear_history(
    app: AppHandle,
    state: AppStateRef<'_>,
    keep_pinned: bool,
) -> Result<(), String> {
    let removed = state.store.clear(keep_pinned)?;
    state.forget_items(&removed);
    actions::emit_history(&app);
    actions::emit_status(&app);
    Ok(())
}

#[tauri::command]
pub fn get_settings(state: AppStateRef<'_>) -> Settings {
    state.settings()
}

#[tauri::command]
pub async fn save_settings(app: AppHandle, settings: Settings) -> Result<Settings, String> {
    actions::apply_settings(&app, settings)
}

#[tauri::command]
pub fn get_status(app: AppHandle) -> Status {
    actions::status(&app)
}

#[tauri::command]
pub fn set_paused(app: AppHandle, paused: bool) {
    actions::set_paused(&app, paused);
}

#[tauri::command]
pub fn set_queue_active(app: AppHandle, active: bool) {
    actions::set_queue_active(&app, active);
}

#[tauri::command]
pub fn toggle_queued(app: AppHandle, id: i64) {
    actions::toggle_queued(&app, id);
}

#[tauri::command]
pub fn dashboard_ready(app: AppHandle) {
    dashboard::show(&app);
}

#[tauri::command]
pub fn close_dashboard(app: AppHandle) {
    dashboard::close(&app);
}

#[tauri::command]
pub fn open_data_dir(app: AppHandle) {
    actions::open_data_dir(&app);
}
