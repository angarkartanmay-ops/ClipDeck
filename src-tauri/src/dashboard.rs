// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! The on-demand dashboard window. It is created when opened and destroyed
//! when closed (unless "keep dashboard ready" is on), so the idle process
//! carries no webview.
//!
//! Two display modes: "fullscreen" covers the monitor under the cursor like
//! macOS Mission Control; "window" is a movable, resizable panel whose
//! bounds are remembered between openings. Styling and placement are
//! platform-specific (see `platform::window`).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, PhysicalSize, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder,
};

use crate::platform::window as platform_window;
use crate::settings::{Settings, WindowBounds};
use crate::state::AppState;

pub const LABEL: &str = "dashboard";
/// Default logical size of the window mode before the user resizes it.
pub const WINDOW_WIDTH: f64 = 980.0;
pub const WINDOW_HEIGHT: f64 = 640.0;
const MIN_WIDTH: f64 = 560.0;
const MIN_HEIGHT: f64 = 380.0;

/// Set once the dashboard has been shown, cleared when it closes.
static SHOWN: AtomicBool = AtomicBool::new(false);
/// True between the shown window gaining focus and being closed; a focus
/// loss in that span closes the dashboard (like the Windows clipboard flyout).
static ACTIVE: AtomicBool = AtomicBool::new(false);

fn settings(app: &AppHandle) -> Settings {
    app.state::<Arc<AppState>>().settings()
}

pub fn toggle(app: &AppHandle, target: Option<i64>, mode: &str) {
    let state = app.state::<Arc<AppState>>();
    if let Some(window) = app.get_webview_window(LABEL) {
        if window.is_visible().unwrap_or(false) {
            close(app);
            return;
        }
        *state.target.lock().unwrap() = target;
        // A warm (hidden) window: re-show it straight away.
        let _ = window.emit("dashboard-shown", mode);
        show(app);
        return;
    }
    *state.target.lock().unwrap() = target;
    // Apply the retention period before showing the history.
    state.enforce_retention();
    let windowed = state.settings().is_window_mode();
    let url = WebviewUrl::App(format!("index.html?mode={mode}").into());
    let builder = WebviewWindowBuilder::new(app, LABEL, url)
        .title("ClipDeck")
        .inner_size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .min_inner_size(MIN_WIDTH, MIN_HEIGHT)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(windowed)
        .maximizable(false)
        .minimizable(false)
        .visible(false)
        .focused(false);
    match platform_window::configure(builder, windowed).build() {
        Ok(window) => platform_window::style(&window, windowed),
        Err(err) => eprintln!("ClipDeck: cannot open dashboard: {err}"),
    }
    // The page calls `dashboard_ready` after its first render, which shows it.
}

/// Positions, shows and focuses the dashboard.
pub fn show(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LABEL) else {
        return;
    };
    place(app, &window, &settings(app));
    SHOWN.store(true, Ordering::SeqCst);
    let _ = window.show();
    let _ = window.set_focus();
}

/// Re-applies the display mode to an open dashboard (after a settings change).
pub fn apply_mode(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LABEL) else {
        return;
    };
    let settings = settings(app);
    let windowed = settings.is_window_mode();
    let _ = window.set_resizable(windowed);
    platform_window::style(&window, windowed);
    place(app, &window, &settings);
    let _ = window.set_focus();
}

pub fn close(app: &AppHandle) {
    SHOWN.store(false, Ordering::SeqCst);
    ACTIVE.store(false, Ordering::SeqCst);
    let Some(window) = app.get_webview_window(LABEL) else {
        return;
    };
    remember_bounds(app, &window);
    if settings(app).keep_dashboard_warm {
        let _ = window.hide();
        let _ = window.emit("dashboard-hidden", ());
    } else {
        let _ = window.destroy();
    }
}

/// Stores where the user left the window-mode panel.
fn remember_bounds(app: &AppHandle, window: &WebviewWindow) {
    let state = app.state::<Arc<AppState>>();
    if !state.settings().is_window_mode() || !window.is_visible().unwrap_or(false) {
        return;
    }
    let (Ok(pos), Ok(size)) = (window.outer_position(), window.outer_size()) else {
        return;
    };
    let bounds = WindowBounds {
        x: pos.x,
        y: pos.y,
        width: size.width,
        height: size.height,
    };
    let mut settings = state.settings.write().unwrap();
    if settings.window_bounds != Some(bounds) {
        settings.window_bounds = Some(bounds);
        let _ = settings.save(&state.data_dir);
    }
}

pub fn on_focus_changed(app: &AppHandle, focused: bool) {
    if focused {
        // Focus events can arrive while the window is still hidden and loading.
        if SHOWN.load(Ordering::SeqCst) {
            ACTIVE.store(true, Ordering::SeqCst);
        }
    } else if ACTIVE.swap(false, Ordering::SeqCst) {
        let app = app.clone();
        std::thread::spawn(move || close(&app));
    }
}

/// Sizes and positions the window for the current display mode.
fn place(app: &AppHandle, window: &WebviewWindow, settings: &Settings) {
    let Some((pos, size, scale)) = platform_window::placement(app, settings) else {
        return;
    };
    // Move first so a DPI change between monitors cannot rescale the size.
    let _ = window.set_position(pos);
    let _ = window.set_size(size);
    let _ = window.set_min_size(Some(LogicalSize::new(
        MIN_WIDTH.min(size.width as f64 / scale),
        MIN_HEIGHT.min(size.height as f64 / scale),
    )));
}

/// Placement from Tauri's monitor API (macOS and Linux): the monitor under
/// the cursor; its full frame for the overlay, or the remembered bounds
/// (when still on a connected monitor) for the window.
#[cfg_attr(windows, allow(dead_code))]
pub fn monitor_placement(
    app: &AppHandle,
    settings: &Settings,
) -> Option<(PhysicalPosition<i32>, PhysicalSize<u32>, f64)> {
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|c| app.monitor_from_point(c.x, c.y).ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten())?;
    let scale = monitor.scale_factor();
    if !settings.is_window_mode() {
        return Some((*monitor.position(), *monitor.size(), scale));
    }
    if let Some(b) = settings.window_bounds {
        let (cx, cy) = (b.x + b.width as i32 / 2, b.y + b.height as i32 / 2);
        let on_screen = app
            .monitor_from_point(cx as f64, cy as f64)
            .ok()
            .flatten()
            .is_some();
        if on_screen {
            return Some((
                PhysicalPosition::new(b.x, b.y),
                PhysicalSize::new(b.width, b.height),
                scale,
            ));
        }
    }
    let work = monitor.work_area();
    let w = ((WINDOW_WIDTH * scale).round() as u32).min(work.size.width);
    let h = ((WINDOW_HEIGHT * scale).round() as u32).min(work.size.height);
    Some((
        PhysicalPosition::new(
            work.position.x + (work.size.width as i32 - w as i32) / 2,
            work.position.y + (work.size.height as i32 - h as i32) / 2,
        ),
        PhysicalSize::new(w, h),
        scale,
    ))
}
