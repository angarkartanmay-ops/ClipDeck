// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! Dashboard window on Linux. Desktop blur is not available to apps across
//! compositors, so the deck is an opaque near-black panel.

use tauri::window::Color;
use tauri::{
    AppHandle, PhysicalPosition, PhysicalSize, Theme, WebviewWindow, WebviewWindowBuilder, Wry,
};

use crate::settings::Settings;

pub fn configure<'a>(
    builder: WebviewWindowBuilder<'a, Wry, AppHandle>,
    _windowed: bool,
) -> WebviewWindowBuilder<'a, Wry, AppHandle> {
    builder
        .transparent(false)
        .background_color(Color(14, 14, 16, 255))
        .theme(Some(Theme::Dark))
}

pub fn style(_window: &WebviewWindow, _windowed: bool) {}

pub fn placement(
    app: &AppHandle,
    settings: &Settings,
) -> Option<(PhysicalPosition<i32>, PhysicalSize<u32>, f64)> {
    crate::dashboard::monitor_placement(app, settings)
}

pub fn glass_supported() -> bool {
    false
}
