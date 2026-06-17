---
id: pairing-1-3-identity-mint-consume
level: task
title: "Pairing 1/3 — Identity mint/consume one-time codes + control-plane POST /pair"
short_code: "SQUIRE-T-0044"
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

# Pairing 1/3 — Identity mint/consume one-time codes + control-plane POST /pair

## Parent Initiative

[[SQUIRE-I-0001]] · Implements [[SQUIRE-A-0010]] (decided) step 1 · umbrella [[SQUIRE-T-0042]] · Specs [[SQUIRE-S-0007]]/[[SQUIRE-S-0003]] · extends [[SQUIRE-A-0004]]

## Objective

The **server foundation** for device pairing (A-0010): let an authenticated Knight (via the Keep) mint a **one-time pairing code**, and let an unauthenticated phone exchange that code for the member's per-user token over a new `POST /pair`. Code state is **derived from the append-only log** (mint emits an event; consume checks no prior consume for that code) — no processed-codes side table (A-0001/AR-3). Pure Rust + tests, no UI; unblocks the Keep screen ([[SQUIRE-T-0045]]) and the phones ([[SQUIRE-T-0046]]).

## Acceptance Criteria

- [ ] Identity can **mint** a pairing code for `{household, user_id, role}` (caller must be an authenticated Knight): a high-entropy (≥128-bit) single-use token with a **30-minute TTL** (A-0010). Mint appends an event carrying the code's hash, target member, role, and expiry.
- [ ] Identity can **consume** a code: validates it exists, is unexpired, and has no prior consume event; on success appends a consume event and returns the member's **per-user tenant-scoped token** (same shape as `/login`, A-0004) + `{household_handle, user_id, role}`. Expired / unknown / already-consumed are rejected distinctly (internally; see risk note on the external shape).
- [ ] Control-plane endpoints: a **Knight-gated** mint (e.g. `POST /pair/codes` → code + expiry) and an **unauthenticated** `POST /pair` (consume → token). Wired in `crates/api`, and the mint reachable from the Keep's engine-direct path.
- [ ] Code stored/compared as a **hash** (never plaintext in the log); scoped to one household + member + role and cannot cross tenants.
- [ ] `cargo test --workspace` green incl. new tests (mint→consume happy path; expired; unknown; double-consume; wrong-tenant). `openapi.json` re-frozen + SDK regenerated with the new DTOs.

## Implementation Notes

### Technical Approach
Add `PairCode` DTOs to `domain-core` contract + new `Command`/`Event` variants (`PairingCodeMinted { code_hash, household, user, role, expires_at }`, `PairingCodeConsumed { code_hash, at }`) so consumption is log-derived. In `identity`, add `mint_pairing_code(&Principal, target)` and `consume_pairing_code(code) -> (token, Principal)`. In `api`, add the two control-plane routes (mint behind `RequireKnight`, `/pair` open). Hash with a keyed hash (reuse the token-signer key or a dedicated one). Re-freeze openapi + regen SDK per the T-0031/T-0038 pipeline.

### Dependencies
[[SQUIRE-A-0010]] (decided), Identity token issuance ([[SQUIRE-T-0020]]/[[SQUIRE-T-0022]]), control-plane endpoints ([[SQUIRE-T-0017]]), the log/event model ([[SQUIRE-A-0001]]).

### Risk Considerations
`/pair` is the one **unauthenticated** surface — keep it tight: single-use, short TTL, high entropy, hash-compared, tenant-scoped. Avoid an enumeration oracle (uniform error shape/timing for unknown vs expired where feasible). Never log the plaintext code. Keep consumption derived from the log (no side table) to honour AR-3.

## Status Updates

*To be added during implementation*
