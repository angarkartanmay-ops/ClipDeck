// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! The on-demand dashboard window. It is created when opened and destroyed
//! when closed (unless "keep dashboard ready" is on), so the idle process
//! carries no webview.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::window::{Color, Effect, EffectsBuilder};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Theme, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
    DWM_WINDOW_CORNER_PREFERENCE,
};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

use crate::settings::Settings;
use crate::state::AppState;

pub const LABEL: &str = "dashboard";
/// Logical size of the window. The panel fills it edge to edge; Windows 11
/// draws the rounded corners, border and shadow.
const WIDTH: f64 = 640.0;
const HEIGHT: f64 = 420.0;

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
    let url = WebviewUrl::App(format!("index.html?mode={mode}").into());
    let built = WebviewWindowBuilder::new(app, LABEL, url)
        .title("ClipDeck")
        .inner_size(WIDTH, HEIGHT)
        .decorations(false)
        .transparent(true)
        .shadow(true)
        // Native acrylic: blurs whatever is behind the window. The tint
        // colour is only used by Windows 10's acrylic implementation.
        .effects(
            EffectsBuilder::new()
                .effect(Effect::Acrylic)
                .color(Color(24, 24, 30, 96))
                .build(),
        )
        .theme(theme(&state.settings()))
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .visible(false)
        .focused(false)
        .additional_browser_args(BROWSER_ARGS)
        .build();
    match built {
        Ok(window) => round_corners(&window),
        Err(err) => eprintln!("ClipDeck: cannot open dashboard: {err}"),
    }
    // The page calls `dashboard_ready` after its first render, which shows it.
}

/// Positions, shows and focuses the dashboard.
pub fn show(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LABEL) else {
        return;
    };
    let centered = app.state::<Arc<AppState>>().settings().position == "center";
    if let Some((pos, size)) = placement(centered) {
        let _ = window.set_position(pos);
        let _ = window.set_size(size);
    }
    SHOWN.store(true, Ordering::SeqCst);
    let _ = window.show();
    let _ = window.set_focus();
}

pub fn close(app: &AppHandle) {
    SHOWN.store(false, Ordering::SeqCst);
    ACTIVE.store(false, Ordering::SeqCst);
    let Some(window) = app.get_webview_window(LABEL) else {
        return;
    };
    if app.state::<Arc<AppState>>().settings().keep_dashboard_warm {
        let _ = window.hide();
        let _ = window.emit("dashboard-hidden", ());
    } else {
        let _ = window.destroy();
    }
}

/// Window theme for the acrylic tint: follows the app setting, or the
/// system when set to "system".
pub fn theme(settings: &Settings) -> Option<Theme> {
    match settings.theme.as_str() {
        "dark" => Some(Theme::Dark),
        "light" => Some(Theme::Light),
        _ => None,
    }
}

pub fn apply_theme(app: &AppHandle, settings: &Settings) {
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.set_theme(theme(settings));
    }
}

/// Asks Windows 11 for rounded corners on the frameless window (no-op on 10).
fn round_corners(window: &WebviewWindow) {
    let Ok(hwnd) = window.hwnd() else { return };
    let preference: DWM_WINDOW_CORNER_PREFERENCE = DWMWCP_ROUND;
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

/// Physical position and size on the monitor under the cursor: near the
/// cursor (clamped to the work area) or centered.
fn placement(centered: bool) -> Option<(PhysicalPosition<i32>, PhysicalSize<u32>)> {
    unsafe {
        let mut cursor = POINT::default();
        GetCursorPos(&mut cursor).ok()?;
        let monitor = MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return None;
        }
        let (mut dpi_x, mut dpi_y) = (96u32, 96u32);
        let _ = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
        let scale = dpi_x as f64 / 96.0;
        let work = info.rcWork;
        let w = ((WIDTH * scale).round() as i32).min(work.right - work.left);
        let h = ((HEIGHT * scale).round() as i32).min(work.bottom - work.top);
        let (x, y) = if centered {
            (
                work.left + (work.right - work.left - w) / 2,
                work.top + (work.bottom - work.top - h) / 2,
            )
        } else {
            (cursor.x - w / 2, cursor.y - (36.0 * scale) as i32)
        };
        let x = x.clamp(work.left, work.right - w);
        let y = y.clamp(work.top, work.bottom - h);
        Some((
            PhysicalPosition::new(x, y),
            PhysicalSize::new(w as u32, h as u32),
        ))
    }
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
