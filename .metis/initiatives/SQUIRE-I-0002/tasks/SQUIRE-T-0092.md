---
id: native-desktop-app-tauri-window
level: task
title: "Native desktop app: Tauri window hosting the Keep instead of a browser"
short_code: "SQUIRE-T-0092"
created_at: 2026-06-19T20:28:58.503865+00:00
updated_at: 2026-06-19T20:28:58.503865+00:00
parent: SQUIRE-I-0002
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: SQUIRE-I-0002
---

# Native desktop app: Tauri window hosting the Keep instead of a browser

## Parent Initiative

[[SQUIRE-I-0002]] — make the operator UI a native app, not a browser tab.

## Objective

Run the Keep in a native OS window (Tauri v2, WKWebView/WebView2/WebKitGTK) instead of opening a
browser — reusing the existing Keep web UI as-is.

## What shipped (first cut)

- **Refactor**: lifted the server startup from the `squire-serve` bin into `squire_home::run_home_server()`
  so the headless binary and the desktop app run an identical server (the bin is now a thin
  self-update + runtime wrapper).
- **`crates/squire-desktop`** (Tauri v2): embeds the home server on a background tokio runtime and
  opens a native window at the loopback Keep once it's listening. Loads the external loopback URL
  (`WebviewUrl::External`) so the whole Keep — UI, `/api`, cookies — is served by the embedded server
  (no CORS/origin split, no bundled-frontend rewrite). Crest icon set generated from the SVG.

## Acceptance Criteria

- [x] A Tauri app embeds the server + shows the Keep in a native window (no browser).
- [x] Builds and launches; the embedded Keep serves (`/health` 200) and the window points at it.
- [x] Bundled `Squire.app` (`cargo tauri build`), published to the dist Release, and `install.sh`
  installs the native app on macOS (verified: one-liner installs a real Mach-O `.app`).
- [ ] Desktop-app self-update + window polish (menus, single-instance, quit stops the server);
  cross-platform Tauri builds in CI (Intel mac / Windows / Linux native apps).

## Follow-ups

- `cargo tauri build` → `Squire.app`/`.dmg`; publish to the dist repo; make `install.sh` install the
  native app instead of the browser-launcher `.app`.
- CI: build/publish `squire-desktop` per platform; give it its own self-update (like squire-serve).
- Polish: app menu, quit-stops-embedded-server, single-instance guard.

## Status Updates

- First cut done + verified: `cargo build -p squire-desktop` green; launched with throwaway
  ports/data — process + embedded server up, `/health` 200, native window created. Bundled
  `Squire.app`, published, and the installer installed it.
- **BACKED OUT (operator decision).** Removed the `crates/squire-desktop` Tauri crate + its workspace
  entry, reverted `install.sh` to the browser-based launcher, and deleted the `Squire.app.zip` asset
  from the v0.7.3 release. The desktop UI is back to the browser-hosted Keep. **Kept** the
  `run_home_server` refactor (a clean, Tauri-independent improvement to `squire-serve`). The Tauri
  work is recoverable from git history (commits `2218867`/`75e09a8`) if revisited. Closely tied to the
  background-server blocker in [[SQUIRE-T-0093]] (launchd needs code-signing).
