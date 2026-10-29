# Privacy

ClipDeck is designed so that your clipboard never leaves your PC.

## What ClipDeck stores

- Items you copy (text, rich text/HTML, RTF, images and file lists),
  the name of the app they came from, and when they were copied.
- Image thumbnails generated from copied images.
- Your settings.

Everything is stored locally in `%APPDATA%\ClipDeck\`:

| Path | Contents |
| --- | --- |
| `clipdeck.db` | SQLite metadata and short text previews used for search |
| `blobs\` | The full copied data, one file per item |
| `thumbs\` | Thumbnails of image items |
| `settings.json` | Your settings |

The dashboard's web view also keeps its normal browser cache under
`%LOCALAPPDATA%\com.tanmayangarkar.clipdeck\`. It contains no clipboard data
beyond what is shown on screen.

## What ClipDeck does not do

- **No network access.** ClipDeck makes no network connections: no
  telemetry, analytics, crash reporting, update checks or cloud sync. The
  dashboard's content security policy blocks all remote content.
- No accounts and no third-party services.

## Protecting sensitive data

- Copies that apps mark as private are never captured. ClipDeck honours the
  Windows clipboard formats `ExcludeClipboardContentFromMonitorProcessing`,
  `CanIncludeInClipboardHistory` (value 0) and `Clipboard Viewer Ignore`,
  which password managers set.
- A per-app blocklist (Settings → Privacy) skips copies from listed
  executables. It ships with common password managers.
- "Pause capture" in the tray or dashboard stops recording entirely.
- "Encrypt history on disk" protects stored items and thumbnails with
  Windows DPAPI, so only your Windows account can read them.
- "Clear history on exit and sign-out" deletes unpinned items when
  ClipDeck quits or you sign out of Windows.
- Deleted items are removed from disk, and the database runs with
  `secure_delete` so deleted previews are overwritten.

**Warning:** a clipboard history may contain passwords, personal messages
or other sensitive data you copied. Anyone who can sign in to your Windows
account can open ClipDeck and see it. Use the privacy options above, pin
only what you need, and clear the history when sharing a PC.

## Removing all data

Quit ClipDeck from the tray and delete `%APPDATA%\ClipDeck\`. Uninstalling
does not delete your history automatically.
