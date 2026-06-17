---
id: pairing-2-3-keep-pair-a-device
level: task
title: "Pairing 2/3 — Keep 'Pair a device' screen (mint code, render QR + text)"
short_code: "SQUIRE-T-0045"
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

# Pairing 2/3 — Keep 'Pair a device' screen (mint code, render QR + text)

## Parent Initiative

[[SQUIRE-I-0001]] · Implements [[SQUIRE-A-0010]] (decided) step 2 · umbrella [[SQUIRE-T-0042]] · Spec [[SQUIRE-S-0004]] · **blocked_by [[SQUIRE-T-0044]]**

## Objective

Give the parent a way, on the computer (the Keep), to **pair a phone to a chosen member**: a "Pair a device" screen that lists members, mints a one-time code via the T-0044 server path, and renders a **QR** encoding `{host, port, household_handle, pairing_code}` plus the code as text (manual fallback). The Keep is the authenticated minting authority (A-0010).

## Acceptance Criteria

- [ ] A Keep route/screen "Pair a device": pick a member (Knight or Squire) → mint a code (engine-direct / control-plane mint from T-0044) → show a **QR** + the text code + its expiry countdown (30-min TTL).
- [ ] The QR payload is `{host, port, household_handle, pairing_code}` — host/port the LAN address the phone should hit (the api port, not the loopback Keep port). QR generated server-side (a Rust QR crate → SVG/PNG, embedded in the page) so no JS dependency beyond what the Keep already uses.
- [ ] A minted code is single-use + expires; re-opening the screen can mint a fresh one. No plaintext code is persisted beyond the page render.
- [ ] `cargo test -p keep` green (handler/render smoke test); the Keep still never depends on `../api` (existing guard test holds). Manual check: scan the QR with a phone resolves to a working `/pair` exchange (validated end-to-end in [[SQUIRE-T-0046]]).

## Implementation Notes

### Technical Approach
Add a `pair` module + asset to the Keep (mirror the existing authoring screens). Mint via the same engine-direct seam the Keep uses for privileged commands (or call the T-0044 Identity API directly — Keep already holds identity). Render the QR with a pure-Rust crate (e.g. `qrcode` → SVG) inlined into the page. Show host/port from the server's bound LAN address (config/env), plus the household handle and code.

### Dependencies
[[SQUIRE-T-0044]] (mint endpoint + code semantics) — hard blocker. The Keep crate ([[SQUIRE-T-0025]]..[[SQUIRE-T-0030]]).

### Risk Considerations
The QR must carry the **api** host/port (phones talk to the LAN api, not the loopback Keep). Getting the right advertised address matters (ties into NSD discovery in T-0046 — QR host is the fallback/initial). Don't leak the plaintext code into logs or persistent storage. Keep the Keep ↔ api independence (no `../api` dep) intact.

## Status Updates

*To be added during implementation*
