<p align="center">
  <img src="src-tauri/icons/128x128@2x.png" width="96" alt="ClipDeck logo" />
</p>

<h1 align="center">ClipDeck</h1>

<p align="center">
  A lightweight, always-on clipboard manager for Windows.<br/>
  Copy many things, then press <b>Ctrl+Shift+V</b> and pick what to paste.
</p>

<p align="center">
  <b>Source-available, free for non-commercial use.</b>
  &nbsp;·&nbsp; <a href="LICENSE">PolyForm Noncommercial 1.0.0</a>
  &nbsp;·&nbsp; <a href="COMMERCIAL-LICENSE.md">Commercial licensing</a>
</p>

---

## Why

- Copying something new silently overwrites what you copied before.
- Windows' built-in history (Win+V) keeps about 25 items, drops items larger
  than ~4 MB, forgets everything on restart unless pinned, and has no search
  or paste queue.
- Screenshots and images copied from several places are the worst case.

ClipDeck keeps a local history of text, rich text, images and files and gives
you a fast, keyboard-friendly dashboard to paste any of it.

## Features

- **Captures everything you copy**: plain text, rich text (HTML/RTF), images
  (PNG and DIB) and file lists, with the original formats preserved so rich
  text pastes as rich text and images paste as images.
- **Floating dashboard** on <kbd>Ctrl+Shift+V</kbd>: thumbnails, source app,
  timestamps, fuzzy search, filters, pins, smooth animations, dark and light
  themes, accent colours, and support for "reduce motion".
- **Pastes into the app you were using**: pick an item and ClipDeck returns
  focus to the previous window and pastes it for you.
- **Slot hotkeys**: <kbd>Ctrl+Alt+1</kbd>…<kbd>9</kbd> paste recent items
  directly, without opening the dashboard.
- **Paste queue**: <kbd>Ctrl+Shift+Q</kbd> turns queue mode on. Copy several
  things, then each <kbd>Ctrl+Shift+V</kbd> pastes the next one in order.
- **Plain Ctrl+C / Ctrl+V are untouched.** ClipDeck only listens for
  clipboard changes and registers its own hotkeys; it never hooks the keyboard.
- **Privacy first**: no network access at all, honours the "don't record
  this" flags used by password managers, per-app blocklist, pause capture,
  optional DPAPI encryption at rest, optional clear on exit/sign-out.
  See [PRIVACY.md](PRIVACY.md).
- **Light**: the background process uses a few MB of memory and no CPU
  while idle. The dashboard's web view only exists while it is open.
- Starts with Windows, lives in the system tray, one instance per user.

## Install

Download the latest installer (`.exe` or `.msi`) or the portable `.zip` from
[Releases](https://github.com/angarkartanmay-ops/ClipDeck/releases). Each
release lists SHA-256 checksums in `SHA256SUMS.txt`; verify with:

```powershell
Get-FileHash .\ClipDeck_0.1.0_x64-setup.exe -Algorithm SHA256
```

Requirements: Windows 10 (1809+) or Windows 11, x64, and the Microsoft Edge
WebView2 Runtime (included with Windows 11; the installer fetches it if missing).

> **SmartScreen:** early releases are not code-signed, so Windows may show
> "Windows protected your PC". Choose **More info → Run anyway** if you trust
> the download and its checksum matches.

## Using ClipDeck

| Where | Key | Action |
| --- | --- | --- |
| Anywhere | <kbd>Ctrl+Shift+V</kbd> | Open/close the dashboard (or paste next queued item) |
| Anywhere | <kbd>Ctrl+Alt+1</kbd>…<kbd>9</kbd> | Paste history item 1–9 |
| Anywhere | <kbd>Ctrl+Shift+Q</kbd> | Toggle paste queue mode |
| Dashboard | <kbd>↑ ↓ ← →</kbd> | Move |
| Dashboard | <kbd>Enter</kbd> / click | Paste |
| Dashboard | <kbd>Shift+Enter</kbd> | Paste as plain text |
| Dashboard | <kbd>Ctrl+C</kbd> | Copy only (no paste) |
| Dashboard | <kbd>1</kbd>…<kbd>9</kbd> | Paste item 1–9 in the current view |
| Dashboard | <kbd>P</kbd> / <kbd>Q</kbd> / <kbd>Del</kbd> | Pin / queue / delete |
| Dashboard | type, or <kbd>/</kbd> | Search (then use <kbd>Ctrl+P</kbd>, <kbd>Alt+1…9</kbd>, <kbd>Shift+Del</kbd>) |
| Dashboard | <kbd>Tab</kbd> | Next filter (All, Text, Images, Files, Pinned) |
| Dashboard | <kbd>Esc</kbd> | Clear search, then close |

All hotkeys can be changed in Settings (gear icon or tray menu).

History keeps 50 items by default (up to 200) within a 500 MB disk budget;
the oldest unpinned items are removed first and pinned items are never removed.

### Known limitations

- **Elevated windows:** Windows does not let a normal app send keystrokes to
  an app running as administrator. Pasting into an elevated window fails
  unless ClipDeck is also started as administrator; the item is still placed
  on the clipboard, so <kbd>Ctrl+V</kbd> works manually.
- Apps that delay-render huge clipboard data (for example very large Excel
  ranges) can make a capture take a moment; ClipDeck reads it in the
  background and skips items larger than the "Largest item" setting.
- Windows only for now. The core is split so a macOS/Linux port is possible later.

## Building from source

See [CONTRIBUTING.md](CONTRIBUTING.md) for the toolchain. In short:

```bash
npm install
npm run tauri build
```

Architecture notes are in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) and
the manual test plan in [docs/TESTING.md](docs/TESTING.md).

## License

ClipDeck is **source-available, free for non-commercial use** under the
[PolyForm Noncommercial License 1.0.0](LICENSE). You may use, study, modify
and share it for any non-commercial purpose. It is not "open source" in the
OSI sense, because commercial use is not permitted without a separate license.

### Commercial licensing

Using ClipDeck in a business, for paid work, or selling or bundling it
requires a separate written license. Contact **angarkartanmay@gmail.com**.
See [COMMERCIAL-LICENSE.md](COMMERCIAL-LICENSE.md).

The name "ClipDeck" and the logo are trademarks of the author and may not be
used by forks; see [TRADEMARKS.md](TRADEMARKS.md). Third-party components and
their licenses are listed in [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).

## Disclaimer

ClipDeck is provided **"as is", without warranty of any kind**, as stated in
the license. The author is not liable for any damage or data loss arising
from its use.

A clipboard history can contain **sensitive data** such as passwords,
personal messages or financial details that you copied. ClipDeck stores it
on your PC so you can paste it later. Review [PRIVACY.md](PRIVACY.md), use
the blocklist, pause and encryption options, and clear the history before
sharing your Windows account.

Copyright © 2026 Tanmay Angarkar.
