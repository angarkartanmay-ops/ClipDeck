# Architecture

ClipDeck is a single Windows process built with Rust and Tauri 2. It has two
logical parts:

1. **Always-on core** (Rust, no web view): clipboard listener, hotkeys,
   storage, tray icon and autostart. Idle memory is about 2 MB private.
2. **On-demand dashboard** (WebView2): created when the hotkey is pressed and
   destroyed when it closes, so idle memory does not include a browser engine.

```
          ┌──────────────── clipdeck.exe ────────────────────────────────┐
          │                                                               │
 Windows  │  core thread (hidden window)        worker thread             │
 clipboard├─► WM_CLIPBOARDUPDATE ── read ──────► hash / dedupe            │
 hotkeys ─┼─► WM_HOTKEY ───┐       formats      store (SQLite + files)    │
 sign-out─┼─► WM_ENDSESSION│                    thumbnail (image crate)   │
          │                │                    evict over caps           │
          │                ▼                         │ events             │
          │  actions: open dashboard / paste slot / queue ◄──────────────┤
          │                │                                              │
          │  main thread (Tauri event loop): tray menu, dashboard window   │
          │                │ create on demand, destroy on close           │
          └────────────────┼──────────────────────────────────────────────┘
                           ▼
                 WebView2 dashboard (vanilla TypeScript + CSS)
                 invoke() commands  ◄──►  events: history/status/settings
```

## Source layout (`src-tauri/src`)

| Module | Responsibility |
| --- | --- |
| `main.rs` | Single-instance check, setup, run loop (keeps running with no windows) |
| `core_window.rs` | Hidden top-level window on its own thread: `AddClipboardFormatListener`, `RegisterHotKey`, `WM_ENDSESSION`, single-instance signalling |
| `clip.rs` | Reading/writing the clipboard, opt-out flags, format analysis, item serialization |
| `worker.rs` | Capture pipeline off the clipboard thread |
| `storage.rs` | SQLite metadata, payload and thumbnail files, eviction, DPAPI re-encryption |
| `thumbs.rs` | PNG/DIB decoding and thumbnail generation |
| `paste.rs` | Foreground restore (`SetForegroundWindow` + `AttachThreadInput`) and `SendInput` Ctrl+V |
| `dashboard.rs` | Dashboard window lifecycle and placement (cursor/centre, per-monitor DPI) |
| `hotkeys.rs` | Hotkey parsing and registration |
| `actions.rs` | Operations shared by hotkeys, tray and UI; settings application |
| `commands.rs` | Tauri commands for the dashboard |
| `tray.rs`, `autostart.rs`, `settings.rs`, `crypto.rs`, `source_app.rs`, `paths.rs`, `state.rs` | Supporting pieces |

The frontend (`src/`) is plain TypeScript bundled by Vite (~11 KB gzipped JS).
`src/mock.ts` lets the dashboard run in an ordinary browser (`npm run dev`)
with sample data for UI work.

## Capture

- Event driven: `AddClipboardFormatListener`, no polling.
- Changes made by ClipDeck itself are ignored (clipboard owner is the core
  window, or the sequence number matches ClipDeck's last write).
- Skipped when capture is paused, the source executable is blocklisted, or
  the source sets `ExcludeClipboardContentFromMonitorProcessing`,
  `Clipboard Viewer Ignore`, or `CanIncludeInClipboardHistory = 0`.
- Formats kept: `CF_UNICODETEXT`, `HTML Format`, `Rich Text Format`, `PNG`,
  `CF_DIB`, `CF_HDROP` (+ `Preferred DropEffect`). Windows synthesises the
  rest (e.g. `CF_BITMAP`, `CF_DIBV5`) when pasting.
- `OpenClipboard` is retried for ~300 ms when another app holds the clipboard.
- The clipboard thread only copies bytes; hashing, disk writes and thumbnails
  happen on the worker thread, so copying a large image never blocks it.

## Storage (`%APPDATA%\ClipDeck`)

- `clipdeck.db`: one row per item: kind, time, source app, pinned, size,
  content hash, a text preview (≤ 8000 chars, for search) and JSON metadata.
  `PRAGMA secure_delete` is on and the page cache is limited to 512 KB.
- `blobs/<id>.bin`: the item's formats (`CDK1` container), never held in RAM
  after capture.
- `thumbs/<id>.png`: 480×300 max thumbnails for image items.
- With encryption on, previews, blobs and thumbnails are DPAPI-protected
  (current user); toggling re-writes existing items in the background.
- De-duplication: a repeat of the newest item is ignored; copying an older
  item again moves it to the top (keeping its pin).
- Eviction after each capture: oldest unpinned items beyond the item cap,
  then beyond the disk cap. Pinned items are never evicted.

## Pasting

1. The hotkey handler records the foreground window (unless it is ClipDeck).
2. On selection the item is written to the clipboard and the dashboard closes.
3. ClipDeck brings the recorded window back to the foreground, waits for the
   user to release Shift/Alt/Win, and sends Ctrl+V with `SendInput`.
4. Optionally the previous clipboard content is restored ~500 ms later.

Elevated target windows reject synthetic input from non-elevated processes
(UIPI); the item is still on the clipboard in that case.

## Memory and latency

| Measurement (Windows 11, release build, Task Manager "private working set") | Value |
| --- | --- |
| Core, idle | ~2 MB (15 MB total working set), 0% CPU |
| Dashboard open, core + all WebView2 processes | ~120–170 MB (mostly the WebView2 GPU process) |
| After closing the dashboard | ~4 MB; WebView2 processes exit |
| Second launch / hotkey to visible dashboard, cold | ~300–450 ms |

The cold-open time is dominated by WebView2 start-up. "Keep dashboard ready"
in Settings hides the window instead of destroying it: opening becomes
near-instant at the cost of keeping the web view (~120 MB) in memory.
WebView2 is started with extra switches that limit helper processes and
background work (see `BROWSER_ARGS` in `dashboard.rs`).

## Porting

Windows-specific code is confined to `core_window.rs`, `clip.rs`, `paste.rs`,
`source_app.rs`, `autostart.rs`, `crypto.rs` and the placement helper in
`dashboard.rs`. Storage, the worker, settings and the whole frontend are
platform-neutral.
