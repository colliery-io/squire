---
id: signed-release-builds-of-both
level: task
title: "Signed release builds of both phone apps (strip debug bypass, real app identity)"
short_code: "SQUIRE-T-0049"
created_at: 2026-06-17T21:20:00+00:00
updated_at: 2026-06-17T21:50:00.955881+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Signed release builds of both phone apps (strip debug bypass, real app identity)

## Parent Initiative

[[SQUIRE-I-0001]] · Specs [[SQUIRE-S-0005]]/[[SQUIRE-S-0006]] · follow-up of [[SQUIRE-T-0046]] (pairing)

## Objective

Today only **debug** APKs exist (sideloaded via `adb`), and they compile in the "Use demo creds" bypass. Produce **signed release builds** of both apps that a real user can install, with the debug bypass and any demo shortcuts compiled out of release, and a real app identity (label, icon, version). This is the "installable artifact" step — distribution channel (store vs sideload) is out of scope here; just produce a clean, signed release APK/AAB for each.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] Release signing for both apps from a **gitignored `keystore.properties`** (+ `keystore.properties.example`; `keystore/` + `keystore.properties` added to `.gitignore`). `assembleRelease` produces APKs **signed** with `CN=Squire` (verified via `apksigner verify` → VALID for both); when the props file is absent the release still builds (unsigned).
- [x] **Demo bypass absent from release**: R8 folds `BuildConfig.DEBUG` to false and removes the branch — confirmed the release dex has **no `demoLogin` / no `10.0.2.2` / no demo-login path**; on-device both release apps show only Scan QR + manual entry, **no "Use demo creds" button**. (Residual: the dead label string "Use demo creds (debug)" still lives in the shared `:pairing` composable — never rendered in release; noted below.)
- [x] **R8 on** (`isMinifyEnabled = true`) with `proguard-rules.pro` keeping kotlinx-serialization `$$serializer`/`serializer()` for `com.squire.**` + `-dontwarn com.google.errorprone.**` (Tink via security-crypto). APK ~1.9 MB. `usesCleartextTraffic` kept (LAN HTTP, NFR-6); existing `versionName`/`versionCode`, labels (Squire/Knight), and `INTERNET`/`CAMERA` permissions reviewed (default launcher icon kept — custom icon is later polish).
- [x] Both release APKs install + run: release **Squire paired for real** (manual code from the Keep → `/pair`, replay → 401) and reached the player home; release **Knight** installs + shows the pairing screen (no bypass). `:core`/`:knight-core`/`:sdk` tests green; both `assembleRelease` green.

## Implementation Notes

### Technical Approach
Add a `signingConfigs { release { ... } }` reading from `gradle.properties`/env (storeFile/storePassword/keyAlias/keyPassword), reference it in `buildTypes.release`. Generate a dev keystore (gitignored) and document regeneration. Turn on R8 (`isMinifyEnabled = true`) and add ProGuard keep rules for kotlinx-serialization, the generated `com.squire.sdk.*` models, Room entities/DAOs, and ZXing if needed — or defer minify (`= false`) if rules get fiddly and just ship signed-unminified for the first cut (note the choice). Verify the bypass is gone (`aapt`/`strings` on the release APK shouldn't reveal the demo path; functionally, the button is absent).

### Dependencies
[[SQUIRE-T-0046]] (the `BuildConfig.DEBUG` bypass to strip), [[SQUIRE-T-0048]] (a real server to pair against for the no-bypass run).

### Risk Considerations
R8/minify + kotlinx-serialization + reflection-y generated code is the classic breakage — keep rules or skip minify for v1. Keystore secrets must never land in git. Cleartext HTTP is intentional (LAN-only) but flagged by Play if ever uploaded — out of scope here (sideload-first). The no-bypass pairing run depends on a reachable server (use `squire-serve` from T-0048 or `squire-home`).

## Status Updates

**2026-06-17 — Done.** Generated a dev release keystore (`keystore/squire-release.jks`, gitignored) + `keystore.properties` (gitignored) with a committed `.example`. Both `app`/`knight-app` `build.gradle.kts`: load `keystore.properties` if present → `signingConfigs.release`; `buildTypes.release` now `isMinifyEnabled = true` + `proguard-rules.pro` + the signing config. `proguard-rules.pro` keeps kotlinx-serialization for `com.squire.**` and `-dontwarn com.google.errorprone.**` (Tink, transitive via security-crypto — the one R8 break, fixed from R8's `missing_rules.txt`).

Verified: `apksigner verify` → both APKs signed VALID (`CN=Squire`), ~1.9 MB each. Release dex has no `demoLogin`/`10.0.2.2` (bypass stripped); on the emulator both release apps show the pairing screen with **no demo button**. Swapped debug→release on the emulator (uninstall needed — different signing key): the **release Squire paired for real** (Keep-minted code → manual entry → `/pair`, replay → 401) → player home; release Knight installs + pairing screen. `:core`/`:knight-core`/`:sdk` green; both `assembleRelease` green.

**Notes / deferred (not blocking):** (1) the dead label string "Use demo creds (debug)" remains in `:pairing` (R8 can't strip a literal in a reachable composable; fully removing it needs a debug-only source set — cosmetic). (2) Default launcher icon kept (custom icon = polish). (3) cleartext HTTP retained (LAN-only by design; would need addressing only if ever uploaded to Play). Unblocks [[SQUIRE-T-0050]] (real-device pass, ideally on these signed releases).