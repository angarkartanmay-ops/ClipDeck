// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! Administrator-managed settings for company devices.
//!
//! Policy keys are the settings' own JSON names (e.g. `encrypt`,
//! `historyLimit`, `blocklist`), plus two controls:
//!
//! - `disableCapture`: ClipDeck records nothing on this device.
//! - `allowWhenSystemHistoryDisabled`: keep working even when the OS's own
//!   clipboard history has been turned off by policy (Windows).
//!
//! Sources: Windows registry `SOFTWARE\Policies\ClipDeck` (Group Policy /
//! Intune), macOS managed preferences for `com.tanmayangarkar.clipdeck`
//! (configuration profile), Linux `/etc/clipdeck/policy.json`. Values set by
//! policy override the user's choice and are shown as locked.

use serde_json::{Map, Value};

use crate::settings::Settings;

/// What the platform layer read, before interpretation.
#[derive(Default)]
pub struct PolicySource {
    pub values: Map<String, Value>,
    /// The operating system's own clipboard history is disabled by policy.
    pub os_history_disabled: bool,
    pub os_history_notice: String,
}

#[derive(Default, Clone)]
pub struct Policy {
    /// Settings fixed by the administrator.
    values: Map<String, Value>,
    pub capture_disabled: bool,
    pub notice: Option<String>,
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
        Value::String(s) => matches!(s.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes"),
        _ => false,
    }
}

/// Converts a policy value to the JSON type the setting expects (registry
/// DWORDs for booleans, strings for numbers, delimited strings for lists).
fn coerce(current: &Value, v: &Value) -> Value {
    match (current, v) {
        (Value::Bool(_), _) => Value::Bool(truthy(v)),
        (Value::Number(_), Value::String(s)) => s
            .trim()
            .parse::<u64>()
            .map(Value::from)
            .unwrap_or_else(|_| v.clone()),
        (Value::Number(_), Value::Bool(b)) => Value::from(*b as u64),
        (Value::Array(_), Value::String(s)) => Value::Array(
            s.split(['\n', ';', ','])
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .map(Value::from)
                .collect(),
        ),
        (Value::String(_), Value::Number(n)) => Value::from(n.to_string()),
        _ => v.clone(),
    }
}

impl Policy {
    pub fn load() -> Policy {
        Policy::from_source(crate::platform::read_policy())
    }

    pub fn from_source(mut src: PolicySource) -> Policy {
        let disable = src
            .values
            .remove("disableCapture")
            .is_some_and(|v| truthy(&v));
        let allow_anyway = src
            .values
            .remove("allowWhenSystemHistoryDisabled")
            .is_some_and(|v| truthy(&v));
        let notice = if disable {
            Some("Your organization has turned off clipboard capture on this device.".into())
        } else if src.os_history_disabled && !allow_anyway {
            Some(src.os_history_notice)
        } else {
            None
        };
        Policy {
            values: src.values,
            capture_disabled: notice.is_some(),
            notice,
        }
    }

    /// Overrides the user's settings with the administrator's. Unknown keys
    /// and values of the wrong type are ignored.
    pub fn apply(&self, settings: Settings) -> Settings {
        if self.values.is_empty() {
            return settings;
        }
        let Ok(mut json) = serde_json::to_value(&settings) else {
            return settings;
        };
        for (key, value) in &self.values {
            let Some(current) = json.get(key) else {
                continue;
            };
            let mut trial = json.clone();
            trial[key] = coerce(current, value);
            if serde_json::from_value::<Settings>(trial.clone()).is_ok() {
                json = trial;
            }
        }
        serde_json::from_value::<Settings>(json)
            .unwrap_or(settings)
            .sanitized()
    }

    /// Setting names the user cannot change.
    pub fn managed_keys(&self) -> Vec<String> {
        let known = serde_json::to_value(Settings::default()).unwrap_or_default();
        self.values
            .keys()
            .filter(|k| known.get(k.as_str()).is_some())
            .cloned()
            .collect()
    }

    pub fn manages(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn source(values: Value) -> PolicySource {
        PolicySource {
            values: values.as_object().unwrap().clone(),
            ..Default::default()
        }
    }

    #[test]
    fn registry_style_values_override_settings() {
        let policy = Policy::from_source(source(json!({
            "encrypt": 1,
            "historyLimit": "30",
            "blocklist": "a.exe;b.exe",
            "notASetting": 5,
            "maxDiskMb": "lots"
        })));
        let s = policy.apply(Settings::default());
        assert!(s.encrypt || !crate::platform::encryption_supported());
        assert_eq!(s.history_limit, 30);
        assert_eq!(s.blocklist, vec!["a.exe", "b.exe"]);
        assert_eq!(s.max_disk_mb, Settings::default().max_disk_mb);
        let mut managed = policy.managed_keys();
        managed.sort();
        assert_eq!(
            managed,
            vec!["blocklist", "encrypt", "historyLimit", "maxDiskMb"]
        );
        assert!(!policy.capture_disabled);
    }

    #[test]
    fn os_history_policy_is_respected_unless_allowed() {
        let mut src = source(json!({}));
        src.os_history_disabled = true;
        src.os_history_notice = "off".into();
        assert!(Policy::from_source(src).capture_disabled);

        let mut src = source(json!({ "allowWhenSystemHistoryDisabled": 1 }));
        src.os_history_disabled = true;
        assert!(!Policy::from_source(src).capture_disabled);

        assert!(Policy::from_source(source(json!({ "disableCapture": true }))).capture_disabled);
    }
}
