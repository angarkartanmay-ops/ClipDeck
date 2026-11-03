// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! Open at Login on macOS via a per-user LaunchAgent
//! (`~/Library/LaunchAgents`, no administrator rights needed).

use std::path::PathBuf;

const LABEL: &str = "com.tanmayangarkar.clipdeck";

fn plist_path() -> PathBuf {
    crate::paths::home()
        .join("Library/LaunchAgents")
        .join(format!("{LABEL}.plist"))
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn set(enabled: bool) -> Result<(), String> {
    let path = plist_path();
    if !enabled {
        return match std::fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                Err(format!("remove login item: {e}"))
            }
            _ => Ok(()),
        };
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{}</string>
    <string>--background</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>ProcessType</key>
  <string>Interactive</string>
</dict>
</plist>
"#,
        xml_escape(&exe.to_string_lossy())
    );
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("create LaunchAgents: {e}"))?;
    }
    std::fs::write(&path, plist).map_err(|e| format!("write login item: {e}"))
}
