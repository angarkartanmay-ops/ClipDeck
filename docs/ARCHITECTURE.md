# Architecture

ClipDeck is a single process per user, built with Rust and Tauri 2, for
Windows, macOS and Linux. It has two logical parts:

1. **Always-on core** (Rust, no web view): clipboard watcher, hotkeys,
   storage, tray/menu-bar icon and autostart. Idle memory on Windows is about
   2 MB private.
2. **On-demand deck** (the system web view: WebView2, WKWebView or
   WebKitGTK): created when the hotkey is pressed and destroyed when it
   closes, so idle memory does not include a browser engine.

```
          ┌──────────────── clipdeck ─────────────────────────────────────┐
          │                                                               │
  OS      │  platform layer (watch thread)       worker thread            │
 clipboard├─► change event ─── read formats ───► hash / dedupe            │
 hotkeys ─┼─► hotkey ──────┐                      store (SQLite + files)   │
 sign-out─┼─► session end  │                      thumbnail (image crate)  │
          │                │                      evict over caps / age    │
          │                ▼                          │ events             │
          │  actions: open deck / paste slot / queue ◄─────────────────────┤
          │                │                                              │
          │  main thread (Tauri event loop): tray menu, deck window        │
          │                │ create on demand, destroy on close           │
          └────────────────┼──────────────────────────────────────────────┘
                           ▼
                 Deck (vanilla TypeScript + CSS in the system web view)
                 invoke() commands  ◄──►  events: history/status/settings
```

## Source layout (`src-tauri/src`)

Platform-neutral:

| Module | Responsibility |
| --- | --- |
| `main.rs` | Single instance, policy, setup, run loop (keeps running with no windows) |
| `clip.rs` | A captured item as named formats (Windows formats, macOS UTIs, X11 targets), analysis, serialization |
| `worker.rs` | Capture pipeline off the clipboard thread |
| `storage.rs` | SQLite metadata, payload and thumbnail files, eviction, retention, encryption migration |
| `thumbs.rs` | PNG/DIB/TIFF decoding and thumbnail generation |
| `dashboard.rs` | Deck window lifecycle, full-screen/window modes, placement |
| `actions.rs` | Operations shared by hotkeys, tray and UI; status; settings application |
| `policy.rs` | Administrator policy: coercion, locking, the OS clipboard-history rule |
| `commands.rs`, `tray.rs`, `settings.rs`, `state.rs`, `paths.rs` | Supporting pieces |

The platform layer (`platform/`) gives every OS the same functions (listed in
`platform/mod.rs`): clipboard watching and writing, hotkeys, pasting into the
previous app, autostart, encryption, policy, permission notices and window
styling.

| | Windows (`platform/windows`) | macOS (`platform/macos`) | Linux (`platform/linux`) |
| --- | --- | --- | --- |
| Clipboard changes | `AddClipboardFormatListener` on a hidden window | `NSPasteboard.changeCount`, checked every 350 ms | XFixes selection events (`x11-clipboard`) |
| Private copies skipped | `ExcludeClipboardContentFromMonitorProcessing`, `CanIncludeInClipboardHistory = 0`, `Clipboard Viewer Ignore` | nspasteboard.org concealed/transient/auto-generated types | `x-kde-passwordManagerHint` |
| Source app | Clipboard owner / foreground window process | Frontmost app (name, bundle ID) | `_NET_ACTIVE_WINDOW` → `_NET_WM_PID` → `/proc` |
| Hotkeys | `RegisterHotKey` | Carbon hot keys (Tauri global-shortcut plugin) | `XGrabKey` (plugin); Wayland: `clipdeck --toggle` |
| Paste | Restore foreground window, `SendInput` Ctrl+V | Hide deck, activate previous app, `CGEvent` ⌘V (needs Accessibility) | Copy only |
| Encryption | DPAPI (current user) | Keychain key + ChaCha20-Poly1305 | Not available |
| Autostart | `HKCU\...\Run` | `~/Library/LaunchAgents` | `~/.config/autostart` |
| Policy | `SOFTWARE\Policies\ClipDeck` (HKCU, then HKLM) | Forced managed preferences | `/etc/clipdeck/policy.json` |
| Single instance | Named mutex + window message | Tauri single-instance plugin; reopening the app | Tauri single-instance plugin (D-Bus) |

The frontend (`src/`) is plain TypeScript bundled by Vite. `src/keys.ts`
holds the per-platform keyboard conventions (⌘ vs Ctrl, modifier glyphs).
`src/mock.ts` lets the deck run in an ordinary browser (`npm run dev`) with
sample data; add `?platform=macos`, `?platform=linux` or `?managed=1` to
preview other platforms and policy locks.

## Deck window

- Two modes: **full screen** (covers the monitor under the cursor, like
  Mission Control) and **window** (980×640 logical pixels by default, movable
  by its bar and resizable from any edge; position and size are remembered).
- Created hidden and shown once the page has rendered its first frame.
- **Windows:** Tauri's `Effect::Acrylic` (DWM system backdrop on Windows 11,
  accent-policy acrylic on Windows 10 1809+). Square corners in full screen,
  rounded with a shadow as a window. WebView2 starts with switches that limit
  helper processes and turn off background networking (`BROWSER_ARGS` in
  `platform/windows/window.rs`).
- **macOS:** HUD vibrancy, shown on every Space and over full-screen apps.
  Full screen floats at status-bar level above the menu bar; the window mode
  floats above normal windows. The app is a menu-bar accessory (no Dock icon).
- **Linux:** an opaque dark background (WebKitGTK has no reliable
  transparency/blur across compositors).
- Motion is transform/opacity only, with spring curves (`linear()` where the
  engine supports it, cubic-bezier otherwise). Everything collapses to
  instant changes when "reduce motion" is on (system or app setting).
- Fonts (Mona Sans, Hubot Sans, Martian Mono) are bundled locally under the
  SIL OFL 1.1.

## Capture

- Event driven on Windows and Linux; macOS has no change notification, so the
  pasteboard's change counter is compared every 350 ms (a cheap integer read).
- Changes made by ClipDeck itself are ignored.
- Skipped when capture is paused or turned off by policy, the source app is
  blocklisted, or the copy is marked private (see the table above).
- Formats kept: plain text, HTML, RTF, PNG (plus DIB on Windows, TIFF on
  macOS) and file lists. The OS synthesises other formats when pasting.
- The clipboard thread only copies bytes; hashing, disk writes and thumbnails
  happen on the worker thread, so copying a large image never blocks it.

## Storage (per-user data folder)

`%APPDATA%\ClipDeck`, `~/Library/Application Support/ClipDeck` or
`~/.local/share/clipdeck` (mode 0700 on macOS and Linux):

- `clipdeck.db`: one row per item: kind, time, source app, pinned, size,
  content hash, a text preview (≤ 8000 chars, for search) and JSON metadata.
  `PRAGMA secure_delete` is on and the page cache is limited to 512 KB.
- `blobs/<id>.bin`: the item's formats (`CDK1` container), never held in RAM
  after capture.
- `thumbs/<id>.png`: 480×300 max thumbnails for image items.
- With encryption on, previews, blobs and thumbnails are protected with the
  OS key store; toggling (or a policy change, at the next start) re-writes
  existing items in the background.
- De-duplication: a repeat of the newest item is ignored; copying an older
  item again moves it to the top (keeping its pin).
- Eviction after each capture: oldest unpinned items beyond the item cap,
  then beyond the disk cap, then older than the retention period. Pinned
  items are only removed by a retention period set by policy.

## Policy

`policy.rs` reads the platform's source once at startup. Keys are the
settings' JSON names plus `disableCapture` and
`allowWhenSystemHistoryDisabled`; values are coerced to the setting's type
(registry DWORDs to booleans, delimited strings to lists) and ignored if they
don't fit. Managed keys are reported to the deck, which shows them locked. On
Windows the OS "Allow Clipboard History" policy turns capture off unless
`allowWhenSystemHistoryDisabled` is set. See [DEPLOYMENT.md](DEPLOYMENT.md).

## Pasting

1. The hotkey handler records the app in front (unless it is ClipDeck).
2. On selection the item is written to the clipboard and the deck closes.
3. ClipDeck brings the recorded app back to the front, waits for the user to
   release the modifier keys, and sends the paste keystroke.
4. Optionally the previous clipboard content is restored ~500 ms later.

Windows: elevated target windows reject synthetic input from non-elevated
processes (UIPI). macOS: without Accessibility, steps 3–4 are skipped and the
clip stays on the clipboard. Linux: steps 3–4 are always skipped.

## Memory and latency (Windows)

| Measurement (Windows 11, release build, Task Manager "private working set") | Value |
| --- | --- |
| Core, idle | ~2 MB (15 MB total working set), 0% CPU |
| Deck open, core + all WebView2 processes | ~120–170 MB (mostly the WebView2 GPU process) |
| After closing the deck | ~4 MB; WebView2 processes exit |
| Second launch / hotkey to visible deck, cold | ~300–450 ms |

The cold-open time is dominated by web view start-up. "Keep the deck ready"
in Settings hides the window instead of destroying it: opening becomes
near-instant at the cost of keeping the web view in memory.

## Build and release

`.github/workflows/build.yml` checks licenses (`cargo deny` over every
target), runs clippy and the unit tests on Windows, macOS and Linux, then
builds: Windows x64 (NSIS, MSI, portable zip) and ARM64 (NSIS, zip), macOS
universal (`.app`, `.dmg`) and Linux x86-64 (`.deb`, `.rpm`, `.AppImage`).
Signing is applied when the repository has the secrets for it (Windows
certificate or Azure Trusted Signing; Apple Developer ID and notarization);
otherwise builds are unsigned (macOS ad-hoc). Tags `v*` create a draft
release with all files and a combined `SHA256SUMS.txt`.
