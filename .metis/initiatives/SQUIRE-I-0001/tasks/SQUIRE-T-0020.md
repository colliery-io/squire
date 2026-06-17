---
id: identity-credential-hashing-token
level: task
title: "Identity: credential hashing + token issuance/verification"
short_code: "SQUIRE-T-0020"
created_at: 2026-06-17T09:52:21.321789+00:00
updated_at: 2026-06-17T10:04:16.817571+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Identity: credential hashing + token issuance/verification

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0007]] (Identity, Tenancy & Registration)

## Objective

In `crates/identity`, implement credential hashing and token issuance/verification. This task RESOLVES the spec's "credential hashing scheme" and "token format & crypto" decision areas and records the chosen options in the task log.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] Hashing = **Argon2id** (PHC string, random per-credential salt): `creds::hash_secret(secret) -> String`, `creds::verify_secret(secret, phc) -> bool` constant-time, no panic on malformed input; stored as hash only (NFR-2.3).
- [x] Token = self-contained **HMAC-SHA256** bearer `base64url(claims) "." base64url(mac)` carrying `(household, user, role, exp)`: `TokenSigner::issue(principal, now_ms, ttl_ms)` / `verify(token, now_ms) -> Result<Principal, AuthError>`, no DB lookup, tamper-evident (constant-time `verify_slice`), expiry enforced; `now_ms` injected for determinism; token carries the tenant (local + hosted).
- [x] Tests (`crates/identity`, 9 new): hash round-trip / wrong-secret / salt-differs / malformed-no-panic; token round-trip / tampered→BadToken / wrong-key→BadToken / expired→BadToken / garbage→BadToken.

## Implementation Notes

### Technical Approach
Add `argon2` (+ `password-hash`) for hashing and `hmac` + `sha2` (or a compact JWT-like encoding) for the bearer token. The server signing key comes from config (a fixed key in tests). Inject the clock so expiry can be exercised deterministically. Keep the dependency set modest. Record the two resolved decisions (argon2; HMAC-signed self-contained token) in the task log.

### Requirements covered
REQ-1.3, REQ-1.4; NFR-2.2, NFR-2.3; resolves the token-format and credential-hashing decision areas; ADR A-0004.

### Dependencies
SQUIRE-T-0019.

## Status Updates

**2026-06-17 — Completed.** Two decision areas resolved:
- **Credential hashing → Argon2id** (argon2 0.5 default params, v19), random per-credential salt, stored as the PHC string only; upgrade path = re-hash on next login (params live in each PHC string). `crates/identity/src/creds.rs`: `hash_secret`/`verify_secret` (constant-time, no panic).
- **Token → self-contained HMAC-SHA256 bearer** `base64url(claims_json) "." base64url(mac)` over `(household, user, role, exp)` — verified with no DB lookup, tamper-evident, expiry-enforced, `now_ms` injected. `crates/identity/src/token.rs`: `TokenSigner::new/issue/verify`.

Deps added to `crates/identity`: `argon2 0.5`, `password-hash 0.5` (with `getrandom` feature — needed for `OsRng`), `hmac 0.12`, `sha2 0.10`, `base64 0.22`, `serde`/`serde_json`. Re-exported `creds`, `token`, `TokenSigner`.

Tests: `cargo test -p identity` → 15 (6 DevIdentity moved-in + 4 creds + 5 token); `cargo test --workspace` green; 0 warnings. (api shows 32 now — the 6 DevIdentity unit tests relocated to `identity` in T-0019; no regression.) Committed.