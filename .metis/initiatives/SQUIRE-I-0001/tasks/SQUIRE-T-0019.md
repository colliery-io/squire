---
id: identity-crate-extraction-port
level: task
title: "Identity: crate extraction & port consolidation"
short_code: "SQUIRE-T-0019"
created_at: 2026-06-17T09:52:20.388701+00:00
updated_at: 2026-06-17T09:56:13.477427+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Identity: crate extraction & port consolidation

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0007]] (Identity, Tenancy & Registration)

## Objective

Create a dedicated `crates/identity` crate and move the `Identity` port out of `crates/api` so that S-0007 can supply a production implementation without introducing a dependency cycle. The dependency direction must become `api → identity → store/domain-core`. This is a mechanical extraction that keeps the public surface stable so the `api` crate compiles with minimal edits.

## Acceptance Criteria

## Acceptance Criteria

- [ ] New `crates/identity` crate exists (deps: `domain-core` + `store`), added to the workspace `members` list.
- [ ] `Identity` trait + `Principal` + `AuthError` + `DevIdentity` moved from `crates/api/src/identity.rs` into `crates/identity`; api's `identity.rs` is removed (or reduced to a thin re-export).
- [ ] `api` depends on `crates/identity`; `AppState` holds `Arc<dyn identity::Identity>`; the auth extractor + handlers use `identity::{Principal, AuthError}`; api tests use `identity::DevIdentity`. NO dependency cycle.
- [ ] `cargo test --workspace` is green (api's 38 tests unchanged) and the build is warning-free.

## Implementation Notes

### Technical Approach
Mechanical move plus `use`-path updates. `DevIdentity` (which shares `store::SharedStore`) moves into the new crate as well. Keep the public surface (trait shape, type names, error variants) stable so `api` compiles with only `use`-path edits. Add the crate to the workspace `members`, wire `api`'s `Cargo.toml` to depend on `identity`, and re-point `AppState`, the auth extractor, handlers, and tests at `identity::{...}`.

### Requirements covered
Foundational for S-0007 — enables the A-0004 per-user production impl by breaking the api↔identity cycle. No new REQ/NFR introduced by this task.

### Dependencies
`api` (T-0014..T-0018), `store`, `domain-core`.

## Status Updates

*To be added during implementation*