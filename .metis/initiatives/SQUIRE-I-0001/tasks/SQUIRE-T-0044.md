---
id: pairing-1-3-identity-mint-consume
level: task
title: "Pairing 1/3 — Identity mint/consume one-time codes + control-plane POST /pair"
short_code: "SQUIRE-T-0044"
created_at: 2026-06-17T20:00:00+00:00
updated_at: 2026-06-17T20:21:28.199610+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Pairing 1/3 — Identity mint/consume one-time codes + control-plane POST /pair

## Parent Initiative

[[SQUIRE-I-0001]] · Implements [[SQUIRE-A-0010]] (decided) step 1 · umbrella [[SQUIRE-T-0042]] · Specs [[SQUIRE-S-0007]]/[[SQUIRE-S-0003]] · extends [[SQUIRE-A-0004]]

## Objective

The **server foundation** for device pairing (A-0010): let an authenticated Knight (via the Keep) mint a **one-time pairing code**, and let an unauthenticated phone exchange that code for the member's per-user token over a new `POST /pair`. Code state is **derived from the append-only log** (mint emits an event; consume checks no prior consume for that code) — no processed-codes side table (A-0001/AR-3). Pure Rust + tests, no UI; unblocks the Keep screen ([[SQUIRE-T-0045]]) and the phones ([[SQUIRE-T-0046]]).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] Identity **mints** a pairing code for `{household, user, role}` (Knight-only): 32 random bytes (OsRng, ≥128-bit), base64url; **30-min TTL**; only `sha256(code)` stored in the per-tenant `pairing_codes` table with the target member + role + expiry.
- [x] Identity **consumes** a code: take-by-hash + delete (single-use), confirm the member is still active, check expiry, then issue the member's **per-user tenant-scoped token** (same `TokenSigner` as `/login`, A-0004) + `{household, user, role}` as `PairResp`. Unknown / expired / used all surface as a uniform `BadToken` (no enumeration oracle).
- [x] Control-plane endpoints: `POST /pair/codes` (**RequireKnight** → code + expiry) and `POST /pair` (**unauthenticated** → token). Wired in `crates/api`; mint also callable from the Keep (engine-direct via the same identity).
- [x] Code stored/compared as a **hash** (plaintext never persisted/logged); the `pairing_codes` table lives inside the tenant schema, so codes can't cross tenants (and `consume` is `WrongTenant`-guarded for single-tenant postures).
- [x] `cargo test --workspace` green incl. 5 new identity tests (mint→consume round-trip; single-use/double-consume; unknown; non-Knight-can't-mint; expired-via-preseeded-row). `openapi.json` re-frozen; SDK regenerated with `MintPairCodeReq/Resp`, `PairReq/Resp`, `ControlApi.mintPairCode`/`pair`.

## Implementation Notes

### Technical Approach
Add `PairCode` DTOs to `domain-core` contract + new `Command`/`Event` variants (`PairingCodeMinted { code_hash, household, user, role, expires_at }`, `PairingCodeConsumed { code_hash, at }`) so consumption is log-derived. In `identity`, add `mint_pairing_code(&Principal, target)` and `consume_pairing_code(code) -> (token, Principal)`. In `api`, add the two control-plane routes (mint behind `RequireKnight`, `/pair` open). Hash with a keyed hash (reuse the token-signer key or a dedicated one). Re-freeze openapi + regen SDK per the T-0031/T-0038 pipeline.

### Dependencies
[[SQUIRE-A-0010]] (decided), Identity token issuance ([[SQUIRE-T-0020]]/[[SQUIRE-T-0022]]), control-plane endpoints ([[SQUIRE-T-0017]]), the log/event model ([[SQUIRE-A-0001]]).

### Risk Considerations
`/pair` is the one **unauthenticated** surface — keep it tight: single-use, short TTL, high entropy, hash-compared, tenant-scoped. Avoid an enumeration oracle (uniform error shape/timing for unknown vs expired where feasible). Never log the plaintext code. Keep consumption derived from the log (no side table) to honour AR-3.

## Status Updates

**2026-06-17 — Design deviation from A-0010 (flagged for review).** The ADR proposed pairing-code state be **log-derived** (mint/consume as domain events), to stay consistent with A-0001's "no processed-commands side table." On reading the code, **credentials** (the closest analog — hashed member secrets) are deliberately stored in a dedicated per-tenant `credentials` table, written directly by the identity layer and **bypassing the event log/`apply`** (they are auth material, not household activity). Pairing codes are the same kind of thing. So I'm implementing them as a per-tenant **`pairing_codes` table** mirroring `credentials` (store row: `code_hash` PK, `user_id`, `role`, `expires_at`); **mint** inserts, **consume** looks up by hash → checks expiry → deletes (single-use enforced by deletion). This keeps auth material out of the household event log (consistent with credentials), avoids threading new variants through the domain Engine/projections, and preserves every security property the ADR wanted (single-use, 30-min TTL, ≥128-bit, hashed-at-rest, tenant-scoped). **A-0001's "no side table" was about command idempotency dedup, not auth material** — so this isn't really in tension, but noting it explicitly since the ADR text said "log-derived." Will reconcile the ADR wording if accepted.

**2026-06-17 — Done (approach: `pairing_codes` table, confirmed by Dylan).** Implemented end-to-end:
- **store**: `pairing_codes` table (migration up/down, `schema.rs`, `PairingCodeRow`), methods `insert_pairing_code` and `take_pairing_code` (lookup+delete = single-use). Mirrors `credentials` (auth material, bypasses `apply`/event log). Portable DDL (SQLite + Postgres).
- **domain-core**: `MintPairCodeReq{user}` / `MintPairCodeResp{code,expires_at}` / `PairReq{household,code}` / `PairResp{token,household,user,role}` (serde + utoipa).
- **identity**: `Identity::mint_pairing_code` / `consume_pairing_code` (default-unsupported in the trait so `DevIdentity` is untouched; full impl in `ProdIdentity`). Code = 32 bytes OsRng → base64url; hash = base64url(sha256(code)); TTL const 30 min. Uniform `BadToken` for unknown/expired/used.
- **api**: `control::mint_pair_code` (`RequireKnight`) + `control::pair` (open); routes `/pair/codes` + `/pair`; registered in `openapi.rs` (paths + 4 schemas). Re-froze `openapi.json`, regenerated the Kotlin SDK.
- **tests**: 5 new integration tests in `crates/identity/tests/prod.rs` (added `sha2`/`base64` dev-deps to pre-seed an expired code). `cargo test --workspace` all green; `:sdk:assemble` green.

Unblocks [[SQUIRE-T-0045]] (Keep QR screen) and [[SQUIRE-T-0046]] (phone pairing). **ADR A-0010 wording still says "log-derived"** — should be reconciled to "per-tenant `pairing_codes` table (like credentials)" to match what was built + approved.