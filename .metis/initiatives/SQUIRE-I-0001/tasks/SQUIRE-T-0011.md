---
id: store-schema-per-tenant
level: task
title: "Store: schema-per-tenant provisioning & connection selection"
short_code: "SQUIRE-T-0011"
created_at: 2026-06-17T04:08:43.565492+00:00
updated_at: 2026-06-17T04:49:50.713644+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Store: schema-per-tenant provisioning & connection selection

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0002]] (Persistence & Store)

## Objective

Provide tenant-store lifecycle: `provision` (create + migrate) and `deprovision` (drop) a tenant store, and select backend/connection at startup. SQLite uses file-per-tenant; Postgres uses schema-per-tenant via `search_path`. The store operates on a handed connection; the household-handle → connection registry/routing is owned by S-0007 and explicitly out of scope here.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] `Provisioner::provision(handle)` creates + migrates (SQLite file `<dir>/<handle>.sqlite`; PG `CREATE SCHEMA t_<handle>` + `search_path` + migrate); `deprovision` drops it (delete file / `DROP SCHEMA … CASCADE`, idempotent on missing); `open` hands back a tenant-scoped `Store`.
- [x] Backend chosen at startup via a `Backend` config enum (`Sqlite{dir}` / `Postgres{base_url}`); PG connections `SET search_path` to the tenant schema immediately.
- [x] No `tenant_id` columns; isolation is structural (NFR-2.4). Handles sanitized to `[a-z0-9_]`, ≤48 chars, else `InvalidHandle` (no path-traversal/SQL-injection).
- [x] Tests on **both backends** (`tests/tenant.rs`, 3): full isolation (two households see only their own data), provision→use→deprovision→re-provision lifecycle, handle rejection — SQLite file-per-tenant + the compose Postgres schema-per-tenant.

## Implementation Notes

### Technical Approach
SQLite: each handle maps to its own database file path; provisioning creates the file and migrates it, deprovisioning deletes it. Postgres: `CREATE SCHEMA` per tenant then migrate with the connection's `search_path` set to that schema; deprovisioning drops the schema. Connection acquisition sits behind a small factory that picks the backend from config/URL at startup. Isolation is structural — no `tenant_id` columns anywhere (NFR-2.4). The household-handle → connection registry/routing is owned by S-0007 and is NOT built here; this task only provisions/deprovisions and hands back connections.

ENV CAVEAT: isolation is tested on the SQLite file-per-tenant path; the Postgres schema-per-tenant path compiles with portable DDL and is verified live only when `DATABASE_URL` is set.

### Requirements covered
REQ-1.9 / 1.10, NFR-2.4; ADR A-0002.

### Dependencies
SQUIRE-T-0008 (schema/backend abstraction + migration runner).

## Status Updates

**2026-06-17 — Completed.** New `crates/store/src/tenant.rs`: `Backend` config (`Sqlite{dir}` / `#[cfg(postgres)] Postgres{base_url}`), `Provisioner` with `provision`/`open`/`open_conn`/`deprovision`, `ProvisionError`, `sanitize_handle`. SQLite: handle → `<dir>/<handle>.sqlite` (create+migrate / delete). Postgres: handle → schema `t_<handle>` (`CREATE SCHEMA IF NOT EXISTS` + `SET search_path` + migrate; `DROP SCHEMA … CASCADE`); every opened PgConnection `SET search_path TO t_<handle>` so unqualified table names resolve only to that tenant's schema (no `public`, no cross-tenant visibility). `open` returns a `Store<C>` whose connection is already tenant-scoped, so T-0010's `snapshot`/`apply` are unchanged. **Handle rule:** `[a-z0-9_]`, non-empty, ≤48 chars (under PG's 63-byte id limit), else `InvalidHandle` — rejection (not escaping) makes file/schema names always safe to interpolate. Registry/routing (handle→tenant for a request) is explicitly S-0007, not built here.

Tests `tests/tenant.rs` (3, both backends): full isolation (two households see only their own data), provision→use→deprovision→re-provision lifecycle, handle rejection. SQLite via `TempDir`; Postgres schemas via compose (serialized behind a Mutex, cleaned pre/post). Results: SQLite `cargo test -p store` → 23 green (incl. tenant 3); Postgres run → tenant 3 green on real PG schemas; `cargo test --workspace` → domain-core 70 intact. Committed.