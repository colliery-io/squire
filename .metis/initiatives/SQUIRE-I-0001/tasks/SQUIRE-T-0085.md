---
id: ota-updates-migrate-version-based
level: task
title: "OTA updates: migrate version-based detection to content-hash (two-phase rollout)"
short_code: "SQUIRE-T-0085"
created_at: 2026-06-19T13:47:51.682715+00:00
updated_at: 2026-06-19T13:47:51.682715+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# OTA updates: migrate version-based detection to content-hash (two-phase rollout)

## Parent Initiative

[[SQUIRE-I-0001]]

## Objective

Make OTA update detection content-based (SHA-256 of the APK) instead of `versionCode`-based, so a
rebuilt-but-same-version APK still propagates. Root cause of the miss: `UpdateChecker.check` only
fires when `manifest.versionCode > installedVersionCode`, and the server (`app_dist.rs`) serves a
hand-written `manifest.json` verbatim — a same-version rebuild is invisible, and a hand-typed version
can drift from the actual file.

## Rollout constraint (why two phases)

Every **already-installed** client only understands `versionCode`. The only signal that reaches them
is a higher `versionCode`. So we cannot switch to hash-only in one step — clients must first be
upgraded *into* a hash-aware build via a version bump.

### Phase 1 — teach the fleet hash-awareness (THIS release, versionCode 6 / 0.7.2)
- **Server** (`app_dist.rs`): compute SHA-256 of each referenced APK and inject `sha256` into the
  `/app/manifest` response (mtime-cached). Keep `versionCode`/`versionName`. Hash is derived from the
  file, so it can never drift.
- **Phone** (`UpdateChecker`): add `sha256` to `AppRelease`/`UpdateInfo`; persist the last hash the
  server gave at install time (SharedPreferences) — the phone does NOT self-hash; it trusts the
  server's advertised hash as the identity of what it installed. Update when `serverHash != storedHash`.
  Bootstrap baseline: when `storedHash == null`, if `serverVersionCode == currentVersionCode` then the
  server's current build IS us → seed `storedHash = serverHash` silently (no prompt). `versionCode`
  comparison stays as the delivery mechanism + fallback this round.
- **Bump versionCode 5 → 6 / 0.7.2** so existing v4/v5 clients pull this hash-aware build.

### Phase 2 — go hash-only (a few releases later)
Once telemetry/confidence says the fleet is on hash-aware builds (>= v6), stop bumping `versionCode`
and remove the version comparison from `UpdateChecker` → pure hash. Update this task / open a follow-up
when starting Phase 2.

## Acceptance Criteria (Phase 1)

- [ ] Server `/app/manifest` includes a correct `sha256` per release, computed from the APK file (mtime-cached).
- [ ] `AppRelease`/`UpdateInfo` carry `sha256`; phone persists the installed hash and compares hash-first.
- [ ] Baseline seed on first null hash via `versionCode == current`; no spurious prompt right after a version-delivered update.
- [ ] `versionCode` retained as bootstrap + fallback this round; manifest still carries it.
- [ ] versionCode bumped 5 → 6 / 0.7.2; signed release built (signer matches live cert) and published with manifest.
- [ ] Server rebuilt + redeployed so it advertises the hash.

## Implementation Notes

- `api` already depends on `serde_json`; add `sha2 = "0.10"` (already used by `identity`). Hex-encode without a new crate.
- `UpdateChecker.check` gains a `Context` (both call sites — `MainActivity`, `KnightHomeHost` — have one) for SharedPreferences.
- Persist on successful install in `UpdateBanner` (AppUpdater returns null = success).

## Dependencies

Builds on the OTA flow from [[SQUIRE-T-0051]] / in-app install [[SQUIRE-T-0059]]; follows the visual
redesign release [[SQUIRE-T-0084]] (which exposed the same-version-no-propagate gap).

## Status Updates

- Plan recorded. Implementing Phase 1.
