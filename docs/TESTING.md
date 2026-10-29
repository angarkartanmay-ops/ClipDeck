# Testing

## Automated

```bash
cd src-tauri
cargo test                 # serialization, hashing, DROPFILES parsing, hotkey parsing,
                           # storage CRUD, eviction (pinned kept), DPAPI round trip, DIB thumbnails
cargo clippy --all-targets
cd .. && npx tsc --noEmit
```

CI (`.github/workflows/build.yml`) also runs `cargo deny` and the npm license check.

## Manual matrix

Record results per release. Mark ✅ / ❌ / n/a.

### Paste targets (text, rich text, image, files)

| Target | Text | Rich | Image | Files |
| --- | --- | --- | --- | --- |
| Notepad | | n/a | n/a | n/a |
| Word | | | | n/a |
| Chrome (text field, Google Docs) | | | | n/a |
| VS Code | | n/a | n/a | n/a |
| Discord | | | | |
| WhatsApp Desktop | | | | |
| Paint | n/a | n/a | | n/a |
| File Explorer | n/a | n/a | n/a | |

For each: dashboard paste (Enter and click), Shift+Enter (plain), slot hotkey
Ctrl+Alt+1, paste queue (3 items in order).

### Image sources

Snipping Tool, Win+Shift+S, Chrome/Edge "Copy image", Photoshop, Figma:
thumbnail correct, dimensions shown, pastes at full resolution, transparency
preserved where the source provided PNG.

### Privacy

- Copy from KeePassXC, Bitwarden and 1Password: nothing captured.
- Add `notepad.exe` to the blocklist: copies from Notepad are skipped.
- Pause capture from the tray: nothing captured until resumed.
- Turn encryption on: files in `blobs\` and `thumbs\` are unreadable; items still display and paste.
- Clear on exit: quit from the tray and sign out; unpinned items are gone, pinned kept.

### Resources

- Task Manager → Details, add "Memory (private working set)": core under
  20 MB after several hours idle; ~0% CPU.
- Dashboard open: under ~150 MB total including `msedgewebview2.exe` children;
  after closing, those processes exit.

### Edge cases

- 100 MB+ image copy (over "Largest item"): skipped without freezing.
- Rapid repeated copies (hold Ctrl+C in an editor): one item, no duplicates.
- Clipboard locked by another app: capture retries, no crash.
- Remote Desktop session (local and remote clipboard): capture works, no loops.
- Elevated target (admin Notepad): paste fails gracefully; item is on the clipboard.
- Hotkey already taken by another app: Settings shows a warning.
- Second launch of ClipDeck opens the running instance's dashboard.
- Multi-monitor with mixed DPI: dashboard opens on the cursor's monitor at the right size.
