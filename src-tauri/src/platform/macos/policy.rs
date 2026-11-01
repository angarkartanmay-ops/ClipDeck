// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! Administrator policy on macOS: managed preferences for the domain
//! `com.tanmayangarkar.clipdeck`, installed with a configuration profile
//! (Jamf, Intune, Kandji, Apple Business Manager...). Only values the
//! profile forces are read; the user's own preferences are not policy.

use std::ffi::c_void;

use core_foundation::array::CFArray;
use core_foundation::base::{CFType, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use core_foundation_sys::preferences::{CFPreferencesAppValueIsForced, CFPreferencesCopyAppValue};
use serde_json::{Map, Value};

use crate::policy::PolicySource;
use crate::settings::Settings;

const DOMAIN: &str = "com.tanmayangarkar.clipdeck";

fn to_json(value: &CFType) -> Option<Value> {
    if let Some(b) = value.downcast::<CFBoolean>() {
        return Some(Value::Bool(b.into()));
    }
    if let Some(n) = value.downcast::<CFNumber>() {
        return n
            .to_i64()
            .map(Value::from)
            .or_else(|| n.to_f64().map(Value::from));
    }
    if let Some(s) = value.downcast::<CFString>() {
        return Some(Value::String(s.to_string()));
    }
    if let Some(array) = value.downcast::<CFArray<*const c_void>>() {
        let items = array
            .iter()
            .filter_map(|item| {
                let item = unsafe { CFType::wrap_under_get_rule(*item) };
                item.downcast::<CFString>()
                    .map(|s| Value::String(s.to_string()))
            })
            .collect();
        return Some(Value::Array(items));
    }
    None
}

pub fn read() -> PolicySource {
    let domain = CFString::new(DOMAIN);
    let known = serde_json::to_value(Settings::default()).unwrap_or_default();
    let keys = known
        .as_object()
        .map(|o| o.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .chain(["disableCapture".to_string()]);

    let mut values = Map::new();
    for key in keys {
        let cf_key = CFString::new(&key);
        unsafe {
            if CFPreferencesAppValueIsForced(
                cf_key.as_concrete_TypeRef(),
                domain.as_concrete_TypeRef(),
            ) == 0
            {
                continue;
            }
            let raw = CFPreferencesCopyAppValue(
                cf_key.as_concrete_TypeRef(),
                domain.as_concrete_TypeRef(),
            );
            if raw.is_null() {
                continue;
            }
            if let Some(value) = to_json(&CFType::wrap_under_create_rule(raw)) {
                values.insert(key, value);
            }
        }
    }
    PolicySource {
        values,
        ..Default::default()
    }
}
