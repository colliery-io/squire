---
id: store-schema-per-tenant
level: task
title: "Store: schema-per-tenant provisioning & connection selection"
short_code: "SQUIRE-T-0011"
created_at: 2026-06-17T04:08:43.565492+00:00
updated_at: 2026-06-17T04:08:43.565492+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Store: schema-per-tenant provisioning & connection selection

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0002]] (Persistence & Store)

## Objective

Provide tenant-store lifecycle: `provision` (create + migrate) and `deprovision` (drop) a tenant store, and select backend/connection at startup. SQLite uses file-per-tenant; Postgres uses schema-per-tenant via `search_path`. The store operates on a handed connection; the household-handle → connection registry/routing is owned by S-0007 and explicitly out of scope here.

## Acceptance Criteria

- [ ] `provision(handle)` creates + migrates a fresh tenant store (new SQLite file, or new PG schema + migrate); `deprovision(handle)` drops it; behaves sensibly on already-exists / missing.
- [ ] Backend selected at startup (config/URL): SQLite → file; Postgres URL → schema (`search_path` per connection).
- [ ] No `tenant_id` columns anywhere; isolation is structural (NFR-2.4).
- [ ] Tests (SQLite, file-per-tenant): provision two households, write different data to each, assert each `snapshot()` sees only its own; deprovision removes it.

## Implementation Notes

### Technical Approach
SQLite: each handle maps to its own database file path; provisioning creates the file and migrates it, deprovisioning deletes it. Postgres: `CREATE SCHEMA` per tenant then migrate with the connection's `search_path` set to that schema; deprovisioning drops the schema. Connection acquisition sits behind a small factory that picks the backend from config/URL at startup. Isolation is structural — no `tenant_id` columns anywhere (NFR-2.4). The household-handle → connection registry/routing is owned by S-0007 and is NOT built here; this task only provisions/deprovisions and hands back connections.

ENV CAVEAT: isolation is tested on the SQLite file-per-tenant path; the Postgres schema-per-tenant path compiles with portable DDL and is verified live only when `DATABASE_URL` is set.

### Requirements covered
REQ-1.9 / 1.10, NFR-2.4; ADR A-0002.

### Dependencies
SQUIRE-T-0008 (schema/backend abstraction + migration runner).

## Status Updates

*To be added during implementation*