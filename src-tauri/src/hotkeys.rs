// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! Global hotkeys via RegisterHotKey. ClipDeck never installs a keyboard hook,
//! so plain Ctrl+C / Ctrl+V are untouched.

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT,
    MOD_SHIFT, MOD_WIN,
};

use crate::settings::Settings;

pub const ID_DASHBOARD: i32 = 1;
pub const ID_QUEUE: i32 = 2;
/// Slot hotkeys use ids SLOT_BASE + 1 ..= SLOT_BASE + 9.
pub const SLOT_BASE: i32 = 10;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hotkey {
    pub modifiers: u32,
    pub vk: u32,
}

fn parse_modifier(token: &str) -> Option<u32> {
    match token.to_ascii_lowercase().as_str() {
        "ctrl" | "control" => Some(MOD_CONTROL.0),
        "shift" => Some(MOD_SHIFT.0),
        "alt" => Some(MOD_ALT.0),
        "win" | "super" | "meta" => Some(MOD_WIN.0),
        _ => None,
    }
}

fn parse_key(token: &str) -> Option<u32> {
    let upper = token.to_ascii_uppercase();
    if upper.len() == 1 {
        let c = upper.as_bytes()[0];
        if c.is_ascii_alphanumeric() {
            return Some(c as u32);
        }
    }
    if let Some(n) = upper.strip_prefix('F').and_then(|n| n.parse::<u32>().ok()) {
        if (1..=24).contains(&n) {
            return Some(0x70 + n - 1);
        }
    }
    Some(match upper.as_str() {
        "SPACE" => 0x20,
        "INSERT" => 0x2D,
        "HOME" => 0x24,
        "END" => 0x23,
        "PAGEUP" => 0x21,
        "PAGEDOWN" => 0x22,
        "`" | "BACKQUOTE" => 0xC0,
        "." | "PERIOD" => 0xBE,
        "," | "COMMA" => 0xBC,
        ";" | "SEMICOLON" => 0xBA,
        "/" | "SLASH" => 0xBF,
        _ => return None,
    })
}

/// Parses strings like "Ctrl+Shift+V". At least one modifier is required.
pub fn parse(spec: &str) -> Option<Hotkey> {
    let tokens: Vec<&str> = spec
        .split('+')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .collect();
    let (key, mods) = tokens.split_last()?;
    let mut modifiers = 0;
    for m in mods {
        modifiers |= parse_modifier(m)?;
    }
    if modifiers == 0 {
        return None;
    }
    Some(Hotkey {
        modifiers,
        vk: parse_key(key)?,
    })
}

pub fn parse_modifiers(spec: &str) -> Option<u32> {
    let mut modifiers = 0;
    for token in spec.split('+').map(str::trim).filter(|t| !t.is_empty()) {
        modifiers |= parse_modifier(token)?;
    }
    (modifiers != 0).then_some(modifiers)
}

fn register(hwnd: HWND, id: i32, hotkey: Hotkey) -> bool {
    let mods = HOT_KEY_MODIFIERS(hotkey.modifiers) | MOD_NOREPEAT;
    unsafe { RegisterHotKey(Some(hwnd), id, mods, hotkey.vk) }.is_ok()
}

/// (Re-)registers all hotkeys. Returns human-readable errors for any that
/// could not be registered (usually because another app already owns them).
pub fn register_all(hwnd: HWND, settings: &Settings) -> Vec<String> {
    unsafe {
        for id in [ID_DASHBOARD, ID_QUEUE]
            .into_iter()
            .chain((1..=9).map(|n| SLOT_BASE + n))
        {
            let _ = UnregisterHotKey(Some(hwnd), id);
        }
    }
    let mut errors = Vec::new();
    for (id, spec, what) in [
        (ID_DASHBOARD, &settings.hotkey_dashboard, "dashboard"),
        (ID_QUEUE, &settings.hotkey_queue, "paste queue"),
    ] {
        if spec.trim().is_empty() {
            continue;
        }
        match parse(spec) {
            Some(hk) if register(hwnd, id, hk) => {}
            Some(_) => errors.push(format!("{spec} ({what}) is already used by another app")),
            None => errors.push(format!("\"{spec}\" ({what}) is not a valid hotkey")),
        }
    }
    if !settings.slot_modifiers.trim().is_empty() {
        match parse_modifiers(&settings.slot_modifiers) {
            Some(modifiers) => {
                let failed: Vec<String> = (1..=9)
                    .filter(|n| {
                        !register(
                            hwnd,
                            SLOT_BASE + n,
                            Hotkey {
                                modifiers,
                                vk: 0x30 + *n as u32,
                            },
                        )
                    })
                    .map(|n| n.to_string())
                    .collect();
                if !failed.is_empty() {
                    errors.push(format!(
                        "{}+{} (paste slot) is already used by another app",
                        settings.slot_modifiers,
                        failed.join("/")
                    ));
                }
            }
            None => errors.push(format!(
                "\"{}\" are not valid slot modifiers",
                settings.slot_modifiers
            )),
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hotkeys() {
        assert_eq!(
            parse("Ctrl+Shift+V"),
            Some(Hotkey {
                modifiers: MOD_CONTROL.0 | MOD_SHIFT.0,
                vk: 'V' as u32
            })
        );
        assert_eq!(
            parse("alt + f2"),
            Some(Hotkey {
                modifiers: MOD_ALT.0,
                vk: 0x71
            })
        );
        assert_eq!(parse("V"), None);
        assert_eq!(parse("Ctrl+Hyper+V"), None);
        assert_eq!(parse_modifiers("Ctrl+Alt"), Some(MOD_CONTROL.0 | MOD_ALT.0));
    }
}
