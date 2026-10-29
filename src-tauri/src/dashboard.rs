// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! The on-demand dashboard window. It is created when opened and destroyed
//! when closed (unless "keep dashboard ready" is on), so the idle process
//! carries no webview.
//!
//! Two display modes: "fullscreen" covers the monitor under the cursor like
//! macOS Mission Control; "window" is a movable, resizable panel whose
//! bounds are remembered between openings.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::window::{Color, Effect, EffectsBuilder};
use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, PhysicalSize, Theme, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder,
};
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND, DWMWCP_ROUND,
    DWM_WINDOW_CORNER_PREFERENCE,
};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MonitorFromRect, HMONITOR, MONITORINFO,
    MONITOR_DEFAULTTONEAREST, MONITOR_DEFAULTTONULL,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

use crate::settings::{Settings, WindowBounds};
use crate::state::AppState;

pub const LABEL: &str = "dashboard";
/// Default logical size of the window mode before the user resizes it.
const WINDOW_WIDTH: f64 = 980.0;
const WINDOW_HEIGHT: f64 = 640.0;
const MIN_WIDTH: f64 = 560.0;
const MIN_HEIGHT: f64 = 380.0;

/// Trims WebView2 helper processes and background work to keep the open
/// dashboard light. The first three features are Tauri's own defaults.
const BROWSER_ARGS: &str = concat!(
    "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection,SpareRendererForSitePerProcess ",
    "--renderer-process-limit=1 --disable-background-networking --disable-component-update ",
    "--disable-extensions --disable-sync --no-pings"
);

/// Set once the dashboard has been shown, cleared when it closes.
static SHOWN: AtomicBool = AtomicBool::new(false);
/// True between the shown window gaining focus and being closed; a focus
/// loss in that span closes the dashboard (like the Windows clipboard flyout).
static ACTIVE: AtomicBool = AtomicBool::new(false);

fn settings(app: &AppHandle) -> Settings {
    app.state::<Arc<AppState>>().settings()
}

pub fn toggle(app: &AppHandle, target: Option<isize>, mode: &str) {
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
    let windowed = state.settings().is_window_mode();
    let url = WebviewUrl::App(format!("index.html?mode={mode}").into());
    let built = WebviewWindowBuilder::new(app, LABEL, url)
        .title("ClipDeck")
        .inner_size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .min_inner_size(MIN_WIDTH, MIN_HEIGHT)
        .decorations(false)
        .transparent(true)
        .shadow(windowed)
        // Native acrylic: blurs whatever is behind the window. The tint
        // colour is only used by Windows 10's acrylic implementation.
        .effects(
            EffectsBuilder::new()
                .effect(Effect::Acrylic)
                .color(Color(8, 8, 10, 150))
                .build(),
        )
        // Monochrome black glass in every system theme.
        .theme(Some(Theme::Dark))
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(windowed)
        .maximizable(false)
        .minimizable(false)
        .visible(false)
        .focused(false)
        .additional_browser_args(BROWSER_ARGS)
        .build();
    match built {
        Ok(window) => set_corners(&window, windowed),
        Err(err) => eprintln!("ClipDeck: cannot open dashboard: {err}"),
    }
    // The page calls `dashboard_ready` after its first render, which shows it.
}

/// Positions, shows and focuses the dashboard.
pub fn show(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LABEL) else {
        return;
    };
    place(&window, &settings(app));
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
    let _ = window.set_shadow(windowed);
    set_corners(&window, windowed);
    place(&window, &settings);
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

/// Square corners for the full-screen overlay, rounded for the window (Windows 11).
fn set_corners(window: &WebviewWindow, round: bool) {
    let Ok(hwnd) = window.hwnd() else { return };
    let preference: DWM_WINDOW_CORNER_PREFERENCE = if round {
        DWMWCP_ROUND
    } else {
        DWMWCP_DONOTROUND
    };
    unsafe {
        let _ = DwmSetWindowAttribute(
            HWND(hwnd.0 as _),
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &preference as *const _ as *const _,
            std::mem::size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
        );
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

fn monitor_info(monitor: HMONITOR) -> Option<MONITORINFO> {
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    unsafe { GetMonitorInfoW(monitor, &mut info).as_bool() }.then_some(info)
}

fn cursor_monitor() -> Option<HMONITOR> {
    let mut cursor = POINT::default();
    unsafe {
        GetCursorPos(&mut cursor).ok()?;
        Some(MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST))
    }
}

fn scale_of(monitor: HMONITOR) -> f64 {
    let (mut dpi_x, mut dpi_y) = (96u32, 96u32);
    let _ = unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) };
    dpi_x as f64 / 96.0
}

/// Sizes and positions the window for the current display mode.
fn place(window: &WebviewWindow, settings: &Settings) {
    let Some(monitor) = cursor_monitor() else {
        return;
    };
    let Some(info) = monitor_info(monitor) else {
        return;
    };
    let (pos, size) = if settings.is_window_mode() {
        window_rect(settings.window_bounds, monitor, &info)
    } else {
        // The whole monitor, taskbar included, like Mission Control.
        let r = info.rcMonitor;
        (
            PhysicalPosition::new(r.left, r.top),
            PhysicalSize::new((r.right - r.left) as u32, (r.bottom - r.top) as u32),
        )
    };
    // Move first so a DPI change between monitors cannot rescale the size.
    let _ = window.set_position(pos);
    let _ = window.set_size(size);
    let scale = scale_of(monitor);
    let _ = window.set_min_size(Some(LogicalSize::new(
        MIN_WIDTH.min(size.width as f64 / scale),
        MIN_HEIGHT.min(size.height as f64 / scale),
    )));
}

/// The remembered window bounds if they are still on a connected monitor,
/// otherwise a default-sized window centered on the cursor's monitor.
fn window_rect(
    saved: Option<WindowBounds>,
    monitor: HMONITOR,
    info: &MONITORINFO,
) -> (PhysicalPosition<i32>, PhysicalSize<u32>) {
    if let Some(b) = saved {
        let rect = RECT {
            left: b.x,
            top: b.y,
            right: b.x + b.width as i32,
            bottom: b.y + b.height as i32,
        };
        if !unsafe { MonitorFromRect(&rect, MONITOR_DEFAULTTONULL) }.is_invalid() {
            return (
                PhysicalPosition::new(b.x, b.y),
                PhysicalSize::new(b.width, b.height),
            );
        }
    }
    let scale = scale_of(monitor);
    let work = info.rcWork;
    let w = ((WINDOW_WIDTH * scale).round() as i32).min(work.right - work.left);
    let h = ((WINDOW_HEIGHT * scale).round() as i32).min(work.bottom - work.top);
    (
        PhysicalPosition::new(
            work.left + (work.right - work.left - w) / 2,
            work.top + (work.bottom - work.top - h) / 2,
        ),
        PhysicalSize::new(w as u32, h as u32),
    )
}

/// Windows build number (e.g. 22631), read once from the registry.
fn os_build() -> u32 {
    use std::sync::OnceLock;
    use windows::core::w;
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};
    static BUILD: OnceLock<u32> = OnceLock::new();
    *BUILD.get_or_init(|| unsafe {
        let mut buf = [0u16; 32];
        let mut len = std::mem::size_of_val(&buf) as u32;
        let status = RegGetValueW(
            HKEY_LOCAL_MACHINE,
            w!(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion"),
            w!("CurrentBuildNumber"),
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr() as *mut _),
            Some(&mut len),
        );
        if status.is_err() {
            return 0;
        }
        String::from_utf16_lossy(&buf)
            .trim_end_matches('\0')
            .parse()
            .unwrap_or(0)
    })
}

/// Acrylic needs Windows 10 1809 (build 17763) or later.
pub fn glass_supported() -> bool {
    os_build() >= 17763
}

/// Windows 11 rounds and shadows the window natively.
pub fn native_corners() -> bool {
    os_build() >= 22000
}
