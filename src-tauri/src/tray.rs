// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

use crate::state::AppState;
use crate::{actions, dashboard};

pub const TRAY_ID: &str = "clipdeck";

/// Menu items whose check marks change at runtime.
pub struct TrayItems {
    pub pause: CheckMenuItem<Wry>,
    pub queue: CheckMenuItem<Wry>,
    pub autostart: CheckMenuItem<Wry>,
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let state = app.state::<Arc<AppState>>();
    let settings = state.settings();

    let open = MenuItem::with_id(app, "open", "Open ClipDeck", true, None::<&str>)?;
    let pause = CheckMenuItem::with_id(app, "pause", "Pause capture", true, false, None::<&str>)?;
    let queue =
        CheckMenuItem::with_id(app, "queue", "Paste queue mode", true, false, None::<&str>)?;
    let autostart = CheckMenuItem::with_id(
        app,
        "autostart",
        "Start with Windows",
        true,
        settings.autostart,
        None::<&str>,
    )?;
    let settings_item = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let folder = MenuItem::with_id(app, "folder", "Open data folder", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit ClipDeck", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &open,
            &PredefinedMenuItem::separator(app)?,
            &pause,
            &queue,
            &PredefinedMenuItem::separator(app)?,
            &autostart,
            &settings_item,
            &folder,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("ClipDeck")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => dashboard::toggle(app, None, ""),
            "settings" => dashboard::toggle(app, None, "settings"),
            "pause" => {
                let paused = app.state::<Arc<AppState>>().paused.load(Ordering::Relaxed);
                actions::set_paused(app, !paused);
            }
            "queue" => actions::toggle_queue(app),
            "autostart" => actions::toggle_autostart(app),
            "folder" => actions::open_data_dir(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                dashboard::toggle(tray.app_handle(), None, "");
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    app.manage(TrayItems {
        pause,
        queue,
        autostart,
    });
    Ok(())
}
