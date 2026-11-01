// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! ClipDeck: a lightweight clipboard manager for Windows, macOS and Linux.
//!
//! The process is split into an always-on core (clipboard watching, global
//! hotkeys, a capture worker, SQLite storage and the tray/menu-bar icon) and
//! an on-demand dashboard webview that is created when the hotkey is pressed
//! and destroyed when it closes. Operating-system specifics live in
//! `platform`.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod clip;
mod commands;
mod dashboard;
mod paths;
mod platform;
mod policy;
mod settings;
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
    // running instance to open its dashboard and exits. (On macOS and Linux
    // the single-instance plugin below does the same.)
    let Some(_instance_guard) = platform::acquire_single_instance() else {
        platform::signal_running_instance();
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
    let policy = policy::Policy::load();
    let state = Arc::new(AppState::new(store, settings, policy, data_dir));

    let builder = tauri::Builder::default();
    #[cfg(not(windows))]
    let builder = builder
        // Must be the first plugin. A second launch (also `clipdeck --toggle`,
        // which Wayland users bind to a system shortcut) toggles the deck.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            let app = app.clone();
            let target = platform::foreground_target();
            std::thread::spawn(move || dashboard::toggle(&app, target, ""));
        }))
        .plugin(platform::shortcuts::plugin());

    let app = builder
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
            commands::open_system_settings,
        ])
        .on_window_event(|window, event| {
            if window.label() == dashboard::LABEL {
                if let WindowEvent::Focused(focused) = event {
                    dashboard::on_focus_changed(window.app_handle(), *focused);
                }
            }
        })
        .setup(move |app| {
            // A menu-bar app on macOS: no Dock icon, no app switcher entry.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let handle = app.handle().clone();
            tray::create(&handle)?;
            let jobs = worker::spawn(handle.clone());
            platform::start(&handle, jobs);

            let settings = state.settings();
            if !cfg!(debug_assertions) {
                if let Err(err) = platform::set_autostart(settings.autostart) {
                    eprintln!("ClipDeck: autostart: {err}");
                }
            }
            state
                .store
                .enforce_limits(settings.history_limit, settings.max_disk_bytes())
                .ok();
            state.enforce_retention();
            // A policy can switch encryption on or off before the user ever
            // sees the setting; bring stored items in line (no-op if they are).
            {
                let state = state.clone();
                let encrypt = settings.encrypt;
                std::thread::spawn(move || {
                    if let Err(err) = state.store.set_encryption(encrypt) {
                        eprintln!("ClipDeck: encryption migration: {err}");
                    }
                });
            }

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
        // macOS doesn't start a second copy when ClipDeck.app is opened
        // again (Finder, Spotlight, Launchpad); toggle the deck the same way.
        #[cfg(target_os = "macos")]
        RunEvent::Reopen { .. } => {
            let app = app.clone();
            let target = platform::foreground_target();
            std::thread::spawn(move || dashboard::toggle(&app, target, ""));
        }
        _ => {}
    });
}
