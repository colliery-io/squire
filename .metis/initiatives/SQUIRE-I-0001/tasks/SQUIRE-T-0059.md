---
id: in-app-update-install
level: task
title: "In-app update install (PackageInstaller) — no browser bounce"
short_code: "SQUIRE-T-0059"
created_at: 2026-06-18T11:35:15.846742+00:00
updated_at: 2026-06-18T11:55:10.958875+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# In-app update install (PackageInstaller) — no browser bounce

## Parent Initiative

[[SQUIRE-I-0001]] · Follow-up to the OTA distribution ([[SQUIRE-T-0051]]). User pulled the v0.5.0 polish build successfully but the "Get update" action bounced them through a **browser download** — they want it to install **in-app**.

## Objective

Replace the browser-`ACTION_VIEW` update action with an **in-app download + install**: tap "Get update" → the app downloads the APK itself (with progress) → the system install confirmation appears directly (no browser, no Downloads app). Uses Android's `PackageInstaller` session API.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] `AppUpdater` (`:pairing`) downloads the APK from the LAN URL (OkHttp) straight into a `PackageInstaller` session and commits it; the system install prompt is raised via a `PendingIntent` → `InstallReceiver` (handles `STATUS_PENDING_USER_ACTION`). No browser, no `ACTION_VIEW`.
- [x] `REQUEST_INSTALL_PACKAGES` permission added; `InstallReceiver` declared (not exported). First time, the user grants "install unknown apps" for **Squire itself** (one-time), not the browser.
- [x] `UpdateBanner` drives the flow with state: idle ("Get update") → "Downloading… N%" → hands off to the installer; failure shows a retryable message. Both the Squire and Knight hosts call it (no more `Intent(ACTION_VIEW)`).
- [x] `:app:assembleDebug` + `:app:assembleRelease` build. Verified on a device/emulator that the flow downloads and reaches the system installer without a browser.

## Implementation Notes

### Technical Approach
`PackageInstaller.Session` (`MODE_FULL_INSTALL`): `createSession` → `openWrite` (stream OkHttp body, `setStaged`/size from `Content-Length`) → `commit(pendingIntent.intentSender)`. A manifest-declared `InstallReceiver : BroadcastReceiver` receives `EXTRA_STATUS`; on `STATUS_PENDING_USER_ACTION` it launches `EXTRA_INTENT` (`FLAG_ACTIVITY_NEW_TASK`) — the system "update Squire?" dialog. Progress reported by counting bytes copied vs `Content-Length`. Keep it in `:pairing` (has OkHttp + Context); wire the two hosts' banner callbacks.

### Dependencies
[[SQUIRE-T-0051]] (manifest + download URL), [[SQUIRE-T-0054]] (merged app key "squire").

### Risk Considerations
Self-update still needs the same signing key (release→release) — unchanged from T-0051. `REQUEST_INSTALL_PACKAGES` is a normal permission but gated by a per-app user grant the first time. `FLAG_MUTABLE` required on the commit `PendingIntent` (API 31+). No silent install (not a device owner) — the system confirmation is expected and correct.

## Status Updates

**2026-06-18 — Done + verified end-to-end on the emulator.** Built the in-app updater: `AppUpdater` (`:pairing`) streams the LAN APK (OkHttp) straight into a `PackageInstaller.Session` (`MODE_FULL_INSTALL`, size from `Content-Length`), commits with a `PendingIntent` (`FLAG_MUTABLE` on API 31+) to a manifest-declared, non-exported `InstallReceiver` that raises the system install dialog on `STATUS_PENDING_USER_ACTION`. `UpdateBanner` is now self-contained and stateful (idle "Get update" → "Downloading… N%" + spinner → installer; "Retry" on failure); both hosts call `UpdateBanner(info)` (removed the `Intent(ACTION_VIEW)` browser bounce + now-unused imports). Added `REQUEST_INSTALL_PACKAGES` + the receiver to the manifest.

**Live test (emulator):** installed the updater-equipped build, server advertised a newer build, tapped **Get update** → in-app progress to **61%** → system `PackageInstallerActivity` (`CONFIRM_INSTALL`) with the **Squire** crest as the source (no browser) → one-time "Install unknown apps" grant for Squire → **"Do you want to update this app?" → Update** → installed (`versionCode` advanced 2→3, confirmed via `dumpsys package`). Captured the whole sequence.

**Shipped the fix:** bumped to **versionCode 3 / v0.6.0**, built the signed release (CN=Squire), staged `squire-3.apk` + `manifest.json` into the live `SQUIRE_APK_DIR` — the running server serves it immediately (manifest read per-request, no restart). Note: the v0.5.0 the user already has predates this, so that one *next* pull is a final browser bounce; v0.6.0 onward installs in-app.

`:app:assembleDebug` + `:app:assembleRelease` both green.