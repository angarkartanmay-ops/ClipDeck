// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! The always-on core: a hidden top-level window on its own thread that
//! receives clipboard notifications (AddClipboardFormatListener, no polling),
//! hotkeys (RegisterHotKey) and session-end messages.

use std::sync::atomic::{AtomicIsize, AtomicU32, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{
    GetLastError, ERROR_ALREADY_EXISTS, HANDLE, HWND, LPARAM, LRESULT, WPARAM,
};
use windows::Win32::System::DataExchange::{
    AddClipboardFormatListener, GetClipboardOwner, GetClipboardSequenceNumber,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, CreateWindowExW, DefWindowProcW, DispatchMessageW, FindWindowW,
    GetForegroundWindow, GetMessageW, GetWindowThreadProcessId, PostMessageW, RegisterClassW,
    TranslateMessage, MSG, WM_APP, WM_CLIPBOARDUPDATE, WM_ENDSESSION, WM_HOTKEY,
    WM_QUERYENDSESSION, WNDCLASSW, WS_EX_TOOLWINDOW, WS_POPUP,
};

use super::{clipboard, hotkeys, source_app};
use crate::actions;
use crate::clip::{ReadOptions, ReadOutcome};
use crate::state::AppState;
use crate::worker::Job;

const CLASS_NAME: PCWSTR = w!("ClipDeck.CoreWindow");
const WM_APP_RELOAD_HOTKEYS: u32 = WM_APP + 1;
const WM_APP_SHOW: u32 = WM_APP + 2;

struct Context {
    app: AppHandle,
    jobs: Sender<Job>,
}

static CONTEXT: OnceLock<Context> = OnceLock::new();
static CORE_HWND: AtomicIsize = AtomicIsize::new(0);
/// Clipboard sequence number right after ClipDeck itself wrote the clipboard.
static SELF_WRITE_SEQ: AtomicU32 = AtomicU32::new(0);

pub fn hwnd() -> HWND {
    HWND(CORE_HWND.load(Ordering::Acquire) as *mut _)
}

pub fn mark_self_write() {
    SELF_WRITE_SEQ.store(unsafe { GetClipboardSequenceNumber() }, Ordering::Release);
}

pub fn reload_hotkeys() {
    unsafe {
        let _ = PostMessageW(Some(hwnd()), WM_APP_RELOAD_HOTKEYS, WPARAM(0), LPARAM(0));
    }
}

/// Holds the single-instance mutex for the life of the process.
pub struct InstanceGuard(#[allow(dead_code)] HANDLE);

pub fn acquire_single_instance() -> Option<InstanceGuard> {
    unsafe {
        let handle = CreateMutexW(None, true, w!("Local\\ClipDeck.SingleInstance")).ok()?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return None;
        }
        Some(InstanceGuard(handle))
    }
}

pub fn signal_running_instance() {
    unsafe {
        if let Ok(hwnd) = FindWindowW(CLASS_NAME, None) {
            // This (just launched) process may take the foreground; pass that
            // right on so the running instance's dashboard can be focused.
            let mut pid = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            let _ = AllowSetForegroundWindow(pid);
            let _ = PostMessageW(Some(hwnd), WM_APP_SHOW, WPARAM(0), LPARAM(0));
        }
    }
}

pub fn spawn(app: AppHandle, jobs: Sender<Job>) {
    let _ = CONTEXT.set(Context { app, jobs });
    std::thread::Builder::new()
        .name("clipdeck-core".into())
        .spawn(|| unsafe { run() })
        .expect("spawn core thread");
}

unsafe fn run() {
    let instance = GetModuleHandleW(None).expect("module handle");
    let class = WNDCLASSW {
        lpfnWndProc: Some(wndproc),
        hInstance: instance.into(),
        lpszClassName: CLASS_NAME,
        ..Default::default()
    };
    RegisterClassW(&class);
    // A hidden top-level window rather than a message-only one, so that it
    // also receives WM_QUERYENDSESSION / WM_ENDSESSION broadcasts.
    let hwnd = CreateWindowExW(
        WS_EX_TOOLWINDOW,
        CLASS_NAME,
        w!("ClipDeck"),
        WS_POPUP,
        0,
        0,
        0,
        0,
        None,
        None,
        Some(instance.into()),
        None,
    )
    .expect("create core window");
    CORE_HWND.store(hwnd.0 as isize, Ordering::Release);

    if let Err(err) = AddClipboardFormatListener(hwnd) {
        eprintln!("ClipDeck: AddClipboardFormatListener failed: {err}");
    }
    reregister_hotkeys(hwnd);

    let mut msg = MSG::default();
    while GetMessageW(&mut msg, None, 0, 0).as_bool() {
        let _ = TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
}

fn context() -> &'static Context {
    CONTEXT.get().expect("core context")
}

fn state() -> Arc<AppState> {
    context().app.state::<Arc<AppState>>().inner().clone()
}

fn reregister_hotkeys(hwnd: HWND) {
    let errors = hotkeys::register_all(hwnd, &state().settings());
    for err in &errors {
        eprintln!("ClipDeck: hotkey: {err}");
    }
    *state().hotkey_errors.lock().unwrap() = errors;
    actions::emit_status(&context().app);
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CLIPBOARDUPDATE => {
            on_clipboard_update(hwnd);
            LRESULT(0)
        }
        WM_HOTKEY => {
            on_hotkey(wparam.0 as i32);
            LRESULT(0)
        }
        WM_APP_RELOAD_HOTKEYS => {
            reregister_hotkeys(hwnd);
            LRESULT(0)
        }
        WM_APP_SHOW => {
            let app = context().app.clone();
            std::thread::spawn(move || crate::dashboard::toggle(&app, None, ""));
            LRESULT(0)
        }
        WM_QUERYENDSESSION => LRESULT(1),
        WM_ENDSESSION => {
            if wparam.0 != 0 {
                actions::clear_if_configured(&state());
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn on_hotkey(id: i32) {
    // Remember the window to paste into before the dashboard takes focus.
    let target = super::foreground_target();
    let app = context().app.clone();
    std::thread::spawn(move || match id {
        hotkeys::ID_DASHBOARD => actions::dashboard_hotkey(&app, target),
        hotkeys::ID_QUEUE => actions::toggle_queue(&app),
        n if (hotkeys::SLOT_BASE + 1..=hotkeys::SLOT_BASE + 9).contains(&n) => {
            actions::paste_slot(&app, (n - hotkeys::SLOT_BASE) as usize, target)
        }
        _ => {}
    });
}

fn on_clipboard_update(hwnd: HWND) {
    let state = state();
    if !state.capture_allowed() {
        return;
    }
    unsafe {
        let owner = GetClipboardOwner().unwrap_or_default();
        if owner == hwnd || GetClipboardSequenceNumber() == SELF_WRITE_SEQ.load(Ordering::Acquire) {
            return; // ClipDeck's own paste
        }
        let source_window = if owner.is_invalid() {
            GetForegroundWindow()
        } else {
            owner
        };
        let source = source_app::for_window(source_window);
        let settings = state.settings();
        if let Some(src) = &source {
            if settings.is_blocked(&src.exe) {
                return;
            }
        }
        let opts = ReadOptions {
            images: settings.capture_images,
            files: settings.capture_files,
            rich: settings.capture_rich,
            max_bytes: settings.max_item_bytes(),
        };
        match clipboard::read(hwnd, &opts) {
            Ok(ReadOutcome::Data(data)) => {
                let created_at = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or_default();
                let (source_exe, source_name) = source.map(|s| (s.exe, s.name)).unwrap_or_default();
                let _ = context().jobs.send(Job::Capture {
                    data,
                    created_at,
                    source_exe,
                    source_name,
                });
            }
            Ok(_) => {}
            Err(err) => eprintln!("ClipDeck: capture: {err}"),
        }
    }
}
