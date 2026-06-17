---
id: identity-integration-isolation
level: task
title: "Identity: integration, isolation, dual-backend & MVP seed; wire API to prod identity"
short_code: "SQUIRE-T-0023"
created_at: 2026-06-17T09:52:25.200559+00:00
updated_at: 2026-06-17T10:30:55.529578+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Identity: integration, isolation, dual-backend & MVP seed; wire API to prod identity

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0007]] (Identity, Tenancy & Registration)

## Objective

Wire the api onto the production `Identity`, prove full tenant isolation and per-user no-bypass end-to-end, seed the MVP household through the real flow, and run the suite on both backends.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] `AppState` can be built with the production `Identity` (a constructor/example wiring `ProdIdentity` in place of `DevIdentity`); the api integration flow (register→login→claim→approve→state) passes over HTTP on the production identity.
- [x] Full tenant isolation: two households registered; each member's token reaches only its own tenant; no cross-tenant read/write; a member of A cannot authenticate into or enumerate B (no global user directory).
- [x] No account-bypass: every protected call requires a valid token; there is no shared/anonymous path (NFR-2.2).
- [x] MVP seed: seed a household with N Knights + N Squires (e.g. 2 Knights + 1 Squire) through the REAL register/add-member flow (no fixture back-door, REQ-1.10); each member logs in and acts per role.
- [x] Audit: "who added member X (and when)" is answerable from the users audit columns (REQ-1.12).
- [x] Dual-backend: identity + provisioning + isolation tests run on SQLite AND the compose Postgres (`--features postgres` + `DATABASE_URL`).
- [x] `cargo test --workspace` is green and the build is warning-free.

## Implementation Notes

### Technical Approach
Compose `ProdIdentity` into the api's `Arc<dyn Identity>`. Reuse the api oneshot/HTTP harness but drive it on `ProdIdentity`. Isolation tests register two households and assert no cross-tenant reach (read, write, auth, or enumeration). Seed the MVP household via the real register + add-member endpoints rather than a fixture. Run the PG-sensitive tests via docker compose Postgres with `--features postgres` and `DATABASE_URL` set.

### Requirements covered
REQ-1.5, REQ-1.6, REQ-1.10; NFR-2.1, NFR-2.2, NFR-2.4; ADR A-0002, A-0004.

### Dependencies
SQUIRE-T-0019..T-0022; api (T-0018); `store` Provisioner.

## Status Updates

**2026-06-17 — Done.** Wired the api onto the production identity and proved isolation / no-bypass / MVP-seed on both backends.

- **`AppState::local_prod(backend, handle, signer, ttl)`** (`crates/api/src/state.rs`): provisions the tenant (creating the SQLite file / Postgres schema + migrating — required on PG, where `open` alone does not create the schema), opens the handlers' store, and binds a `ProdIdentity` over the SAME handle/backend. ⚠️ *Superseded by the concurrency follow-up below: the identity now shares the handlers' one store, not a separate registry connection.*
- **Hardened `ProdIdentity::verify`**: in Local posture, a presented handle ≠ the bound household is now `WrongTenant`, so an api bound to household B refuses an otherwise-valid token minted for A.
- **`crates/api/Cargo.toml`**: added `postgres` feature = `["store/postgres", "identity/postgres"]`.
- **`crates/api/tests/prod_integration.rs`** (new, dual-backend via `each_backend_async`): AC-1 full flow over HTTP on `ProdIdentity`; MVP seed of **2 Knights + 1 Squire** built only through real `register` + `/members` (no fixture), each logging in and acting per role; **audit** assertions (admin Knight = system seed `created_by=None`; added members record Knight #1 via `store::user_audit`); **no-bypass** (missing/garbage/wrong-tenant → 401); **two-household isolation** (two AppStates, shared signing key: A's token reaches nothing on B; A's admin cannot log into B; B's directory holds only its own member — ids are per-tenant so isolation is asserted on member data, not id equality).
- **`crates/identity/tests/prod.rs`**: added a **hosted** multi-tenant isolation test (two households registered through the real flow; cross-tenant `verify`/`login` rejected; a probe registry confirms a member added to A never appears in B; no global directory).

**Verification.** `cargo test --workspace` green & warning-free (32 test binaries). Postgres path run against docker-compose (`docker compose up -d postgres`; `PQ_LIB_DIR`/`DYLD_FALLBACK_LIBRARY_PATH`/`DATABASE_URL` + `cargo test -p identity -p api --features postgres`) — all green, prod-integration executed (not skipped). Compose torn down. Committed as `73aa10f`.

**2026-06-17 — Concurrency follow-up (`cb0b3fc`).** The api services requests CONCURRENTLY (confirmed with the user), so the initial wiring — `ProdIdentity` routing through its own `TenantRegistry`, opening a SEPARATE connection from the handlers' `Mutex<Store>` — was a second uncoordinated writer per tenant (SQLite `SQLITE_BUSY`; lost-update hazard on `users` everywhere). Reworked so the single-writer invariant (AR-1) holds under real interleaving: `ProdIdentity` gained a `Tenancy` enum (`Shared` / `LocalRegistry` / `Hosted`); `ProdIdentity::shared_local` + `AppState::local_prod` now hand the identity the SAME `Arc<Mutex<Store>>` the handlers hold, so one lock over one connection serializes ALL writers. `register`'s existing-Knight check + seed are atomic under that lock (one `with_tenant_store` closure). Added a multi-thread concurrency test (`concurrent_control_plane_and_feature_writes_serialize`): concurrent `/members` + `/admin/adjust` all 200, apply exactly once, no busy errors (stable across repeated runs). Hosted concurrent-api (a per-tenant `SharedStore` map shared with handlers) is noted as future work; not needed for the single-tenant local MVP. Dual-backend re-verified green.