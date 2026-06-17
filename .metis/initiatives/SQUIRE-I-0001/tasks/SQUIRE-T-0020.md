---
id: identity-credential-hashing-token
level: task
title: "Identity: credential hashing + token issuance/verification"
short_code: "SQUIRE-T-0020"
created_at: 2026-06-17T09:52:21.321789+00:00
updated_at: 2026-06-17T09:52:21.321789+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Identity: credential hashing + token issuance/verification

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0007]] (Identity, Tenancy & Registration)

## Objective

In `crates/identity`, implement credential hashing and token issuance/verification. This task RESOLVES the spec's "credential hashing scheme" and "token format & crypto" decision areas and records the chosen options in the task log.

## Acceptance Criteria

- [ ] Hashing decision = **argon2**: `hash_secret(secret) -> StoredHash` with a per-credential salt, and `verify_secret(secret, &StoredHash) -> bool` that is constant-time; secrets are stored as a hash only (NFR-2.3).
- [ ] Token decision = a self-contained **signed/MAC'd bearer token** carrying `(household, user, role)` + expiry: `issue(principal, ttl) -> AuthToken` and `verify(&AuthToken) -> Result<Principal, AuthError>`. Cheap, tamper-evident (HMAC over the claims with a server signing key), with expiry enforced; works for both local and hosted because the token carries the tenant.
- [ ] Tests: hash round-trip; wrong-secret fails; same secret hashes differently (per-credential salt); token round-trip; tampered token rejected; expired token rejected; wrong signing key rejected.

## Implementation Notes

### Technical Approach
Add `argon2` (+ `password-hash`) for hashing and `hmac` + `sha2` (or a compact JWT-like encoding) for the bearer token. The server signing key comes from config (a fixed key in tests). Inject the clock so expiry can be exercised deterministically. Keep the dependency set modest. Record the two resolved decisions (argon2; HMAC-signed self-contained token) in the task log.

### Requirements covered
REQ-1.3, REQ-1.4; NFR-2.2, NFR-2.3; resolves the token-format and credential-hashing decision areas; ADR A-0004.

### Dependencies
SQUIRE-T-0019.

## Status Updates

*To be added during implementation*