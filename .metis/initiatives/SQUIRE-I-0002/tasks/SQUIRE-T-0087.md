---
id: server-startup-pull-fetch-latest
level: task
title: "Server startup-pull: fetch latest APK from squire into the OTA updates dir"
short_code: "SQUIRE-T-0087"
created_at: 2026-06-19T17:49:00.945627+00:00
updated_at: 2026-06-19T17:49:00.945627+00:00
parent: SQUIRE-I-0002
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: SQUIRE-I-0002
---

# Server startup-pull: fetch latest APK from squire into the OTA updates dir

## Parent Initiative

[[SQUIRE-I-0002]] — implements [[SQUIRE-A-0012]].

## Objective

On `squire-serve` startup (and on a refresh interval), fetch the latest phone APK from the public
`colliery-io/squire` Releases into the OTA updates dir, and write `manifest.json` so the existing
LAN serving (`/app/*` + server-injected sha256) picks it up. Removes the last manual publish step.

## Design

- `squire` is **public**, so reads need no auth: `GET api.github.com/repos/colliery-io/squire/releases/latest`,
  find the asset named `squire-<N>.apk`, download it.
- Derive `versionCode` from the asset name (`squire-<N>.apk`), `versionName` from the release tag
  (`vX.Y.Z` → `X.Y.Z`).
- Write `<updates_dir>/squire-<N>.apk` + `manifest.json` (`{squire:{versionCode,versionName,file}}`).
  The server already injects the sha256 ([[SQUIRE-T-0085]]).
- **Offline-tolerant & non-blocking**: run in a spawned task; any failure (no network, no release,
  parse error) logs and leaves whatever is already present. Never delays or crashes startup.
- **Idempotent**: skip the download if `squire-<N>.apk` already exists.
- Config: `SQUIRE_DIST_REPO` (default `colliery-io/squire`), `SQUIRE_APK_SYNC` (`off` to disable),
  refresh interval (default a few hours).

## Acceptance Criteria

- [ ] On startup, `squire-serve` fetches the latest `squire` APK into the updates dir + writes the manifest.
- [ ] No release / offline → logged, server still serves existing artifacts, no crash.
- [ ] Already-present version → no re-download.
- [ ] `/app/manifest` then advertises the pulled version with the injected sha256.

## Status Updates

- Implemented: `apk_sync` module (reqwest/rustls, serde_json) fetches the latest `colliery-io/squire`
  release, downloads the `squire-<N>.apk` asset into the updates dir (atomic temp+rename, idempotent),
  and writes the manifest. Spawned from `squire-serve` as a non-blocking 6h refresh loop;
  `SQUIRE_APK_SYNC=off` / `SQUIRE_DIST_REPO` configurable.
- Verified: unit test (asset-name parsing) green; graceful path smoke-tested — no release yet →
  logs `HTTP 404`, server stays healthy (`/health` + `/app/manifest` 200).
- PENDING happy-path: needs a real Release in `colliery-io/squire` (via CI on a tag, or a one-off seed)
  to confirm the actual pull + manifest + served sha256. Task stays active until that's checked.
