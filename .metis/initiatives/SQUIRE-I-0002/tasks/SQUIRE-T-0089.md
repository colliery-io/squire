---
id: cross-platform-server-binaries-in
level: task
title: "Cross-platform server binaries in CI + squire-serve self-update from the dist repo"
short_code: "SQUIRE-T-0089"
created_at: 2026-06-19T19:07:40.930426+00:00
updated_at: 2026-06-19T19:07:40.930426+00:00
parent: SQUIRE-I-0002
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: SQUIRE-I-0002
---

# Cross-platform server binaries in CI + squire-serve self-update

## Parent Initiative

[[SQUIRE-I-0002]] — "the computer app self-updates" from [[SQUIRE-A-0012]].

## Objective

Build the `squire-serve` binary for macOS/Windows/Linux in CI and publish it to the public dist
release; have the running server check for + apply a newer build of itself from there, so a household
that installed once stays current automatically.

## Approach

- **CI** (`release-server.yml`): native-runner matrix (ubuntu/macos/windows). SQLite is **bundled**
  (`libsqlite3-sys` `bundled`), so no system deps. Package each as
  `squire-serve-<version>-<target>.{tar.gz,zip}` and publish to the dist Release on the version tag.
- **Self-update** (`self_update` crate, github backend): on startup, silent + best-effort — find the
  latest release asset for this `target` newer than `CARGO_PKG_VERSION`, download, replace the current
  exe, and re-exec into it. `SQUIRE_SELF_UPDATE=off` disables; **skip in dev** (exe under `target/`).
- **Version alignment**: bump the `squire-home` crate version to track the release tag (0.1.0 → 0.7.x)
  so the self-updater compares correctly.

## Acceptance Criteria

- [ ] CI publishes `squire-serve` binaries for macOS/Windows/Linux to the dist Release on a tag.
- [ ] The running server detects a newer published version and self-replaces + re-execs (silent).
- [ ] Dev (`cargo run`) and `SQUIRE_SELF_UPDATE=off` skip self-update; offline is a no-op (best-effort).
- [ ] Server version tracks the release tag.

## Open questions

- **First install for others**: how a new household gets the server binary the *first* time (download
  page / install script). Feeds operator onboarding; out of scope for this task.

## Status Updates

- ✅ CI cross-platform server build PROVEN: `release-server.yml` matrix built squire-serve for all 4
  targets and published `squire-serve-0.7.2-<target>.{tar.gz,zip}` to the v0.7.2 Release (SQLite
  bundled → no system deps; no Windows/macOS issues).
- ✅ Self-update implemented (`updater::maybe_self_update`, `self_update` crate): silent, best-effort,
  re-execs on update; skipped for dev builds (exe under `target/`) and `SQUIRE_SELF_UPDATE=off`. Runs
  before the tokio runtime (blocking HTTP can't nest in tokio) — `squire-serve` main restructured.
  Server version aligned to the release tag (0.1.0 → 0.7.3).
- Verified locally: builds; restructured main starts + serves (`/health` 200); dev build correctly
  skips self-update (no panic). Full self-replace exercises once two real releases exist.
- Cutting v0.7.3 (APK versionCode 7 + version-aligned 0.7.3 server binaries) to publish the aligned set.
