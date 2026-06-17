---
id: real-pairing-qr-lan-discovery
level: task
title: "Real pairing (QR + LAN discovery) + secure per-user token storage (NFR-5)"
short_code: "SQUIRE-T-0042"
created_at: 2026-06-17T17:25:00.000000+00:00
updated_at: 2026-06-17T17:25:00.000000+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


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

**Recommend** authoring an ADR ("Device pairing & per-user token provisioning over the LAN") and driving it to *decided* (human-in-the-loop) before implementing — this spans Identity, API, Keep, and both phones.

## Acceptance Criteria

- [ ] ADR decided for the pairing channel, LAN discovery, and secure storage (supersedes the baked-creds placeholder).
- [ ] Control-plane `/pair`: the Keep generates a one-time pairing code (scoped to a household + member/role, short TTL); the phone exchanges it for a per-user token. Wrong/expired/used codes are rejected. Tests cover the happy path + rejections.
- [ ] Keep UI: a "Pair a device" screen that mints + displays a code/QR for a selected member (Knight or Squire).
- [ ] Both apps: a first-run pairing screen (scan QR / enter code) that discovers the host via NSD, pairs, and stores `{host, token}` in Keystore-backed encrypted storage; subsequent launches use the stored token; the demo `MainActivity` constants are gone.
- [ ] Token presented on every API call (existing interceptor); offline-first still holds (no pairing needed to render cache). LAN-only; no cloud relay. `cargo test --workspace` + Android assembles green; live emulator pairing demo captured.

## Implementation Notes

### Technical Approach
Server: add `/pair` to the control plane backed by Identity (mint a one-time code → token), and a Keep authoring screen to generate it. Phone: Android NSD (`NsdManager`) to discover `_squire._tcp`; a QR scanner (e.g. ML Kit / ZXing) for the code; `EncryptedSharedPreferences` (Jetpack Security) for `{host, token}`; a small `Pairing`/`SessionStore` that the existing adapters read instead of the baked constants. Both adapters already attach `Authorization: Bearer <token>` and tolerate an unreachable Keep — only the *source* of host+token changes.

### Dependencies
Identity token issuance (A-0004, [[SQUIRE-T-0020]]/[[SQUIRE-T-0022]]); control-plane endpoints ([[SQUIRE-T-0017]]); the two phone apps ([[SQUIRE-T-0037]]/[[SQUIRE-T-0040]]). Likely decomposes into: ADR → server `/pair` + Keep UI → phone pairing/discovery/storage (per app).

### Risk Considerations
This is the biggest remaining item and spans every component — strong candidate to **decompose into sub-tasks** after the ADR. Security: one-time codes must be short-TTL, single-use, role-scoped; the token must never land in plaintext prefs/logs. Discovery: NSD on the emulator/host loopback can be finicky — keep a manual host-entry fallback. Out of scope: cloud relay, multi-LAN, cert pinning (LAN HTTP for MVP, per NFR-6).

## Status Updates

*To be added during implementation*
