# Contributing to ClipDeck

Thanks for your interest! Bug reports, test results and ideas are very
welcome as GitHub issues.

## Contributor License Agreement (required)

ClipDeck is source-available under the PolyForm Noncommercial License, and
the author also offers paid commercial licenses. To keep that possible,
**every pull request needs a signed [Contributor License Agreement](CLA.md)**.

When you open your first pull request, the CLA Assistant bot will comment
with instructions. Reply to the pull request with:

> I have read the CLA Document and I hereby sign the CLA

The bot records your signature in this repository and re-checks the pull
request. Pull requests from contributors who have not signed cannot be merged.

In addition, sign off each commit (Developer Certificate of Origin) with
`git commit -s`, which adds a `Signed-off-by: Your Name <email>` line.

## Development setup

All platforms need [Rust](https://rustup.rs) (stable) and Node.js 20+. Then:

- **Windows 10/11:** Visual Studio Build Tools with the "Desktop development
  with C++" workload (MSVC toolchain), and the WebView2 runtime
  (preinstalled on Windows 11).
- **macOS 11+:** Xcode Command Line Tools (`xcode-select --install`).
- **Linux (Debian/Ubuntu 22.04+):**
  `sudo apt install build-essential curl wget file libwebkit2gtk-4.1-dev libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev`

```bash
npm install
npm run tauri dev        # run with hot-reloading UI
npm run tauri build      # release bundles in src-tauri/target/release/bundle
```

`npm run dev` alone serves the dashboard in a normal browser with sample
data, which is handy for UI work. Add `?platform=macos`, `?platform=linux` or
`?managed=1` to the URL to preview other platforms and policy locks.

Tests and checks:

```bash
cd src-tauri && cargo test && cargo clippy --all-targets -- -D warnings
npx tsc --noEmit
cargo deny --manifest-path src-tauri/Cargo.toml check licenses bans sources
npm run licenses
```

CI runs clippy and the tests on all three platforms, so a change made on one
OS is checked on the others before it is merged. Platform-specific code lives
in `src-tauri/src/platform/<os>/`; see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Guidelines

- Keep the background core small: no new long-lived threads, timers or
  polling (the macOS pasteboard check is the one exception, as macOS has no
  change notification), and no payloads kept in RAM.
- Never add network access, telemetry or remote content.
- New dependencies must be MIT, Apache-2.0, BSD, ISC, Zlib or similarly
  permissive (enforced by `deny.toml` and `npm run licenses`); fonts may be
  SIL OFL 1.1. No GPL, AGPL or LGPL.
- Every source file starts with:
  `Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.`
- Match the surrounding code style; run `cargo fmt`.
