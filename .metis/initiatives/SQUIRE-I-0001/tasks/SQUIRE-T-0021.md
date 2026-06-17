---
id: identity-tenant-registry
level: task
title: "Identity: tenant registry + provisioning/routing"
short_code: "SQUIRE-T-0021"
created_at: 2026-06-17T09:52:22.732130+00:00
updated_at: 2026-06-17T09:52:22.732130+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Identity: tenant registry + provisioning/routing

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0007]] (Identity, Tenancy & Registration)

## Objective

In `crates/identity`, implement the tenant registry plus provisioning/routing on top of `store::Provisioner`. This task RESOLVES the "tenant-registry storage & routing" and "how provisioning invokes migrations" decision areas. A tenant is a Household, fully isolated per ADR A-0002 (SQLite file-per-tenant / Postgres schema-per-tenant).

## Acceptance Criteria

- [ ] `TenantRegistry` maps `HouseholdHandle → tenant store`: **local mode** is a degenerate single-tenant registry (one store, trivial routing); **hosted mode** is a thin handle→schema map holding ONLY routing metadata (no household data, no global user list).
- [ ] `provision(handle)` creates and migrates a fresh tenant via `store::Provisioner` (SQLite file / PG schema); `deprovision(handle)` drops it; `resolve(handle) -> tenant Store`.
- [ ] No cross-tenant reach and no global user directory (NFR-2.1).
- [ ] Tests: local degenerate routing; provision two households → each store sees only its own data (full isolation); deprovision; dual-backend exercising the hosted/schema path against the compose Postgres.

## Implementation Notes

### Technical Approach
Wrap `store::Provisioner` (T-0011). The local registry is trivial (single store, identity routing). The hosted registry is an in-memory handle→schema map. Document the storage choice: in-memory for the MVP, with persistence treated as a hosted-ops concern (the registry holds only routing metadata, never household or user data). Provisioning invokes the store's migration set when creating each tenant.

### Requirements covered
REQ-1.6, REQ-1.7, REQ-1.8; NFR-2.1; resolves the registry-storage/routing and provisioning-migration decision areas; ADR A-0002, A-0003.

### Dependencies
SQUIRE-T-0019; `store::Provisioner` (T-0011).

## Status Updates

*To be added during implementation*