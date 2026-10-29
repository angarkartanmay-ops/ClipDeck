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

Requirements: Windows 10/11, [Rust](https://rustup.rs) (stable, MSVC
toolchain), Visual Studio Build Tools with the "Desktop development with
C++" workload, Node.js 20+, and the WebView2 runtime (preinstalled on
Windows 11).

```bash
npm install
npm run tauri dev        # run with hot-reloading UI
npm run tauri build      # release installers in src-tauri/target/release/bundle
```

`npm run dev` alone serves the dashboard in a normal browser with sample
data, which is handy for UI work.

Tests and checks:

```bash
cd src-tauri && cargo test && cargo clippy --all-targets
npx tsc --noEmit
cargo deny --manifest-path src-tauri/Cargo.toml check licenses bans sources
npm run licenses
```

## Guidelines

- Keep the background core small: no new long-lived threads, timers or
  polling, and no payloads kept in RAM.
- Never add network access, telemetry or remote content.
- New dependencies must be MIT, Apache-2.0, BSD, ISC, Zlib or similarly
  permissive (enforced by `deny.toml` and `npm run licenses`); fonts may be
  SIL OFL 1.1. No GPL, AGPL or LGPL.
- Every source file starts with:
  `Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.`
- Match the surrounding code style; run `cargo fmt`.
