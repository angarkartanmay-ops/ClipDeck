// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! Resolves which application a clipboard change came from.

use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};

use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HWND};
use windows::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;

pub struct SourceApp {
    /// Lower-case executable file name, e.g. "chrome.exe".
    pub exe: String,
    /// Friendly name from the executable's version info, e.g. "Google Chrome".
    pub name: String,
}

fn names() -> &'static Mutex<HashMap<String, String>> {
    static CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    CACHE.get_or_init(Default::default)
}

pub fn for_window(hwnd: HWND) -> Option<SourceApp> {
    if hwnd.is_invalid() {
        return None;
    }
    let path = unsafe {
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(process);
        ok.ok()?;
        String::from_utf16_lossy(&buf[..len as usize])
    };
    let exe = path.rsplit('\\').next().unwrap_or(&path).to_lowercase();
    let name = {
        let mut cache = names().lock().unwrap();
        cache
            .entry(path.clone())
            .or_insert_with(|| file_description(&path).unwrap_or_else(|| prettify(&exe)))
            .clone()
    };
    Some(SourceApp { exe, name })
}

fn prettify(exe: &str) -> String {
    let stem = exe.strip_suffix(".exe").unwrap_or(exe);
    let mut chars = stem.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn file_description(path: &str) -> Option<String> {
    let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let size = GetFileVersionInfoSizeW(PCWSTR(wide.as_ptr()), None);
        if size == 0 {
            return None;
        }
        let mut buf = vec![0u8; size as usize];
        GetFileVersionInfoW(
            PCWSTR(wide.as_ptr()),
            None,
            size,
            buf.as_mut_ptr() as *mut c_void,
        )
        .ok()?;

        let mut ptr: *mut c_void = std::ptr::null_mut();
        let mut len = 0u32;
        let key: Vec<u16> = "\\VarFileInfo\\Translation\0".encode_utf16().collect();
        if !VerQueryValueW(
            buf.as_ptr() as *const c_void,
            PCWSTR(key.as_ptr()),
            &mut ptr,
            &mut len,
        )
        .as_bool()
            || len < 4
        {
            return None;
        }
        let lang = *(ptr as *const u16);
        let codepage = *(ptr as *const u16).add(1);
        let key: Vec<u16> =
            format!("\\StringFileInfo\\{lang:04x}{codepage:04x}\\FileDescription\0")
                .encode_utf16()
                .collect();
        if !VerQueryValueW(
            buf.as_ptr() as *const c_void,
            PCWSTR(key.as_ptr()),
            &mut ptr,
            &mut len,
        )
        .as_bool()
            || len == 0
        {
            return None;
        }
        let text = std::slice::from_raw_parts(ptr as *const u16, len as usize);
        let text = String::from_utf16_lossy(text);
        let text = text.trim_end_matches('\0').trim();
        (!text.is_empty()).then(|| text.to_string())
    }
}
