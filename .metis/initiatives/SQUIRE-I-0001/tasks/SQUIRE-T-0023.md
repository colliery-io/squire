---
id: identity-integration-isolation
level: task
title: "Identity: integration, isolation, dual-backend & MVP seed; wire API to prod identity"
short_code: "SQUIRE-T-0023"
created_at: 2026-06-17T09:52:25.200559+00:00
updated_at: 2026-06-17T09:52:25.200559+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Identity: integration, isolation, dual-backend & MVP seed; wire API to prod identity

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0007]] (Identity, Tenancy & Registration)

## Objective

Wire the api onto the production `Identity`, prove full tenant isolation and per-user no-bypass end-to-end, seed the MVP household through the real flow, and run the suite on both backends.

## Acceptance Criteria

- [ ] `AppState` can be built with the production `Identity` (a constructor/example wiring `ProdIdentity` in place of `DevIdentity`); the api integration flow (register→login→claim→approve→state) passes over HTTP on the production identity.
- [ ] Full tenant isolation: two households registered; each member's token reaches only its own tenant; no cross-tenant read/write; a member of A cannot authenticate into or enumerate B (no global user directory).
- [ ] No account-bypass: every protected call requires a valid token; there is no shared/anonymous path (NFR-2.2).
- [ ] MVP seed: seed a household with N Knights + N Squires (e.g. 2 Knights + 1 Squire) through the REAL register/add-member flow (no fixture back-door, REQ-1.10); each member logs in and acts per role.
- [ ] Audit: "who added member X (and when)" is answerable from the users audit columns (REQ-1.12).
- [ ] Dual-backend: identity + provisioning + isolation tests run on SQLite AND the compose Postgres (`--features postgres` + `DATABASE_URL`).
- [ ] `cargo test --workspace` is green and the build is warning-free.

## Implementation Notes

### Technical Approach
Compose `ProdIdentity` into the api's `Arc<dyn Identity>`. Reuse the api oneshot/HTTP harness but drive it on `ProdIdentity`. Isolation tests register two households and assert no cross-tenant reach (read, write, auth, or enumeration). Seed the MVP household via the real register + add-member endpoints rather than a fixture. Run the PG-sensitive tests via docker compose Postgres with `--features postgres` and `DATABASE_URL` set.

### Requirements covered
REQ-1.5, REQ-1.6, REQ-1.10; NFR-2.1, NFR-2.2, NFR-2.4; ADR A-0002, A-0004.

### Dependencies
SQUIRE-T-0019..T-0022; api (T-0018); `store` Provisioner.

## Status Updates

*To be added during implementation*