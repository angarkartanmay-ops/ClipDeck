# Security

## Reporting a vulnerability

Please report security problems privately, not in a public issue, by email
to **angarkartanmay@gmail.com** with "ClipDeck security" in the subject.
Include the version, operating system and steps to reproduce.

Only the latest release receives security fixes.

## Security model

ClipDeck keeps a history of what you copy, which can include passwords and
other secrets. It is built to keep that history on the device and to touch as
little as possible.

**What ClipDeck does**

- Reads the clipboard when it changes and stores the contents in the user's
  own data folder, readable only by that user.
- Runs as the signed-in user with no elevation, service, driver or helper
  process. One instance per user.
- Registers a few global shortcuts with the operating system. It installs no
  keyboard hook and cannot see other keystrokes.
- On Windows and on macOS (with the user's Accessibility permission), sends a
  single paste keystroke to the app the user was working in after they choose
  a clip.

**What ClipDeck does not do**

- No network access of any kind: no telemetry, update checks, crash reports,
  accounts or sync. The dashboard's content security policy allows only the
  app's own bundled files, and the core makes no network calls.
- No reading of other apps' windows, files or screen contents.
- No capture of copies that apps mark as private (password managers do), nor
  of copies from apps on the blocklist.

**Protections you can turn on** (or enforce by policy, see
[docs/DEPLOYMENT.md](docs/DEPLOYMENT.md))

- Encryption at rest: Windows DPAPI, or on macOS a random key kept in the
  user's login Keychain with ChaCha20-Poly1305. This protects history from
  other users, from offline access to the disk and from backups. It cannot
  protect against malware already running as the same user, which could also
  read the live clipboard.
- Automatic deletion after a number of days, clearing on exit and sign-out,
  pausing capture, and turning capture off entirely.

**Hardening**

- Rust core; the dashboard is a local web page with a strict content security
  policy and no remote content. The page can call only ClipDeck's own
  commands and Tauri's default window and event APIs.
- On macOS the app uses the hardened runtime. On Windows the web view runs
  with background networking, component updates and sync turned off.
- Deleted items are removed from disk and the database uses `secure_delete`.

## Verifying a download

Release files are built by GitHub Actions from the tagged source
([.github/workflows/build.yml](.github/workflows/build.yml)) and listed in
`SHA256SUMS.txt` on the release page.

```powershell
Get-FileHash .\ClipDeck_0.1.1_x64-setup.exe -Algorithm SHA256    # Windows
Get-AuthenticodeSignature .\ClipDeck_0.1.1_x64-setup.exe           # Windows signer, if signed
```

```bash
shasum -a 256 ClipDeck_0.1.1_universal.dmg                         # macOS
codesign -dv --verbose=2 /Applications/ClipDeck.app                # signer and Team ID
spctl -a -vv /Applications/ClipDeck.app                            # Gatekeeper verdict
sha256sum ClipDeck_0.1.1_amd64.AppImage                            # Linux
```

A release is code-signed only when its notes say so. Unsigned builds work the
same but trigger the warnings described in
[docs/DEPLOYMENT.md](docs/DEPLOYMENT.md).
