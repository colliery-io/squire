---
id: identity-tenant-registry
level: task
title: "Identity: tenant registry + provisioning/routing"
short_code: "SQUIRE-T-0021"
created_at: 2026-06-17T09:52:22.732130+00:00
updated_at: 2026-06-17T10:09:49.275628+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Identity: tenant registry + provisioning/routing

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0007]] (Identity, Tenancy & Registration)

## Objective

In `crates/identity`, implement the tenant registry plus provisioning/routing on top of `store::Provisioner`. This task RESOLVES the "tenant-registry storage & routing" and "how provisioning invokes migrations" decision areas. A tenant is a Household, fully isolated per ADR A-0002 (SQLite file-per-tenant / Postgres schema-per-tenant).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] `TenantRegistry::local(backend, handle)` (degenerate single-tenant — resolves the one bound handle, else `WrongTenant`) / `TenantRegistry::hosted(backend)` (a `Mutex<HashSet<HouseholdHandle>>` of known handles — routing metadata only, no household data / user list).
- [x] `provision(handle)`/`deprovision(handle)`/`resolve(handle) -> Store<SystemClock>` delegate create+migrate/drop/open to `store::Provisioner` (SQLite file / PG schema); `TenantError` wraps `ProvisionError` + `UnknownTenant`/`WrongTenant`.
- [x] No cross-tenant reach; no global user directory (NFR-2.1) — unknown/cross handle rejected.
- [x] Tests (`tests/tenant.rs`, both backends): local resolve-own/reject-other; hosted two-household isolation (each sees only its own), un-provisioned→error, deprovision(alpha)→resolve fails while beta untouched. SQLite always + compose Postgres schemas under `--features postgres`+`DATABASE_URL`.

## Implementation Notes

### Technical Approach
Wrap `store::Provisioner` (T-0011). The local registry is trivial (single store, identity routing). The hosted registry is an in-memory handle→schema map. Document the storage choice: in-memory for the MVP, with persistence treated as a hosted-ops concern (the registry holds only routing metadata, never household or user data). Provisioning invokes the store's migration set when creating each tenant.

### Requirements covered
REQ-1.6, REQ-1.7, REQ-1.8; NFR-2.1; resolves the registry-storage/routing and provisioning-migration decision areas; ADR A-0002, A-0003.

### Dependencies
SQUIRE-T-0019; `store::Provisioner` (T-0011).

## Status Updates

**2026-06-17 — Completed.** `crates/identity/src/tenant.rs`: `TenantRegistry` over `store::tenant::Backend`, modes `local(backend, handle)` and `hosted(backend)`. **Registry storage/routing (decision):** routing metadata only — local mode holds the single bound handle; hosted mode a `Mutex<HashSet<HouseholdHandle>>` of known tenants (no household domain data, no user directory — NFR-2.1); persisting the known-set is a hosted-ops concern (in-memory for MVP). **Provisioning→migrations (decision):** the registry never reimplements migrations — `provision`/`resolve`/`deprovision` delegate to `store::tenant::Provisioner` (which runs the embedded Diesel migrations on the tenant's SQLite file / `t_<handle>` PG schema); S-0002 stays the sole owner of the schema. `TenantError` wraps `ProvisionError` + `UnknownTenant`/`WrongTenant`. Added a `postgres` feature on identity (`= ["store/postgres"]`).

Tests `tests/tenant.rs` (2, both backends): local resolve-own/reject-other; hosted two-household isolation + un-provisioned→error + deprovision lifecycle. SQLite via `TempDir`; Postgres schemas via compose (Mutex-serialized, cleaned). Results: `cargo test -p identity` → 15 + 2 green; PG run (`--features postgres`+env) → 15 + 2 green on PG; `cargo test --workspace` green; 0 warnings. Committed.