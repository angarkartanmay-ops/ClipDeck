// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! Global hotkeys on macOS and Linux via the Tauri global-shortcut plugin
//! (Carbon `RegisterEventHotKey` on macOS, X11 key grabs on Linux; neither
//! needs an input-monitoring permission or a keyboard hook).

use std::str::FromStr;
use std::sync::{Arc, Mutex};

use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_global_shortcut::{
    Builder, GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState,
};

use crate::actions;
use crate::state::AppState;

#[derive(Clone, Copy)]
enum Action {
    Dashboard,
    Queue,
    Slot(usize),
}

/// Registered shortcut ids and what they do.
static REGISTERED: Mutex<Vec<(u32, Action)>> = Mutex::new(Vec::new());

pub fn plugin() -> TauriPlugin<Wry> {
    Builder::new().with_handler(handle).build()
}

fn handle(app: &AppHandle, shortcut: &Shortcut, event: ShortcutEvent) {
    if event.state != ShortcutState::Pressed {
        return;
    }
    let action = REGISTERED
        .lock()
        .unwrap()
        .iter()
        .find(|(id, _)| *id == shortcut.id())
        .map(|(_, action)| *action);
    let Some(action) = action else {
        return;
    };
    // Remember the app to paste into before the dashboard takes focus.
    let target = super::foreground_target();
    let app = app.clone();
    std::thread::spawn(move || match action {
        Action::Dashboard => actions::dashboard_hotkey(&app, target),
        Action::Queue => actions::toggle_queue(&app),
        Action::Slot(n) => actions::paste_slot(&app, n, target),
    });
}

/// Accepts the same spelling as the Windows parser ("Ctrl+Shift+V",
/// "Win"/"Cmd"/"Super", "Alt"/"Option").
fn parse(spec: &str) -> Option<Shortcut> {
    let normalized: Vec<String> = spec
        .split('+')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|t| match t.to_ascii_lowercase().as_str() {
            "win" | "meta" | "command" => "Super".to_string(),
            "option" => "Alt".to_string(),
            _ => t.to_string(),
        })
        .collect();
    if normalized.len() < 2 {
        return None; // a modifier is required
    }
    Shortcut::from_str(&normalized.join("+")).ok()
}

/// (Re-)registers every hotkey from the current settings. Must not run on
/// the main thread (registration itself is marshalled there).
pub fn reload(app: &AppHandle) {
    let state = app.state::<Arc<AppState>>().inner().clone();
    let settings = state.settings();
    let shortcuts = app.global_shortcut();
    let _ = shortcuts.unregister_all();

    let mut wanted: Vec<(String, Action, &str)> = vec![
        (
            settings.hotkey_dashboard.clone(),
            Action::Dashboard,
            "dashboard",
        ),
        (settings.hotkey_queue.clone(), Action::Queue, "paste queue"),
    ];
    if !settings.slot_modifiers.trim().is_empty() {
        for n in 1..=9 {
            wanted.push((
                format!("{}+{n}", settings.slot_modifiers),
                Action::Slot(n),
                "paste slot",
            ));
        }
    }

    let mut registered = Vec::new();
    let mut errors = Vec::new();
    for (spec, action, what) in wanted {
        if spec.trim().is_empty() {
            continue;
        }
        match parse(&spec) {
            Some(shortcut) => match shortcuts.register(shortcut) {
                Ok(()) => registered.push((shortcut.id(), action)),
                Err(_) => errors.push(format!("{spec} ({what}) is already used by another app")),
            },
            None => errors.push(format!("\"{spec}\" ({what}) is not a valid shortcut")),
        }
    }
    *REGISTERED.lock().unwrap() = registered;
    for err in &errors {
        eprintln!("ClipDeck: hotkey: {err}");
    }
    *state.hotkey_errors.lock().unwrap() = errors;
    actions::emit_status(app);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cross_platform_spellings() {
        assert!(parse("Ctrl+Cmd+V").is_some());
        assert!(parse("Ctrl+Alt+1").is_some());
        assert_eq!(parse("Win+V"), parse("Super+V"));
        assert_eq!(parse("Option+Cmd+B"), parse("Alt+Super+B"));
        assert!(parse("V").is_none());
    }
}
