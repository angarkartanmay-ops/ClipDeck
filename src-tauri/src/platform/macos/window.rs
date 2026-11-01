// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! Dashboard window styling on macOS: HUD vibrancy (a dark, translucent
//! blur of what is behind), shown on every Space and over full-screen apps;
//! the full-screen mode floats above the menu bar like Mission Control.

use objc2_app_kit::{
    NSFloatingWindowLevel, NSStatusWindowLevel, NSWindow, NSWindowCollectionBehavior,
};
use tauri::window::{Effect, EffectState, EffectsBuilder};
use tauri::{
    AppHandle, PhysicalPosition, PhysicalSize, Theme, WebviewWindow, WebviewWindowBuilder, Wry,
};

use crate::settings::Settings;

/// Corner radius of the window mode (matches the CSS).
const RADIUS: f64 = 14.0;

fn effects(windowed: bool) -> tauri::utils::config::WindowEffectsConfig {
    EffectsBuilder::new()
        .effect(Effect::HudWindow)
        .state(EffectState::Active)
        .radius(if windowed { RADIUS } else { 0.0 })
        .build()
}

pub fn configure<'a>(
    builder: WebviewWindowBuilder<'a, Wry, AppHandle>,
    windowed: bool,
) -> WebviewWindowBuilder<'a, Wry, AppHandle> {
    builder
        .transparent(true)
        .shadow(windowed)
        .effects(effects(windowed))
        .theme(Some(Theme::Dark))
        .visible_on_all_workspaces(true)
}

pub fn style(window: &WebviewWindow, windowed: bool) {
    let _ = window.set_shadow(windowed);
    let _ = window.set_effects(effects(windowed));
    let Ok(ns_window) = window.ns_window() else {
        return;
    };
    let ns_window = ns_window as usize;
    let _ = window.run_on_main_thread(move || {
        // SAFETY: the pointer is the live NSWindow of this webview window,
        // used on the main thread as AppKit requires.
        let ns_window = unsafe { &*(ns_window as *const NSWindow) };
        ns_window.setLevel(if windowed {
            NSFloatingWindowLevel
        } else {
            NSStatusWindowLevel
        });
        ns_window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
    });
}

pub fn placement(
    app: &AppHandle,
    settings: &Settings,
) -> Option<(PhysicalPosition<i32>, PhysicalSize<u32>, f64)> {
    crate::dashboard::monitor_placement(app, settings)
}

pub fn glass_supported() -> bool {
    true
}
