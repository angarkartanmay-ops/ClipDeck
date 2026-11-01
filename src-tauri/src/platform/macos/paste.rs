// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! Pasting into the previous app on macOS: hand focus back to it, then post
//! ⌘V. Posting keystrokes needs the Accessibility permission, which the user
//! grants once in System Settings; without it ClipDeck only copies.

use std::ffi::c_void;
use std::time::{Duration, Instant};

use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::CFString;
use objc2::rc::autoreleasepool;
use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication, NSWorkspace};
use tauri::AppHandle;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> u8;
    fn AXIsProcessTrustedWithOptions(options: *const c_void) -> u8;
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventSourceCreate(state: i32) -> *mut c_void;
    fn CGEventSourceFlagsState(state: i32) -> u64;
    fn CGEventCreateKeyboardEvent(source: *mut c_void, key: u16, down: bool) -> *mut c_void;
    fn CGEventSetFlags(event: *mut c_void, flags: u64);
    fn CGEventPost(tap: u32, event: *mut c_void);
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(cf: *const c_void);
}

const COMBINED_SESSION_STATE: i32 = 0;
const HID_SYSTEM_STATE: i32 = 1;
const HID_EVENT_TAP: u32 = 0;
const KEY_V: u16 = 0x09; // kVK_ANSI_V
const FLAG_SHIFT: u64 = 1 << 17;
const FLAG_CONTROL: u64 = 1 << 18;
const FLAG_OPTION: u64 = 1 << 19;
const FLAG_COMMAND: u64 = 1 << 20;

/// Whether ClipDeck may post keystrokes (Accessibility permission).
pub fn can_auto_paste() -> bool {
    unsafe { AXIsProcessTrusted() != 0 }
}

/// Shows the system prompt that adds ClipDeck to the Accessibility list.
pub fn request_accessibility() {
    let key = CFString::new("AXTrustedCheckOptionPrompt");
    let options =
        CFDictionary::from_CFType_pairs(&[(key.as_CFType(), CFBoolean::true_value().as_CFType())]);
    unsafe {
        AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef() as *const c_void);
    }
}

fn frontmost_pid() -> Option<i32> {
    autoreleasepool(|_| {
        NSWorkspace::sharedWorkspace()
            .frontmostApplication()
            .map(|app| app.processIdentifier())
    })
}

/// The app to paste into (its process id), unless ClipDeck itself is in front.
pub fn foreground_target() -> Option<i64> {
    let pid = frontmost_pid()?;
    (pid as u32 != std::process::id()).then_some(pid as i64)
}

fn wait_frontmost(pid: i32, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if frontmost_pid() == Some(pid) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    frontmost_pid() == Some(pid)
}

/// Waits (up to 1.5 s) for the user to let go of the hotkey's modifiers so
/// the target app sees exactly ⌘V.
fn wait_for_modifiers_released() {
    let deadline = Instant::now() + Duration::from_millis(1500);
    while Instant::now() < deadline {
        let flags = unsafe { CGEventSourceFlagsState(HID_SYSTEM_STATE) };
        if flags & (FLAG_SHIFT | FLAG_CONTROL | FLAG_OPTION | FLAG_COMMAND) == 0 {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn post_cmd_v() {
    unsafe {
        let source = CGEventSourceCreate(COMBINED_SESSION_STATE);
        for down in [true, false] {
            let event = CGEventCreateKeyboardEvent(source, KEY_V, down);
            if event.is_null() {
                continue;
            }
            // Exactly ⌘: any key the user still holds must not leak in.
            CGEventSetFlags(event, FLAG_COMMAND);
            CGEventPost(HID_EVENT_TAP, event);
            CFRelease(event);
        }
        if !source.is_null() {
            CFRelease(source);
        }
    }
}

/// Returns focus to `pid` and pastes. Returns false (item stays copied)
/// without the Accessibility permission or if the app cannot be focused.
pub fn paste_into(app: &AppHandle, pid: i64) -> bool {
    if !can_auto_paste() {
        return false;
    }
    let pid = pid as i32;
    // Hiding ClipDeck hands activation back to the previous app.
    let _ = app.hide();
    if !wait_frontmost(pid, Duration::from_millis(250)) {
        autoreleasepool(|_| {
            if let Some(target) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
            {
                #[allow(deprecated)]
                target
                    .activateWithOptions(NSApplicationActivationOptions::ActivateIgnoringOtherApps);
            }
        });
        if !wait_frontmost(pid, Duration::from_millis(600)) {
            return false;
        }
    }
    wait_for_modifiers_released();
    post_cmd_v();
    true
}
