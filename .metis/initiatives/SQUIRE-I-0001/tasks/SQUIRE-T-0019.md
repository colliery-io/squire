---
id: identity-crate-extraction-port
level: task
title: "Identity: crate extraction & port consolidation"
short_code: "SQUIRE-T-0019"
created_at: 2026-06-17T09:52:20.388701+00:00
updated_at: 2026-06-17T10:00:21.960049+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

## Acceptance Criteria

- [x] New `crates/identity` crate (deps `domain-core`+`store`), added to workspace `members`.
- [x] `Identity`/`Principal`/`AuthError`/`DevIdentity` (+ `SharedStore` alias + 6 unit tests) moved verbatim into `crates/identity/src/lib.rs`; api's `identity.rs` deleted.
- [x] `api` depends on `identity`; `AppState` holds `Arc<dyn identity::Identity>`; auth/handlers/tests use `identity::{...}`. Dependency direction `api → identity → {store, domain-core}` — **no cycle** (confirmed via `cargo tree`).
- [x] `cargo test --workspace` green (domain-core 70, store 27, api 38 unchanged, identity 6); 0 warnings. No behavior/signature/assertion changes.

## Implementation Notes

### Technical Approach
Mechanical move plus `use`-path updates. `DevIdentity` (which shares `store::SharedStore`) moves into the new crate as well. Keep the public surface (trait shape, type names, error variants) stable so `api` compiles with only `use`-path edits. Add the crate to the workspace `members`, wire `api`'s `Cargo.toml` to depend on `identity`, and re-point `AppState`, the auth extractor, handlers, and tests at `identity::{...}`.

### Requirements covered
Foundational for S-0007 — enables the A-0004 per-user production impl by breaking the api↔identity cycle. No new REQ/NFR introduced by this task.

### Dependencies
`api` (T-0014..T-0018), `store`, `domain-core`.

## Status Updates

**2026-06-17 — Completed.** Pure refactor: created `crates/identity` (deps `domain-core`+`store`) and moved `Identity`/`Principal`/`AuthError`/`DevIdentity` + the `SharedStore` alias + 6 unit tests out of `crates/api/src/identity.rs` into `crates/identity/src/lib.rs` (deleted api's `identity.rs`). `api` now depends on `identity`; `state.rs` re-exports `identity::SharedStore` so internal `crate::state::SharedStore` uses still resolve; `auth.rs`/`control.rs`/`knight.rs`/`lib.rs` + all `tests/*.rs` re-pointed to `identity::{...}`. `AppState` holds `Arc<dyn identity::Identity>`. Direction `api → identity → {store, domain-core}`, no cycle (cargo tree). All behavior/assertions unchanged. `cargo test --workspace` green (domain-core 70, store 27, api 38, identity 6), 0 warnings. Committed.