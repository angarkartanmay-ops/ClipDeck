// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! Administrator policy on Windows: values under
//! `HKLM\SOFTWARE\Policies\ClipDeck` (and `HKCU\...`), as written by the
//! ClipDeck Group Policy template or Intune, plus Windows' own
//! "Allow Clipboard History" policy.

use serde_json::{Map, Value};
use windows::core::{w, PCWSTR, PWSTR};
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    RegCloseKey, RegEnumValueW, RegGetValueW, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER,
    HKEY_LOCAL_MACHINE, KEY_READ, REG_DWORD, REG_EXPAND_SZ, REG_MULTI_SZ, REG_SZ, RRF_RT_REG_DWORD,
};

use crate::policy::PolicySource;

const CLIPDECK_KEY: PCWSTR = w!(r"SOFTWARE\Policies\ClipDeck");

pub fn read() -> PolicySource {
    let mut values = Map::new();
    // User policy first; machine policy is applied last and wins.
    for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        read_key(root, &mut values);
    }
    let history_off = [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER]
        .into_iter()
        .any(|root| {
            dword(
                root,
                w!(r"SOFTWARE\Policies\Microsoft\Windows\System"),
                w!("AllowClipboardHistory"),
            ) == Some(0)
        });
    PolicySource {
        values,
        os_history_disabled: history_off,
        os_history_notice:
            "Clipboard history is turned off by your organization's Windows policy, \
                            so ClipDeck is not recording new copies."
                .into(),
    }
}

fn dword(root: HKEY, path: PCWSTR, name: PCWSTR) -> Option<u32> {
    let mut value = 0u32;
    let mut len = std::mem::size_of::<u32>() as u32;
    let status = unsafe {
        RegGetValueW(
            root,
            path,
            name,
            RRF_RT_REG_DWORD,
            None,
            Some(&mut value as *mut u32 as *mut _),
            Some(&mut len),
        )
    };
    (status == ERROR_SUCCESS).then_some(value)
}

fn utf16_string(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let end = units.iter().position(|u| *u == 0).unwrap_or(units.len());
    String::from_utf16_lossy(&units[..end])
}

fn read_key(root: HKEY, out: &mut Map<String, Value>) {
    let mut key = HKEY::default();
    unsafe {
        if RegOpenKeyExW(root, CLIPDECK_KEY, None, KEY_READ, &mut key) != ERROR_SUCCESS {
            return;
        }
        for index in 0.. {
            let mut name = [0u16; 256];
            let mut name_len = name.len() as u32;
            let mut kind = 0u32;
            let mut data = vec![0u8; 32 * 1024];
            let mut data_len = data.len() as u32;
            let status = RegEnumValueW(
                key,
                index,
                Some(PWSTR(name.as_mut_ptr())),
                &mut name_len,
                None,
                Some(&mut kind),
                Some(data.as_mut_ptr()),
                Some(&mut data_len),
            );
            if status != ERROR_SUCCESS {
                break;
            }
            data.truncate(data_len as usize);
            let name = String::from_utf16_lossy(&name[..name_len as usize]);
            let value = match kind {
                k if k == REG_DWORD.0 && data.len() >= 4 => {
                    Value::from(u32::from_le_bytes([data[0], data[1], data[2], data[3]]))
                }
                k if k == REG_SZ.0 || k == REG_EXPAND_SZ.0 => Value::from(utf16_string(&data)),
                k if k == REG_MULTI_SZ.0 => {
                    let units: Vec<u16> = data
                        .chunks_exact(2)
                        .map(|c| u16::from_le_bytes([c[0], c[1]]))
                        .collect();
                    Value::Array(
                        units
                            .split(|u| *u == 0)
                            .filter(|part| !part.is_empty())
                            .map(|part| Value::from(String::from_utf16_lossy(part)))
                            .collect(),
                    )
                }
                _ => continue,
            };
            out.insert(name, value);
        }
        let _ = RegCloseKey(key);
    }
}
