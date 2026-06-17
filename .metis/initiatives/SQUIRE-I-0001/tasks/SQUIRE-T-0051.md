---
id: server-distributed-app-updates
level: task
title: "Server-distributed app updates: version manifest + APK endpoint + in-app update prompt"
short_code: "SQUIRE-T-0051"
created_at: 2026-06-17T22:30:00+00:00
updated_at: 2026-06-17T22:22:31.204952+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Server-distributed app updates: version manifest + APK endpoint + in-app update prompt

## Parent Initiative

[[SQUIRE-I-0001]] · Spec [[SQUIRE-S-0003]] (Local API) · follow-up of [[SQUIRE-T-0049]] (signed releases) · the "how do phones upgrade" gap

## Objective

Today there is **no upgrade path** — you rebuild an APK and reinstall by hand. Let the LAN server
distribute updates: it hosts the latest signed APKs + a version manifest; the apps check on launch
and, when a newer build is available, prompt the user to get it. (Android won't let a sideloaded app
update **silently** — the user taps through the system installer — so the UX is "Update available →
Get update", not invisible.)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] Server (`api::app_dist`, LAN-facing): `GET /app/manifest` returns the JSON from `SQUIRE_APK_DIR/manifest.json`; `GET /app/{file}` streams the APK as `application/vnd.android.package-archive`. Unauthenticated. Verified: unset dir → `{}` (200, no 500); non-`.apk` → 400; missing → 404; traversal-safe (only a bare `<name>.apk` directly in the dir).
- [x] Both apps check `/app/manifest` once per session (over the session host), compare `BuildConfig.VERSION_CODE`, and show a non-blocking **"Update available — vX → Get update"** banner when the server's is higher (shared `UpdateChecker`/`UpdateBanner` in `:pairing`, rendered above the home in each `…HomeHost`). "Get update" fires `ACTION_VIEW` to `http://<host>:<port>/app/<file>` → the browser downloads, the system installer takes over. **Verified live**: manifest v5 vs installed v1 → banner shown → "Get update" opened Chrome to the APK URL.
- [x] Non-destructive: same signing key ([[SQUIRE-T-0049]]) + higher `versionCode` installs over the old app keeping the session + Room data. Admin workflow (drop APKs + `manifest.json` into `SQUIRE_APK_DIR`) documented in `clients/RUNBOOK.md`.
- [x] `cargo test --workspace` green (48 groups); both apps assemble. Update banner verified on the emulator (manifest v5 → banner → browser opens the download).

## Implementation Notes

### Technical Approach
Server: a small axum module serving two routes from `SQUIRE_APK_DIR` — `manifest` reads `<dir>/manifest.json` (admin-maintained; the admin who builds the release fills in versionCode/Name/file), and `<file>` reads `<dir>/<file>` but **only** filenames listed in the manifest (no arbitrary paths). Wire into the api router. App: an `UpdateChecker` (in `:pairing`, shared) fetches `/app/manifest` over the session host, picks its own entry (squire/knight), compares `BuildConfig.VERSION_CODE`; a banner in the home screen with an `Intent.ACTION_VIEW` to the download URL (browser handles download + install — avoids `REQUEST_INSTALL_PACKAGES`/FileProvider for v1).

### Dependencies
[[SQUIRE-T-0049]] (signed releases, stable key so updates install over the old app), [[SQUIRE-T-0048]] (the server hosting them).

### Risk Considerations
Android can't silently update sideloaded apps — the browser-download + system-installer path is the simplest honest UX (no extra permission). The new APK MUST be signed with the same key and a higher `versionCode`, or the install is rejected. Keep `/app/*` path-traversal-safe. A fancier in-app download + `PackageInstaller` flow (one-tap, needs `REQUEST_INSTALL_PACKAGES`) is a possible follow-up. Out of scope: signing/verification of the manifest itself (LAN-trusted), staged rollouts, deltas.

## Status Updates

**2026-06-17 — Done.** Server: `crates/api/src/app_dist.rs` (`GET /app/manifest` reads
`SQUIRE_APK_DIR/manifest.json` or returns `{}`; `GET /app/{file}` streams a bare `<name>.apk` from
the dir, traversal-safe), wired unauthenticated into the api router. Verified by curl (manifest,
APK content-type + bytes, 400/404 safety, empty-when-unset). Apps: `:pairing/UpdateChecker.kt`
(`UpdateChecker.check(baseUrl, "squire"|"knight", BuildConfig.VERSION_CODE)` — okhttp + kotlinx,
never throws) + `UpdateBanner` composable; both `…HomeHost`s do a once-per-session check and render
the banner above the home, with "Get update" → `ACTION_VIEW(downloadUrl)`.

Caught a gotcha mid-test: a binary that *links* the api (squire-home/squire-serve) must be rebuilt
after an api change — `cargo build -p api` alone left a stale server without the `/app` routes.
**Live**: ran squire-home with `SQUIRE_APK_DIR` (manifest v5), paired the debug Squire → the
"Update available — v0.5.0 / Get update" banner showed above the home; tapping it opened Chrome to
the APK URL. `cargo test --workspace` green; both apps assemble. RUNBOOK updated with the push-an-
update workflow. (Browser-download UX chosen to avoid `REQUEST_INSTALL_PACKAGES`/FileProvider; a
one-tap in-app `PackageInstaller` flow is a possible follow-up.)