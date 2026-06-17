---
id: identity-production-identity-impl
level: task
title: "Identity: production Identity impl (register/login/add-member/verify/authorize)"
short_code: "SQUIRE-T-0022"
created_at: 2026-06-17T09:52:24.150966+00:00
updated_at: 2026-06-17T10:18:53.972652+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Identity: production Identity impl (register/login/add-member/verify/authorize)

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0007]] (Identity, Tenancy & Registration)

## Objective

In `crates/identity`, implement the production `Identity` impl that ties together hashing (T-0020), tokens (T-0020), the tenant registry (T-0021), and the `Store` writer. Users and their hashed credentials live IN the tenant schema; there is no global user directory.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] `ProdIdentity::register(req)`: provision/resolve tenant, seed first **Knight** via `apply(None, [PutUser])`, store `hash_secret(admin_secret)` in-tenant, mint token → `{household, admin, token}`.
- [x] `add_member(caller, req)`: Knight-only (else `Forbidden`); `PutUser` via `apply(Some(caller.user))` (audited); `set_credential(hash(initial_secret))` → `{user}`. Any N Knights/Squires.
- [x] `login(req)`: resolve tenant → `verify_secret` against the stored hash → role from `users` → token → `{token, role}`; bad/missing → `BadToken`.
- [x] `verify(handle, token)`: `TokenSigner::verify` (tamper/expiry) → Principal; `principal.household != handle` → `WrongTenant`.
- [x] Authorize by `(tenant, user, role)`: credentials stored only as Argon2id hashes in the **in-tenant `credentials` table** (plaintext never stored); users in-tenant; no global directory.
- [x] Audit: `by=Some(caller)` on add-member, `None` on register seed → users-table audit answers "who added member X".
- [x] Tests (`tests/prod.rs`, 8): register→verify(Knight), hash-not-plaintext, add-member Knight (audited)/Squire→Forbidden, login wrong-secret/unknown→BadToken, token tamper/expiry→BadToken, cross-tenant→WrongTenant. + store `credentials_set_and_read_back` (both backends).

## Implementation Notes

### Technical Approach
Store each member's hashed secret in-tenant by **adding an in-tenant `credentials` table (user_id → hash) to the store migrations** (coordinate with S-0002's migration set; this is the in-tenant credential storage REQ-1.6 needs). Register and add-member write it; login reads and verifies it. Compose hashing (T-0020) + tokens (T-0020) + registry (T-0021) + the `Store` writer into a single `Identity` implementation.

### Requirements covered
REQ-1.1, REQ-1.2, REQ-1.3, REQ-1.4, REQ-1.5, REQ-1.9, REQ-1.10, REQ-1.11, REQ-1.12; NFR-2.2, NFR-2.3, NFR-2.5; ADR A-0004, A-0005, A-0007.

### Dependencies
SQUIRE-T-0019, SQUIRE-T-0020, SQUIRE-T-0021.

## Status Updates

**2026-06-17 — Completed.** **In-tenant credentials (store change):** added a portable `credentials(user_id TEXT PK, secret_hash TEXT)` table to the store's migration (+ `down.sql`, `schema.rs`, `pg::provision_clean` check); `Store::set_credential` (upsert via `run_upsert!`) + `Store::credential` — NOT a domain `Change`, written directly (no audit cols); migrates on both backends. `crates/identity/src/prod.rs`: `ProdIdentity{ registry, signer, clock, ttl, counter }` impl `Identity`: `register` (provision/resolve tenant → seed Knight via `apply(None,[PutUser])` → `set_credential(hash)` → token), `add_member` (Knight-only → `apply(Some(caller))` → set_credential), `login` (resolve → verify_secret vs stored hash → role from users → token), `verify` (signer verify + household match → else `WrongTenant`). **register local vs hosted:** local uses the registry's bound handle (`Forbidden` if already has a Knight); hosted derives a sanitized handle from `household_name`+counter then `registry.provision`. Credentials stored only as Argon2id hashes in-tenant; plaintext never stored. `DevIdentity` unchanged.

Tests: `tests/prod.rs` (8) + store `credentials_set_and_read_back` (both backends). Results: `cargo test -p identity` → 15 unit + 8 prod + 2 tenant green; `cargo test -p store` → 28 (incl. credentials); PG run (`--features postgres`+compose) → credentials + migration green on PG; `cargo test --workspace` green; 0 warnings. Committed.