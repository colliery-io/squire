---
id: deploy-accumulated-backend-to-live
level: task
title: "Deploy accumulated backend to live squire-serve + publish OTA app v0.7.0"
short_code: "SQUIRE-T-0083"
created_at: 2026-06-19T00:25:08.916911+00:00
updated_at: 2026-06-19T00:25:08.916911+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Deploy accumulated backend to live squire-serve + publish OTA app v0.7.0

## Parent Initiative

[[SQUIRE-I-0001]]

## Objective

Ship the work accumulated since v0.6.0 to the real household: rebuild + restart the live `squire-serve` (A-0011 timezone, all `/admin` authoring + member endpoints, `StateView.badges`, Keep refinements) and publish the new app as an OTA update the running phone picks up.

## What was deployed (since v0.6.0 / versionCode 3)

- **Backend**: T-0064/65/72/74/75 (`/admin/quests|achievements|items|members`), T-0079 (`StateView.badges`), the A-0011 timezone/config + LocalClock, and all Keep refinements (T-0071/73/80/82).
- **App**: native achievement + reward + member authoring, reject-reason capture, earned-badges, child-home polish → versionCode 4 / 0.7.0.

## Procedure (executed)

1. **Safety check**: SDK uses `ignoreUnknownKeys = true` → the installed v3 app tolerates the new `badges` field → backend can deploy without breaking the old app.
2. **Build**: `cargo build --release --bin squire-serve` (all api changes compiled in).
3. **Restart** (preserving state): captured the live launch env (`API_PORT=8088 SQUIRE_ADMIN_NAME=Dad SQUIRE_ADMIN_SECRET=… SQUIRE_PAIR_HOST=10.0.0.227 SQUIRE_APK_DIR=$HOME/squire-updates`), stopped the old PID, relaunched the new binary. Durable data dir (`~/Library/Application Support/squire`) + signing key untouched → `Household: home (loaded)`, paired devices keep working.
4. **Verify**: `/health` → 200; `/admin/{members,items,achievements}` → **401** (routes exist; the old binary 404'd them) → new code confirmed live; timezone America/Detroit loaded.
5. **App OTA**: bumped versionCode 3→4 / 0.6.0→0.7.0; `:app:assembleRelease` (signed with `keystore/squire-release.jks`); verified the signer SHA-256 **matches the installed v3** (9da09d14…) so PackageInstaller updates in place; copied to `~/squire-updates/squire-4.apk`; updated `manifest.json` → v4. Live server serves the v4 manifest and the APK downloads (200, 2.19 MB).

## Acceptance Criteria

- [x] Release `squire-serve` built and running on `10.0.0.227:8088` with the existing household + signing key (no wipe/reseed).
- [x] New api routes verified live (401, not 404) and `/health` 200.
- [x] App bumped to v0.7.0 / versionCode 4, signed with the release keystore (signer matches installed v3).
- [x] `~/squire-updates/squire-4.apk` + `manifest.json` published; the live server advertises v4 and serves the APK.
- [x] versionCode/Name bump committed.

## Status Updates

**2026-06-19 — Deployed.** Backend live (new endpoints + badges + timezone), v0.7.0 published to OTA. The running phone will see versionCode 4 > 3 on its next foreground update check and prompt; tapping update installs squire-4.apk in place. squire-serve runs in the background (data persisted across the restart).

## Notes / follow-ups
- `squire-serve` is a manually-launched background process (not a launchd service); a future nicety would be a launchd/login-item supervisor so it survives reboots without a manual start.
