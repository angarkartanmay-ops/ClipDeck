# Testing

## Automated

```bash
cd src-tauri
cargo test                 # serialization, hashing, DROPFILES and uri-list parsing, hotkey parsing,
                           # policy coercion, storage CRUD, eviction and retention (pinned kept),
                           # DPAPI round trip (Windows), cipher round trip (macOS), DIB thumbnails
cargo clippy --all-targets -- -D warnings
cd .. && npx tsc --noEmit
```

CI (`.github/workflows/build.yml`) runs these on Windows, macOS and Linux,
plus `cargo deny` for every target and the npm license check. The macOS
Keychain itself is not touched by unit tests (they would write to the
developer's real login Keychain); check it manually below.

## Manual matrix

Record results per release and platform. Mark ✅ / ❌ / n/a.

### Paste targets (text, rich text, image, files)

| Target | Text | Rich | Image | Files |
| --- | --- | --- | --- | --- |
| Windows: Notepad | | n/a | n/a | n/a |
| Windows: Word | | | | n/a |
| Windows: Paint | n/a | n/a | | n/a |
| Windows: File Explorer | n/a | n/a | n/a | |
| macOS: TextEdit | | | | n/a |
| macOS: Pages or Word | | | | n/a |
| macOS: Preview (new from clipboard) | n/a | n/a | | n/a |
| macOS: Finder | n/a | n/a | n/a | |
| Linux: gedit/Kate | | n/a | n/a | n/a |
| Linux: LibreOffice Writer | | | | n/a |
| Linux: Files/Dolphin | n/a | n/a | n/a | |
| Chrome (text field, Google Docs) | | | | n/a |
| VS Code | | n/a | n/a | n/a |
| Slack or Discord | | | | |

For each: deck paste (Enter and click), Shift+Enter (plain), slot hotkey for
item 1, paste queue (3 items in order). On Linux, "paste" means the clip is
on the clipboard and Ctrl+V pastes it.

### Image sources

Windows: Snipping Tool, Win+Shift+S. macOS: ⌃⇧⌘4 (copy screenshot), Preview
"Copy". Linux: GNOME/KDE screenshot to clipboard. Everywhere: Chrome "Copy
image", Figma. Check: thumbnail correct, dimensions shown, pastes at full
resolution, transparency preserved where the source provided PNG.

### Privacy

- Copy a password from KeePassXC, Bitwarden and 1Password: nothing captured
  (each platform).
- Add the current editor to the blocklist (`notepad.exe`,
  `com.apple.TextEdit`, `gedit`): its copies are skipped.
- Pause capture from the tray/menu bar: nothing captured until resumed.
- Turn encryption on (Windows, macOS): files in `blobs/` and `thumbs/` are
  unreadable; items still display and paste. macOS: a
  `com.tanmayangarkar.clipdeck` item appears in Keychain Access; after
  quitting and reopening ClipDeck, items still display without a prompt.
- Clear on exit: quit from the tray/menu bar and sign out; unpinned items are
  gone, pinned kept.

### macOS permissions

- Fresh install: the Accessibility notice appears; **Allow** opens Privacy &
  Security › Accessibility. Before allowing, choosing a clip copies it; after
  allowing, it pastes into the previous app.
- If the system asks about pasting from other apps, choose **Don't Allow**:
  ClipDeck shows the "Paste from Other Apps" notice and records nothing. Set
  it to Allow: recording resumes.
- Opening ClipDeck.app again from Finder while it runs toggles the deck.
- "Open at Login" on: a LaunchAgent exists and ClipDeck starts after
  restarting, without opening the deck.

### Policy

- Windows: import the ADMX/ADML into the local policy store (or set registry
  values under `HKLM\SOFTWARE\Policies\ClipDeck`), set `encrypt`,
  `retentionDays` = 1 and a blocklist. Restart ClipDeck: those settings show
  **Managed** and cannot be changed; existing history is encrypted; clips
  older than a day (pinned too) are removed.
- Windows: set `HKLM\SOFTWARE\Policies\Microsoft\Windows\System\AllowClipboardHistory`
  = 0: ClipDeck shows the notice and records nothing; add
  `allowWhenSystemHistoryDisabled` = 1: recording resumes.
- macOS: install `policy/macos/ClipDeck.mobileconfig` (settings payload);
  the same checks.
- Linux: create `/etc/clipdeck/policy.json`; the same checks.
- `disableCapture` = 1 on any platform: nothing is recorded and the deck says
  so.

### Resources

- Windows: Task Manager → Details, add "Memory (private working set)": core
  under 20 MB after several hours idle; ~0% CPU. Deck open: under ~150 MB
  total including `msedgewebview2.exe` children; after closing, those
  processes exit.
- macOS: Activity Monitor: ClipDeck idle CPU ~0% (the 350 ms pasteboard
  check), memory steady over hours.
- Linux: `top`: idle CPU ~0%, memory steady.

### Edge cases

- 100 MB+ image copy (over "Largest clip"): skipped without freezing.
- Rapid repeated copies (hold Ctrl+C / ⌘C in an editor): one item, no duplicates.
- Clipboard locked by another app (Windows): capture retries, no crash.
- Remote Desktop session (local and remote clipboard): capture works, no loops.
- Elevated target (admin Notepad): paste fails gracefully; item is on the clipboard.
- Hotkey already taken by another app: Settings shows a warning.
- Second launch of ClipDeck opens the running instance's deck (`clipdeck
  --toggle` on Linux).
- Multi-monitor with mixed DPI: the deck opens on the cursor's monitor at the
  right size (each platform).
- macOS: open the deck over a full-screen app and on another Space.
- Linux Wayland session: bind `clipdeck --toggle` to a key; the deck opens
  and copies clips from Wayland apps.
