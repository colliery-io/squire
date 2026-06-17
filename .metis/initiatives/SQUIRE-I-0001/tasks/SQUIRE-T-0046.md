---
id: pairing-3-3-phone-pairing-nsd
level: task
title: "Pairing 3/3 — Phone pairing: NSD discovery, ZXing scan, /pair, Keystore SessionStore, debug bypass"
short_code: "SQUIRE-T-0046"
created_at: 2026-06-17T20:00:00+00:00
updated_at: 2026-06-17T20:54:00.646752+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Pairing 3/3 — Phone pairing: NSD discovery, ZXing scan, /pair, Keystore SessionStore, debug bypass

## Parent Initiative

[[SQUIRE-I-0001]] · Implements [[SQUIRE-A-0010]] (decided) steps 3+4 · umbrella [[SQUIRE-T-0042]] · Specs [[SQUIRE-S-0005]]/[[SQUIRE-S-0006]] · **blocked_by [[SQUIRE-T-0044]]** (and benefits from [[SQUIRE-T-0045]])

## Objective

Replace the **baked demo constants** in both apps' `MainActivity` with a real first-run pairing flow (A-0010): discover the host via mDNS/NSD (manual fallback), scan the QR with **ZXing**, exchange the code at `POST /pair`, and persist `{host, port, handle, token, role}` in **Keystore-backed `EncryptedSharedPreferences`**. Keep a **debug-only bypass** so emulator/`squire-home` demos stay one-tap.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] A shared **`:pairing`** module reused by both apps: `SessionStore` reads/writes `{host, port, household, token, user, role}` in **Keystore-backed `EncryptedSharedPreferences`** (`MasterKey` AES256_GCM); the adapters take `(baseUrl, household, token)` from the session and attach the bearer via the okhttp interceptor — no baked constants, no in-app login.
- [x] First-run `PairingScreen`: **NSD** "Discover" (`NsdManager` browse `_squire._tcp`, best-effort + graceful null) prefills host/port; **manual host/port/household/code** entry; **ZXing** (`zxing-android-embedded`) "Scan QR" → parses `squire://pair?...` → auto-exchanges; `PairingClient` calls `POST /pair` → stores the session. Subsequent launches go straight to home.
- [x] A **"Forget"** action (top bar, both apps) clears the session and returns to pairing.
- [x] **Debug-only bypass** (`BuildConfig.DEBUG`): a "Use demo creds" button logs in against `squire-home` (Squire user 2 / Knight user 1) → session; release builds (no DEBUG) omit it and require pairing.
- [x] Baked `baseUrl/household/user/secret` removed from both `MainActivity`s (now session-gated). `:app` + `:knight-app` assemble; `:core`/`:knight-core`/`:sdk` tests green. **Live emulator demo**: Squire **paired via the real `/pair`** (manual code minted by the Keep) → player home with token-authenticated state (replay of the code → 401); Knight paired via debug bypass → review home; **Forget → back to pairing**. (Camera QR scan is built but not emulator-drivable — no camera; verified via the manual-code + bypass paths, as agreed.)

## Implementation Notes

### Technical Approach
Add a `:pairing` Android library module (or shared package) housing `SessionStore` (Jetpack Security `EncryptedSharedPreferences`), the NSD discovery helper (`NsdManager`), and a Compose pairing screen using `zxing-android-embedded`. The `SquireApiAdapter`/`KnightApiAdapter` take host+token from `SessionStore` (the interceptor already attaches the bearer). The startup flow becomes: if `SessionStore` has a session → home; else → pairing screen (or debug bypass). The emulator reaches the host as `10.0.2.2`, so the debug bypass keeps using that; real devices use the discovered/scanned host.

### Dependencies
[[SQUIRE-T-0044]] (the `/pair` exchange + SDK DTOs) — hard blocker. [[SQUIRE-T-0045]] (the Keep QR to scan) for the full demo. Both phone apps ([[SQUIRE-T-0037]]/[[SQUIRE-T-0040]]).

### Risk Considerations
NSD is flaky on emulators — the manual-host fallback and the debug bypass are how the demo stays runnable. Token never in plaintext prefs/logs (Keystore only). Camera permission for ZXing. Keep `:core`/`:knight-core` pure (pairing lives in `:app`/`:knight-app`/`:pairing`, not the JVM cores). iOS Keychain deferred (no iOS client yet).

## Status Updates

**2026-06-17 — Done.** New `:pairing` Android library (`com.squire.pairing`, added to settings + the `android-library` plugin to the root): `Session` (+ Keystore `SessionStore`), `PairTarget.parse` (`squire://pair?...`), `PairingClient.pair` (over `ControlApi.pair`), `NsdDiscovery` (best-effort), `PairingScreen` (Scan QR via ZXing `ScanContract` / manual entry / Discover / debug demo button) + a CAMERA manifest permission. Catalog gained `securityCrypto` (1.1.0-alpha06) + `zxing` (4.3.0).

Both apps rewired: adapters (`SquireApiAdapter`, `KnightApiAdapter`) are now **token-based** `(baseUrl, household, token)` — dropped the in-app login/lazy-login (T-0037's lazy login is obsolete now that the token is a stored session). `MainActivity`s are session-gated: `SessionStore.load()` → `PairingScreen` or a `…HomeHost` composable that builds the store from the session and runs auto-refresh; `BuildConfig.DEBUG`-gated demo bypass; "Forget" clears the session. `buildConfig=true` enabled on both apps (Knight imports `com.squire.knight.BuildConfig` since its namespace ≠ package).

Also fixed a latent break: `:knight-core` `KnightStoreTest` constructed `HouseholdReview` without the T-0043 fields → re-added `items/quests/today`; green now.

**Live**: Squire reinstalled + data-cleared → pairing screen → typed host `10.0.2.2` / household `demo` / a Keep-minted code → **Pair** → player home (state fetched with the paired token; the code's replay at `/pair` → 401, proving real consumption). Knight → "Use demo creds" → review home. Forget → back to pairing. Camera scan path built (ZXing) but the AVD has no camera, so verified via manual-code + bypass (as agreed). **Known gap**: `squire-home` doesn't yet *advertise* `_squire._tcp`, so "Discover" times out to null — fine because the QR carries host/port; advertising is a small server follow-up. Completes the [[SQUIRE-T-0042]] umbrella (server T-0044 + Keep T-0045 + phone T-0046).