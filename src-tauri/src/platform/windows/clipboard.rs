// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! Reading and writing the Windows clipboard.

use std::time::Duration;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    RegisterClipboardFormatW, SetClipboardData,
};
use windows::Win32::System::Memory::{
    GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE,
};

use crate::clip::{
    decode_utf16, ClipData, ClipFormat, ReadOptions, ReadOutcome, DIB, DROP_EFFECT, HDROP, HTML,
    PNG, RTF, TEXT,
};

const CF_DIB: u32 = 8;
const CF_UNICODETEXT: u32 = 13;
const CF_HDROP: u32 = 15;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn format_id(name: &str) -> u32 {
    match name {
        TEXT => CF_UNICODETEXT,
        DIB => CF_DIB,
        HDROP => CF_HDROP,
        other => {
            let w = wide(other);
            unsafe { RegisterClipboardFormatW(PCWSTR(w.as_ptr())) }
        }
    }
}

/// Keeps the clipboard open for the lifetime of the guard.
struct OpenGuard;

impl Drop for OpenGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseClipboard();
        }
    }
}

/// Another application may briefly hold the clipboard; retry for ~300 ms.
fn open(owner: HWND) -> Result<OpenGuard, String> {
    for attempt in 0..20 {
        if unsafe { OpenClipboard(Some(owner)) }.is_ok() {
            return Ok(OpenGuard);
        }
        std::thread::sleep(Duration::from_millis(if attempt < 5 { 5 } else { 20 }));
    }
    Err("clipboard is locked by another application".into())
}

fn available(id: u32) -> bool {
    id != 0 && unsafe { IsClipboardFormatAvailable(id) }.is_ok()
}

/// Copies one format out of the open clipboard. Returns `None` when the
/// format is missing or larger than `max_bytes`.
fn read_format(id: u32, max_bytes: usize) -> Option<Vec<u8>> {
    if !available(id) {
        return None;
    }
    unsafe {
        let handle = GetClipboardData(id).ok()?;
        let hglobal = HGLOBAL(handle.0);
        let size = GlobalSize(hglobal);
        if size == 0 || size > max_bytes {
            return None;
        }
        let ptr = GlobalLock(hglobal) as *const u8;
        if ptr.is_null() {
            return None;
        }
        let data = std::slice::from_raw_parts(ptr, size).to_vec();
        let _ = GlobalUnlock(hglobal);
        Some(data)
    }
}

pub fn read(owner: HWND, opts: &ReadOptions) -> Result<ReadOutcome, String> {
    let _guard = open(owner)?;

    // Honour the documented opt-out formats used by password managers.
    if available(format_id("ExcludeClipboardContentFromMonitorProcessing"))
        || available(format_id("Clipboard Viewer Ignore"))
    {
        return Ok(ReadOutcome::Excluded);
    }
    if let Some(flag) = read_format(format_id("CanIncludeInClipboardHistory"), 64) {
        if flag.len() >= 4 && u32::from_le_bytes([flag[0], flag[1], flag[2], flag[3]]) == 0 {
            return Ok(ReadOutcome::Excluded);
        }
    }

    let mut budget = opts.max_bytes;
    let mut formats = Vec::new();
    let mut push = |name: &str, formats: &mut Vec<ClipFormat>| {
        if let Some(mut data) = read_format(format_id(name), budget) {
            if name == TEXT {
                let units = data.chunks_exact(2).take_while(|c| c != &[0, 0]).count();
                data.truncate(units * 2);
                if data.is_empty() {
                    return;
                }
            }
            budget = budget.saturating_sub(data.len());
            formats.push(ClipFormat {
                name: name.into(),
                data,
            });
        }
    };

    if available(CF_HDROP) {
        if !opts.files {
            return Ok(ReadOutcome::Empty);
        }
        push(HDROP, &mut formats);
        push(DROP_EFFECT, &mut formats);
        push(TEXT, &mut formats);
    } else {
        push(TEXT, &mut formats);
        if opts.rich {
            push(HTML, &mut formats);
            push(RTF, &mut formats);
        }
        if opts.images {
            push(PNG, &mut formats);
            push(DIB, &mut formats);
        }
    }

    let has_text = formats
        .iter()
        .any(|f| f.name == TEXT && !decode_utf16(&f.data).trim().is_empty());
    let has_payload = has_text
        || formats
            .iter()
            .any(|f| matches!(f.name.as_str(), DIB | PNG | HDROP));
    if !has_payload {
        return Ok(ReadOutcome::Empty);
    }
    Ok(ReadOutcome::Data(ClipData { formats }))
}

pub fn write(owner: HWND, data: &ClipData) -> Result<(), String> {
    let _guard = open(owner)?;
    unsafe {
        EmptyClipboard().map_err(|e| format!("EmptyClipboard: {e}"))?;
        for f in &data.formats {
            // Text formats are stored without terminators; add them back.
            let terminator: &[u8] = match f.name.as_str() {
                TEXT => &[0, 0],
                _ => &[],
            };
            let len = f.data.len() + terminator.len();
            let Ok(hglobal) = GlobalAlloc(GMEM_MOVEABLE, len.max(1)) else {
                continue;
            };
            let ptr = GlobalLock(hglobal) as *mut u8;
            if ptr.is_null() {
                let _ = GlobalFree(Some(hglobal));
                continue;
            }
            std::ptr::copy_nonoverlapping(f.data.as_ptr(), ptr, f.data.len());
            std::ptr::copy_nonoverlapping(
                terminator.as_ptr(),
                ptr.add(f.data.len()),
                terminator.len(),
            );
            let _ = GlobalUnlock(hglobal);
            if SetClipboardData(format_id(&f.name), Some(HANDLE(hglobal.0))).is_err() {
                let _ = GlobalFree(Some(hglobal));
            }
        }
    }
    Ok(())
}
