// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! Focus-safe pasting: put the chosen item on the clipboard, give focus back
//! to the window the user was in, then send Ctrl+V with SendInput.

use std::time::{Duration, Instant};

use windows::Win32::Foundation::HWND;
use windows::Win32::System::Threading::{
    AttachThreadInput, GetCurrentProcessId, GetCurrentThreadId,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId, IsIconic, IsWindow,
    SetForegroundWindow, ShowWindow, SW_RESTORE,
};

use crate::clip::{self, ClipData, ReadOptions, ReadOutcome};
use crate::core_window;

/// The current foreground window, unless it belongs to ClipDeck itself.
pub fn foreground_target() -> Option<isize> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() {
            return None;
        }
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        (pid != GetCurrentProcessId()).then_some(hwnd.0 as isize)
    }
}

fn to_hwnd(raw: isize) -> HWND {
    HWND(raw as *mut _)
}

pub fn snapshot_clipboard() -> Option<ClipData> {
    match clip::read(core_window::hwnd(), &ReadOptions::everything()) {
        Ok(ReadOutcome::Data(data)) => Some(data),
        _ => None,
    }
}

pub fn set_clipboard(data: &ClipData) -> Result<(), String> {
    clip::write(core_window::hwnd(), data)?;
    core_window::mark_self_write();
    Ok(())
}

/// Brings `target` to the foreground. Uses AttachThreadInput to satisfy the
/// foreground-lock rules when another thread currently owns the input.
pub fn focus(target: isize) -> bool {
    let hwnd = to_hwnd(target);
    unsafe {
        if !IsWindow(Some(hwnd)).as_bool() {
            return false;
        }
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        if GetForegroundWindow() != hwnd {
            let foreground = GetForegroundWindow();
            let fg_thread = GetWindowThreadProcessId(foreground, None);
            let own_thread = GetCurrentThreadId();
            let attached = fg_thread != 0
                && fg_thread != own_thread
                && AttachThreadInput(own_thread, fg_thread, true).as_bool();
            let _ = BringWindowToTop(hwnd);
            let _ = SetForegroundWindow(hwnd);
            if attached {
                let _ = AttachThreadInput(own_thread, fg_thread, false);
            }
        }
        let deadline = Instant::now() + Duration::from_millis(500);
        while Instant::now() < deadline {
            if GetForegroundWindow() == hwnd {
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        GetForegroundWindow() == hwnd
    }
}

fn is_down(vk: VIRTUAL_KEY) -> bool {
    unsafe { (GetAsyncKeyState(vk.0 as i32) as u16 & 0x8000) != 0 }
}

/// Waits (up to 1.5 s) for the user to let go of Shift/Alt/Win from the
/// hotkey, so the target app sees exactly Ctrl+V.
fn wait_for_modifiers_released() {
    let deadline = Instant::now() + Duration::from_millis(1500);
    while Instant::now() < deadline {
        if ![VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN]
            .into_iter()
            .any(is_down)
        {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn key(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if up {
                    KEYEVENTF_KEYUP
                } else {
                    KEYBD_EVENT_FLAGS(0)
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

pub fn send_ctrl_v() {
    wait_for_modifiers_released();
    let ctrl_held = is_down(VK_CONTROL);
    let v = VIRTUAL_KEY(b'V' as u16);
    let mut inputs = Vec::with_capacity(4);
    if !ctrl_held {
        inputs.push(key(VK_CONTROL, false));
    }
    inputs.push(key(v, false));
    inputs.push(key(v, true));
    if !ctrl_held {
        inputs.push(key(VK_CONTROL, true));
    }
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}
