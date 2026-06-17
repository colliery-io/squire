---
id: 001-persistence-diesel-with-dual
level: adr
title: "Persistence: Diesel with dual backend (Postgres + SQLite)"
number: 1
short_code: "SQUIRE-A-0003"
created_at: 2026-06-17T02:14:13.150807+00:00
updated_at: 2026-06-17T02:17:10.272052+00:00
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

# ADR-1: Persistence: Diesel with dual backend (Postgres + SQLite)

**Status:** Decided (2026-06-16) · **Decision maker:** Dylan Storey
**Relates to:** SQUIRE-A-0002 (schema-per-tenant), SQUIRE-S-0002 (Persistence). **Resolves** the S-0002 / PRD §10 open item "rusqlite vs sqlx".

## Context **[REQUIRED]**

With dual-mode tenancy (SQUIRE-A-0002) the store must run on **SQLite locally** and **Postgres when hosted**, interchangeably. The contract's `Repository` (snapshot/apply, single-writer) and `Clock` need a single implementation that targets both backends. The PRD left the storage library open (rusqlite vs sqlx).

## Decision **[REQUIRED]**

- Use **Diesel** as the typed query/ORM layer, configured for **dual backend** (the `diesel-dual-db` pattern): the same repository code compiles and runs against both the `Sqlite` and `Pg` backends, selected by deployment/config.
- Implement the contract `Repository` + `Clock` **once over Diesel**; the concrete backend is chosen at startup (SQLite for the local Keep, Postgres for hosted).
- Author **migrations once** with Diesel's migration system; apply them **per-tenant-schema** (the provisioning mechanism from SQUIRE-A-0002). Keep DDL within the **portable subset** common to both backends; isolate any unavoidable backend divergence (e.g. `RETURNING`, upsert syntax, type affinity) behind the `Repository`.
- Map the append-only event log + definition tables to Diesel tables **within each tenant schema**. Single-writer (AR-1) is preserved per tenant.

## Alternatives Analysis **[CONDITIONAL: Complex Decision]**

| Option | Pros | Cons | Risk | Cost |
|--------|------|------|------|------|
| **Diesel, dual backend (chosen)** | One typed DAL for both backends; first-class migrations that double as per-tenant provisioning; schema types checked at compile time | Must stay within the portable dialect subset; some features differ per backend and need care; async story / pooling needs setup | Low | M |
| sqlx | Async, compile-time-checked queries, supports PG + SQLite | Tends toward raw SQL per dialect (two query sets) or careful hand-portability; less ORM ergonomics for schema-per-tenant | Medium | M |
| rusqlite | Simple, fast for SQLite | **SQLite only — no Postgres path**; would block the hosted multi-tenant mode | High | S |

## Rationale **[REQUIRED]**

Diesel's dual-backend support lets us keep **one `Repository` implementation** across SQLite and Postgres rather than maintaining two query sets, and its migration system is exactly the per-tenant-schema provisioning primitive that SQUIRE-A-0002 needs. rusqlite is disqualified by the Postgres requirement; sqlx is viable but pushes toward dialect-specific SQL and was not the chosen direction. This is a directed decision (use `diesel-dual-db`).

## Consequences **[REQUIRED]**

### Positive
- A single typed data-access layer serves both local (SQLite) and hosted (Postgres) deployments.
- Diesel migrations are reused as the household-provisioning mechanism (create + migrate a schema).
- Compile-time schema checking reduces a class of query/runtime errors.

### Negative
- Schema/DDL is constrained to the portable subset; backend-specific behavior must be hidden behind the `Repository`.
- Diesel's async/connection-pooling and per-connection schema selection (`search_path` on PG, file on SQLite) need deliberate setup.
- Two backends to test (CI must cover both).

### Neutral
- Per-request tenant schema selection is wired at the connection layer (SQUIRE-A-0002), transparent to the domain core.
- ORM choice is encapsulated by the `Repository` port, so it remains swappable in principle (NFR-10).