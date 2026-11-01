// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! Dashboard window styling and placement on Windows: native acrylic,
//! DWM corner rounding, and per-monitor DPI-aware placement.

use tauri::window::{Color, Effect, EffectsBuilder};
use tauri::{
    AppHandle, PhysicalPosition, PhysicalSize, Theme, WebviewWindow, WebviewWindowBuilder, Wry,
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

use crate::dashboard::{WINDOW_HEIGHT, WINDOW_WIDTH};
use crate::settings::{Settings, WindowBounds};

/// Trims WebView2 helper processes and background work to keep the open
/// dashboard light. The first three features are Tauri's own defaults.
const BROWSER_ARGS: &str = concat!(
    "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection,SpareRendererForSitePerProcess ",
    "--renderer-process-limit=1 --disable-background-networking --disable-component-update ",
    "--disable-extensions --disable-sync --no-pings"
);

pub fn configure<'a>(
    builder: WebviewWindowBuilder<'a, Wry, AppHandle>,
    windowed: bool,
) -> WebviewWindowBuilder<'a, Wry, AppHandle> {
    builder
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
        .additional_browser_args(BROWSER_ARGS)
}

/// Square corners for the full-screen overlay, rounded for the window (Windows 11).
pub fn style(window: &WebviewWindow, windowed: bool) {
    let _ = window.set_shadow(windowed);
    let Ok(hwnd) = window.hwnd() else { return };
    let preference: DWM_WINDOW_CORNER_PREFERENCE = if windowed {
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

/// Position, size and scale factor for the current display mode, on the
/// monitor under the cursor.
pub fn placement(
    _app: &AppHandle,
    settings: &Settings,
) -> Option<(PhysicalPosition<i32>, PhysicalSize<u32>, f64)> {
    let monitor = cursor_monitor()?;
    let info = monitor_info(monitor)?;
    let scale = scale_of(monitor);
    let (pos, size) = if settings.is_window_mode() {
        window_rect(settings.window_bounds, scale, &info)
    } else {
        // The whole monitor, taskbar included, like Mission Control.
        let r = info.rcMonitor;
        (
            PhysicalPosition::new(r.left, r.top),
            PhysicalSize::new((r.right - r.left) as u32, (r.bottom - r.top) as u32),
        )
    };
    Some((pos, size, scale))
}

/// The remembered window bounds if they are still on a connected monitor,
/// otherwise a default-sized window centered on the cursor's monitor.
fn window_rect(
    saved: Option<WindowBounds>,
    scale: f64,
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
pub fn os_build() -> u32 {
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
