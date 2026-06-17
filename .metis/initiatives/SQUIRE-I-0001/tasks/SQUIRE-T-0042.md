---
id: real-pairing-qr-lan-discovery
level: task
title: "Real pairing (QR + LAN discovery) + secure per-user token storage (NFR-5)"
short_code: "SQUIRE-T-0042"
created_at: 2026-06-17T17:25:00+00:00
updated_at: 2026-06-17T20:54:32.969269+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Real pairing (QR + LAN discovery) + secure per-user token storage (NFR-5)

## Parent Initiative

[[SQUIRE-I-0001]] · Specs: [[SQUIRE-S-0006]] / [[SQUIRE-S-0005]] (pairing decision areas) · [[SQUIRE-S-0007]] (Identity) · [[SQUIRE-S-0003]] (API) · A-0004

## Objective

Replace the **baked demo credentials** (`household=demo`, `user`, `secret=demo`, `baseUrl=10.0.2.2:8080`) hard-coded in both apps' `MainActivity` with a real one-time **pairing** flow that yields a per-user token stored securely, plus **LAN discovery** so the app finds the Keep without a typed IP (NFR-6: the address may change). This is the NFR-5 trust story: a tenant-scoped bearer token bound to the parent's / child's account, not a shared secret, so neither another LAN device nor the wrong-role app can act.

## ⚠️ Needs a decision first (likely an ADR)

The spec leaves these **Decision Areas: ADR TBD** — resolve before/within this task:
- **Pairing channel**: the Keep (admin, on the computer) displays a QR encoding `{host, port, household_handle, one-time pairing code}`; the phone scans it, POSTs the code to a control-plane `/pair` endpoint, and receives its per-user token. (Alternative: short numeric code typed in. QR is the proposed default.)
- **LAN discovery**: mDNS/NSD (`_squire._tcp`) advertised by `squire-home`, browsed by the app, so the host/port aren't hard-coded and survive address changes. (Alternative: remember last-known host + manual re-entry.)
- **Secure storage**: Android Keystore-backed `EncryptedSharedPreferences` for the token + host; iOS Keychain later. The token is presented on every call (already wired via the okhttp interceptor).
- **Server surface**: a new `/pair` (consume one-time code → mint per-user token) on the control plane (SQUIRE-S-0003 / Identity SQUIRE-S-0007), and a Keep screen to **generate** a pairing code/QR for a chosen member.

**[[SQUIRE-A-0010]] is DECIDED (2026-06-17).** Resolved choices: QR = **ZXing**; **debug-only demo bypass** kept (release always pairs); pairing code = **single-use, ≥128-bit, 30-min TTL**.

This task is now an **umbrella** tracking three implementation sub-tasks (build them in order):
- **[[SQUIRE-T-0044]]** — Identity mint/consume one-time codes + control-plane `POST /pair` (server foundation; pure Rust + tests).
- **[[SQUIRE-T-0045]]** — Keep "Pair a device" screen (mint + render QR/text). *blocked_by T-0044.*
- **[[SQUIRE-T-0046]]** — Phone pairing: NSD discovery, ZXing scan, `/pair` exchange, Keystore `SessionStore`, debug bypass; removes the baked creds. *blocked_by T-0044.*

This umbrella closes when all three complete. **2026-06-17 — all three done** (T-0044 server, T-0045 Keep QR, T-0046 phone). Real device pairing replaces the baked demo creds: a Knight mints a one-time code in the Keep (QR), the phone exchanges it at `POST /pair` for a per-user token stored Keystore-encrypted; a debug-only bypass keeps the emulator flow one-tap. Live-verified end to end. **Small follow-up noted in T-0046**: `squire-home` doesn't yet advertise `_squire._tcp` for NSD discovery (the QR carries host/port, so this isn't blocking).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] ADR decided ([[SQUIRE-A-0010]]) for the pairing channel, LAN discovery, and secure storage.
- [x] Control-plane `/pair` (T-0044): one-time, single-use, ≥128-bit, 30-min, hashed, tenant-scoped codes; wrong/expired/used rejected (uniform 401); tests cover happy path + rejections.
- [x] Keep UI (T-0045): "Pair a device" screen mints + renders a QR/code for a selected member.
- [x] Both apps (T-0046): first-run `PairingScreen` (scan/manual/discover), `/pair` exchange, Keystore-encrypted `SessionStore`; subsequent launches use the stored token; baked `MainActivity` constants removed.
- [x] Token presented on every call (interceptor); offline-first preserved; LAN-only. `cargo test --workspace` + Android assembles green; live emulator pairing demo captured.

## Implementation Notes

### Technical Approach
Server: add `/pair` to the control plane backed by Identity (mint a one-time code → token), and a Keep authoring screen to generate it. Phone: Android NSD (`NsdManager`) to discover `_squire._tcp`; a QR scanner (e.g. ML Kit / ZXing) for the code; `EncryptedSharedPreferences` (Jetpack Security) for `{host, token}`; a small `Pairing`/`SessionStore` that the existing adapters read instead of the baked constants. Both adapters already attach `Authorization: Bearer <token>` and tolerate an unreachable Keep — only the *source* of host+token changes.

### Dependencies
Identity token issuance (A-0004, [[SQUIRE-T-0020]]/[[SQUIRE-T-0022]]); control-plane endpoints ([[SQUIRE-T-0017]]); the two phone apps ([[SQUIRE-T-0037]]/[[SQUIRE-T-0040]]). Likely decomposes into: ADR → server `/pair` + Keep UI → phone pairing/discovery/storage (per app).

### Risk Considerations
This is the biggest remaining item and spans every component — strong candidate to **decompose into sub-tasks** after the ADR. Security: one-time codes must be short-TTL, single-use, role-scoped; the token must never land in plaintext prefs/logs. Discovery: NSD on the emulator/host loopback can be finicky — keep a manual host-entry fallback. Out of scope: cloud relay, multi-LAN, cert pinning (LAN HTTP for MVP, per NFR-6).

## Status Updates

*To be added during implementation*