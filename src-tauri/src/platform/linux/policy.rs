// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! Administrator policy on Linux: `/etc/clipdeck/policy.json`, a JSON object
//! with the same keys as the Windows and macOS policies.

use crate::policy::PolicySource;

const PATH: &str = "/etc/clipdeck/policy.json";

pub fn read() -> PolicySource {
    let values = std::fs::read_to_string(PATH)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    PolicySource {
        values,
        ..Default::default()
    }
}
