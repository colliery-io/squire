---
id: signed-release-builds-of-both
level: task
title: "Signed release builds of both phone apps (strip debug bypass, real app identity)"
short_code: "SQUIRE-T-0049"
created_at: 2026-06-17T21:20:00.000000+00:00
updated_at: 2026-06-17T21:20:00.000000+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Signed release builds of both phone apps (strip debug bypass, real app identity)

## Parent Initiative

[[SQUIRE-I-0001]] · Specs [[SQUIRE-S-0005]]/[[SQUIRE-S-0006]] · follow-up of [[SQUIRE-T-0046]] (pairing)

## Objective

Today only **debug** APKs exist (sideloaded via `adb`), and they compile in the "Use demo creds" bypass. Produce **signed release builds** of both apps that a real user can install, with the debug bypass and any demo shortcuts compiled out of release, and a real app identity (label, icon, version). This is the "installable artifact" step — distribution channel (store vs sideload) is out of scope here; just produce a clean, signed release APK/AAB for each.

## Acceptance Criteria

- [ ] Release signing configured for `:app` and `:knight-app` (a keystore; key material kept out of git — referenced via `gradle.properties`/env, documented). `./gradlew :app:assembleRelease :knight-app:assembleRelease` produces signed APKs.
- [ ] The **debug demo bypass is absent from release**: confirm `BuildConfig.DEBUG` gating drops the "Use demo creds" button and the hardcoded `demoLogin()` path in release (a release build shows only Scan/Manual pairing). Ideally verify the demo creds string isn't even in the release binary.
- [ ] Release build hygiene: `isMinifyEnabled` decision made (R8 on, with rules so kotlinx-serialization / the generated SDK / Room survive), `usesCleartextTraffic` kept (LAN HTTP, NFR-6) but scoped if feasible, real `versionName`/`versionCode`, app label/icon, and `INTERNET`/`CAMERA` permissions reviewed.
- [ ] Both release APKs install and run on a device/emulator and reach the home screen after a real pairing (no demo bypass). `:core`/`:knight-core`/`:sdk` tests green; both `assembleRelease` green.

## Implementation Notes

### Technical Approach
Add a `signingConfigs { release { ... } }` reading from `gradle.properties`/env (storeFile/storePassword/keyAlias/keyPassword), reference it in `buildTypes.release`. Generate a dev keystore (gitignored) and document regeneration. Turn on R8 (`isMinifyEnabled = true`) and add ProGuard keep rules for kotlinx-serialization, the generated `com.squire.sdk.*` models, Room entities/DAOs, and ZXing if needed — or defer minify (`= false`) if rules get fiddly and just ship signed-unminified for the first cut (note the choice). Verify the bypass is gone (`aapt`/`strings` on the release APK shouldn't reveal the demo path; functionally, the button is absent).

### Dependencies
[[SQUIRE-T-0046]] (the `BuildConfig.DEBUG` bypass to strip), [[SQUIRE-T-0048]] (a real server to pair against for the no-bypass run).

### Risk Considerations
R8/minify + kotlinx-serialization + reflection-y generated code is the classic breakage — keep rules or skip minify for v1. Keystore secrets must never land in git. Cleartext HTTP is intentional (LAN-only) but flagged by Play if ever uploaded — out of scope here (sideload-first). The no-bypass pairing run depends on a reachable server (use `squire-serve` from T-0048 or `squire-home`).

## Status Updates

*To be added during implementation*
