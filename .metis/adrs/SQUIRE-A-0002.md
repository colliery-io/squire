---
id: 001-tenancy-schema-per-tenant-full
level: adr
title: "Tenancy: schema-per-tenant full isolation, dual-mode local SQLite / hosted Postgres"
number: 1
short_code: "SQUIRE-A-0002"
created_at: 2026-06-17T02:14:12.223252+00:00
updated_at: 2026-06-17T02:17:09.319089+00:00
decision_date: 
decision_maker: 
parent: 
archived: false

tags:
  - "#adr"
  - "#phase/decided"


exit_criteria_met: false
initiative_id: NULL
---

# ADR-1: Tenancy: schema-per-tenant full isolation, dual-mode local SQLite / hosted Postgres

**Status:** Decided (2026-06-16) · **Decision maker:** Dylan Storey
**Relates to:** SQUIRE-A-0003 (Diesel dual-db), SQUIRE-A-0004 (identity/roles), SQUIRE-S-0002 (Persistence), SQUIRE-S-0003 (Local API), SQUIRE-S-0007 (Identity & Registration, planned). Revises vision NFR-5/scope.

## Context **[REQUIRED]**

We want Squire to be **multi-tenant-capable from day one** — nothing in the schema or API may assume a single household — while the MVP still ships as one local Keep on a home computer. We also require that **each tenant be fully isolated** (no risk of one household's data appearing in another's queries). The system must run on SQLite locally and Postgres when hosted (SQUIRE-A-0003).

## Decision **[REQUIRED]**

- **A tenant = a Household.** All of a household's data — its users (Knights/Squires), quest/item/achievement definitions, and the append-only event log — belongs to that household and nothing is shared with another.
- **Isolation model: schema-per-tenant.** Each household lives in its **own Postgres schema** (hosted) or its **own SQLite database file** (local). There are **no shared tables and no `tenant_id`/`household_id` discriminator columns** in household data. Selecting the schema/connection *is* selecting the tenant; cross-tenant queries are structurally impossible.
- **Domain core stays tenant-agnostic.** A `Snapshot` is already exactly one tenant's data, so `Engine::handle` and `Projections` never see a tenant id (AR-7 preserved). Tenancy is resolved at the connection/store boundary, below the pure core.
- **Dual-mode, one codebase.** Local = single-tenant SQLite (one DB file = the household = the Keep). Hosted = multi-tenant Postgres (one schema per household). Same migrations, same `Repository` implementation (SQUIRE-A-0003).
- **Thin control plane (hosted only): a tenant registry** mapping a *household handle* → its schema/connection, used **only to route a request to the right schema after the client presents its household handle**. It holds **no household domain data and no global user directory** — full isolation preserved. In local mode the registry is degenerate (one tenant, no routing). Authentication happens *inside* the resolved tenant schema (SQUIRE-A-0004).
- **Provisioning a household** = create + migrate a fresh schema/db and seed its first Knight (admin). De-provisioning = drop the schema/db.
- **Backups/export** are naturally per-tenant (the existing NFR-4 single-file export = one household).

## Alternatives Analysis **[CONDITIONAL: Complex Decision]**

| Option | Pros | Cons | Risk | Cost |
|--------|------|------|------|------|
| **Schema-per-tenant (chosen)** | Full isolation by construction; no discriminator to forget; clean per-tenant backup/drop; degenerates to a single SQLite file locally | Per-tenant migration runs; many schemas at large scale; routing layer in hosted mode | Low | M |
| Shared schema + `tenant_id` column | One schema, simplest hosted ops; cross-tenant analytics easy | Weakest isolation — every query must filter or it leaks; explicitly *not* what was asked; threads tenant_id through everything (pollutes the pure core) | High (leakage) | M |
| Database-per-tenant (separate DBs/clusters) | Even stronger isolation than schemas | Heavyweight at scale; connection-pool sprawl; overkill vs. schema isolation here | Low | L |

## Rationale **[REQUIRED]**

Schema-per-tenant gives the **full isolation** that was explicitly required *without* a `tenant_id` column threaded through every table and query (one missed filter = a cross-household leak). It keeps the domain core pure (AR-7): the core never learns about tenancy because the connection already scopes it. And it degrades perfectly to the local MVP — "one SQLite file per household" is just the single-tenant case of the same model, so there is no separate local-vs-hosted data design. Database-per-tenant is stricter but unnecessary weight; shared-schema was rejected outright for isolation.

## Consequences **[REQUIRED]**

### Positive
- Cross-tenant data leakage is structurally impossible — there are no cross-tenant rows to leak.
- The pure domain core is untouched by multi-tenancy; tenancy lives only in the store/connection layer.
- Per-household backup, export, and deletion are trivially clean (one schema/file).
- Local MVP is just the single-tenant degenerate case — no divergent data model.

### Negative
- Migrations must be applied across all tenant schemas (a runner that iterates tenants in hosted mode).
- Many schemas at large scale stress connection pooling and catalog size (acceptable for foreseeable scale).
- Hosted mode needs a tenant-routing layer + registry (absent locally).

### Neutral
- Cross-tenant analytics would require deliberate cross-schema aggregation — out of scope.
- The household handle (used for routing + pairing) is established at registration/pairing (SQUIRE-A-0004).