---
id: pairing-2-3-keep-pair-a-device
level: task
title: "Pairing 2/3 — Keep 'Pair a device' screen (mint code, render QR + text)"
short_code: "SQUIRE-T-0045"
created_at: 2026-06-17T20:00:00+00:00
updated_at: 2026-06-17T20:28:51.294999+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Pairing 2/3 — Keep 'Pair a device' screen (mint code, render QR + text)

## Parent Initiative

[[SQUIRE-I-0001]] · Implements [[SQUIRE-A-0010]] (decided) step 2 · umbrella [[SQUIRE-T-0042]] · Spec [[SQUIRE-S-0004]] · **blocked_by [[SQUIRE-T-0044]]**

## Objective

Give the parent a way, on the computer (the Keep), to **pair a phone to a chosen member**: a "Pair a device" screen that lists members, mints a one-time code via the T-0044 server path, and renders a **QR** encoding `{host, port, household_handle, pairing_code}` plus the code as text (manual fallback). The Keep is the authenticated minting authority (A-0010).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] A Keep "Pair a device" panel (nav + `#pair-panel`): pick an active member → POST `/api/pair/codes` (mints via the held `identity.mint_pairing_code`) → shows the **QR** + the text code + expiry/host/household line.
- [x] The QR payload is `squire://pair?host={host}&port={port}&household={handle}&code={code}` — host/port from `SQUIRE_PAIR_HOST`/`SQUIRE_PAIR_PORT` (falling back to `API_PORT`, default `10.0.2.2:8080` for the emulator), i.e. the **api** address. QR rendered **server-side** (pure-Rust `qrcode` → SVG, `default-features=false`/`svg` so no `image` dep) and returned in the JSON for the page to drop in — no JS QR lib.
- [x] Single-use + expiring (enforced by T-0044's identity/store); re-submitting mints a fresh code. Only the hashed code is persisted (by identity); the plaintext transits the response once for the QR.
- [x] `cargo test -p keep` green incl. 3 new pairing tests + the existing no-`../api` guard (Keep still doesn't depend on `api`). **Live end-to-end verified**: Keep `/api/pair/codes` → api `POST /pair` consumes the code → per-user Squire token (200); replay → 401 (single-use).

## Implementation Notes

### Technical Approach
Add a `pair` module + asset to the Keep (mirror the existing authoring screens). Mint via the same engine-direct seam the Keep uses for privileged commands (or call the T-0044 Identity API directly — Keep already holds identity). Render the QR with a pure-Rust crate (e.g. `qrcode` → SVG) inlined into the page. Show host/port from the server's bound LAN address (config/env), plus the household handle and code.

### Dependencies
[[SQUIRE-T-0044]] (mint endpoint + code semantics) — hard blocker. The Keep crate ([[SQUIRE-T-0025]]..[[SQUIRE-T-0030]]).

### Risk Considerations
The QR must carry the **api** host/port (phones talk to the LAN api, not the loopback Keep). Getting the right advertised address matters (ties into NSD discovery in T-0046 — QR host is the fallback/initial). Don't leak the plaintext code into logs or persistent storage. Keep the Keep ↔ api independence (no `../api` dep) intact.

## Status Updates

**2026-06-17 — Done.** New `crates/keep/src/pair.rs`: `POST /api/pair/codes` (Operator/Knight-only) → `identity.mint_pairing_code` → returns `{user, code, expires_at, household, host, port, payload, qr_svg}`. QR via pure-Rust `qrcode` (`default-features=false, features=["svg"]`) encoding a `squire://pair?...` URI; the base64url code + sanitized handle are already URL-safe so no escaping. host/port from `SQUIRE_PAIR_HOST`/`SQUIRE_PAIR_PORT`→`API_PORT`, default `10.0.2.2:8080`. Wired the route in `lib.rs`. UI: nav "Pair" + `#pair-panel` in `index.html` (member dropdown, generate button, QR + code + expiry line), `loadPairMembers()` + submit handler in `keep.js`, panel added to both post-login un-hide lists.

3 new tests in `crates/keep/tests/pair.rs` (Knight mints + the code consumes to that member + single-use; Squire operator → 403; unauthenticated → 401); full `cargo test -p keep` green (no-`../api` guard intact). **Live**: restarted `squire-home`, minted via the loopback Keep with a Knight token, exchanged the code at the api `POST /pair` → `{user:2, role:Squire}` token (200), replay → 401. Unblocks [[SQUIRE-T-0046]] (the phone scans this QR). Note: real LAN-IP autodetection for `host` is deferred to the phone's NSD discovery (T-0046); the QR host is the initial/fallback.