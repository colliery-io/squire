---
id: pairing-3-3-phone-pairing-nsd
level: task
title: "Pairing 3/3 — Phone pairing: NSD discovery, ZXing scan, /pair, Keystore SessionStore, debug bypass"
short_code: "SQUIRE-T-0046"
created_at: 2026-06-17T20:00:00.000000+00:00
updated_at: 2026-06-17T20:00:00.000000+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Pairing 3/3 — Phone pairing: NSD discovery, ZXing scan, /pair, Keystore SessionStore, debug bypass

## Parent Initiative

[[SQUIRE-I-0001]] · Implements [[SQUIRE-A-0010]] (decided) steps 3+4 · umbrella [[SQUIRE-T-0042]] · Specs [[SQUIRE-S-0005]]/[[SQUIRE-S-0006]] · **blocked_by [[SQUIRE-T-0044]]** (and benefits from [[SQUIRE-T-0045]])

## Objective

Replace the **baked demo constants** in both apps' `MainActivity` with a real first-run pairing flow (A-0010): discover the host via mDNS/NSD (manual fallback), scan the QR with **ZXing**, exchange the code at `POST /pair`, and persist `{host, port, handle, token, role}` in **Keystore-backed `EncryptedSharedPreferences`**. Keep a **debug-only bypass** so emulator/`squire-home` demos stay one-tap.

## Acceptance Criteria

- [ ] A shared `SessionStore` (or small `:pairing` module) reused by both `:app` and `:knight-app`: reads/writes `{host, port, household_handle, token, role}` in Keystore-backed `EncryptedSharedPreferences`; the existing okhttp interceptor + adapters read host+token from it instead of the baked constants.
- [ ] First-run pairing UI: **NSD** browse for `_squire._tcp` to prefill host/port (with a **manual host-entry fallback**), a **ZXing** (`zxing-android-embedded`) QR scan that fills `{host, port, handle, code}`, then a `POST /pair` exchange that stores the returned token. Subsequent launches skip straight to the home screen.
- [ ] A **"forget device"** action clears the stored session (returns to pairing).
- [ ] **Debug-only bypass** (build flag / `BuildConfig.DEBUG`): when set, skip pairing and use the demo creds against `squire-home` so the emulator flow stays one-tap; **release builds always require pairing**.
- [ ] The baked `baseUrl/household/user/secret` constants are removed from both `MainActivity`s. `:app`/`:knight-app` assemble; live emulator demo: pair the Squire + Knight from the Keep's QR (or the debug bypass), then the existing claim→approve loop works end-to-end with the paired tokens.

## Implementation Notes

### Technical Approach
Add a `:pairing` Android library module (or shared package) housing `SessionStore` (Jetpack Security `EncryptedSharedPreferences`), the NSD discovery helper (`NsdManager`), and a Compose pairing screen using `zxing-android-embedded`. The `SquireApiAdapter`/`KnightApiAdapter` take host+token from `SessionStore` (the interceptor already attaches the bearer). The startup flow becomes: if `SessionStore` has a session → home; else → pairing screen (or debug bypass). The emulator reaches the host as `10.0.2.2`, so the debug bypass keeps using that; real devices use the discovered/scanned host.

### Dependencies
[[SQUIRE-T-0044]] (the `/pair` exchange + SDK DTOs) — hard blocker. [[SQUIRE-T-0045]] (the Keep QR to scan) for the full demo. Both phone apps ([[SQUIRE-T-0037]]/[[SQUIRE-T-0040]]).

### Risk Considerations
NSD is flaky on emulators — the manual-host fallback and the debug bypass are how the demo stays runnable. Token never in plaintext prefs/logs (Keystore only). Camera permission for ZXing. Keep `:core`/`:knight-core` pure (pairing lives in `:app`/`:knight-app`/`:pairing`, not the JVM cores). iOS Keychain deferred (no iOS client yet).

## Status Updates

*To be added during implementation*
