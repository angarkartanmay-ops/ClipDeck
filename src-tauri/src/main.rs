// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! ClipDeck: a lightweight clipboard manager for Windows.
//!
//! The process is split into an always-on core (a hidden Win32 window that
//! listens for clipboard changes and hotkeys, a capture worker, SQLite storage
//! and the tray icon) and an on-demand dashboard webview that is created when
//! the hotkey is pressed and destroyed when it closes.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod autostart;
mod clip;
mod commands;
mod core_window;
mod crypto;
mod dashboard;
mod hotkeys;
mod paste;
mod paths;
mod settings;
mod source_app;
mod state;
mod storage;
mod thumbs;
mod tray;
mod worker;

use std::sync::Arc;

use tauri::{Manager, RunEvent, WindowEvent};

use crate::state::AppState;

fn main() {
    // Only one ClipDeck may run per user session. A second launch asks the
    // running instance to open its dashboard and exits.
    let Some(_instance_guard) = core_window::acquire_single_instance() else {
        core_window::signal_running_instance();
        return;
    };

    let data_dir = paths::data_dir();
    let (settings, first_run) = settings::Settings::load(&data_dir);
    let store = match storage::Store::open(&data_dir) {
        Ok(store) => store,
        Err(err) => {
            eprintln!(
                "ClipDeck: cannot open storage in {}: {err}",
                data_dir.display()
            );
            std::process::exit(1);
        }
    };
    let state = Arc::new(AppState::new(store, settings, data_dir));

    let app = tauri::Builder::default()
        .manage(state.clone())
        .invoke_handler(tauri::generate_handler![
            commands::list_items,
            commands::get_thumb,
            commands::get_preview,
            commands::paste_item,
            commands::copy_item,
            commands::set_pinned,
            commands::delete_item,
            commands::clear_history,
            commands::get_settings,
            commands::save_settings,
            commands::get_status,
            commands::set_paused,
            commands::set_queue_active,
            commands::toggle_queued,
            commands::dashboard_ready,
            commands::close_dashboard,
            commands::open_data_dir,
        ])
        .on_window_event(|window, event| {
            if window.label() == dashboard::LABEL {
                if let WindowEvent::Focused(focused) = event {
                    dashboard::on_focus_changed(window.app_handle(), *focused);
                }
            }
        })
        .setup(move |app| {
            let handle = app.handle().clone();
            tray::create(&handle)?;
            let jobs = worker::spawn(handle.clone());
            core_window::spawn(handle.clone(), jobs);

            let settings = state.settings();
            if !cfg!(debug_assertions) {
                if let Err(err) = autostart::set(settings.autostart) {
                    eprintln!("ClipDeck: autostart: {err}");
                }
            }
            state
                .store
                .enforce_limits(settings.history_limit, settings.max_disk_bytes())
                .ok();

            let launched_in_background = std::env::args().any(|a| a == "--background");
            if first_run && !launched_in_background {
                dashboard::toggle(&handle, None, "welcome");
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building ClipDeck");

    app.run(|app, event| match event {
        // Closing the dashboard destroys the last window; keep the core alive.
        RunEvent::ExitRequested {
            code: None, api, ..
        } => api.prevent_exit(),
        RunEvent::Exit => actions::on_exit(app),
        _ => {}
    });
}
