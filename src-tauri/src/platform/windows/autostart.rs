// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! Start with Windows via the per-user Run key (HKCU, no admin needed).

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::ERROR_FILE_NOT_FOUND;
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
    KEY_SET_VALUE, REG_SZ,
};

const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const VALUE_NAME: PCWSTR = w!("ClipDeck");

pub fn set(enabled: bool) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    unsafe {
        let mut key = HKEY::default();
        RegOpenKeyExW(HKEY_CURRENT_USER, RUN_KEY, None, KEY_SET_VALUE, &mut key)
            .ok()
            .map_err(|e| format!("open Run key: {e}"))?;
        let result = if enabled {
            let command = format!("\"{}\" --background", exe.display());
            let bytes: Vec<u8> = command
                .encode_utf16()
                .chain(std::iter::once(0))
                .flat_map(u16::to_le_bytes)
                .collect();
            RegSetValueExW(key, VALUE_NAME, None, REG_SZ, Some(&bytes)).ok()
        } else {
            match RegDeleteValueW(key, VALUE_NAME) {
                e if e == ERROR_FILE_NOT_FOUND => Ok(()),
                e => e.ok(),
            }
        };
        let _ = RegCloseKey(key);
        result.map_err(|e| format!("update Run key: {e}"))
    }
}
