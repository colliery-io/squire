---
id: persistence-store
level: specification
title: "Persistence & Store"
short_code: "SQUIRE-S-0002"
created_at: 2026-06-17T00:52:20.859105+00:00
updated_at: 2026-06-17T00:52:20.859105+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# Persistence & Store

## Overview **[REQUIRED]**

The Store is the durable, single-writer backend behind the `Repository` port (`snapshot(&self) -> Snapshot`, `apply(&mut self, by: Option<UserId>, &[Change]) -> Result<(), RepoError>`). It is the canonical **per-tenant** household record and the only process permitted to mutate it (AR-1, per tenant). It also supplies the `Clock` port (`today`, `now`) used to stamp events — and, per ADR SQUIRE-A-0007, the last-editor **audit columns** on definition/identity rows. The `by: Option<UserId>` arg names the user applying a change (a Knight for authoring; `None` for system/seed); the Store stamps it (plus the injected `Clock`) onto the audit columns of the affected definition/identity row, and **ignores `by` for `Append(Event)`** (an event already carries its own `actor`/`squire`).

The Store is implemented **once over Diesel as a dual-backend `Repository`** (`diesel-dual-db`): the same repository code compiles and runs against both the `Sqlite` and `Pg` backends, with the concrete backend chosen at startup — **SQLite for the local Keep, Postgres for the hosted deployment** (ADR SQUIRE-A-0003). Backend divergences (`RETURNING`, upsert syntax, type affinity) are isolated behind the `Repository`; DDL is kept to the portable subset common to both backends.

Tenancy is **schema-per-tenant, fully isolated** (ADR SQUIRE-A-0002): each household is its **own Postgres schema** (hosted) or its **own SQLite file** (local). There are **no shared tables and no `tenant_id`/`household_id` discriminator columns** — selecting the schema/connection *is* selecting the tenant, wired per-connection (`search_path` on Postgres, file on SQLite). A `Snapshot` is already exactly one household's data, so the domain core stays tenant-agnostic. The **tenant registry** that routes a household handle to its schema in hosted mode is owned by the Identity component (SQUIRE-S-0007), **not** here; this spec owns only the per-tenant store, the Diesel `Repository`, and the migration/provisioning mechanics.

Storage is split into two shapes (AR-2), all within one tenant schema/file:
- **IDENTITY** — the mutable `User` table (the household's Knights + Squires). Persisted here so events have valid users to attribute to and so the household can be enumerated; rows are upserted and deactivated (not deleted), never authored by `Engine::handle`.
- **DEFINITIONS** — mutable, upsertable rows for `Quest`, `RedeemableItem`, and `Achievement`. Edited freely; never deleted (archive via an `active` flag so historical events stay referentially valid). Per ADR SQUIRE-A-0006 there is **no stored availability counter**: an `Availability::Once` item's out-of-stock state is derived from the event log (the first `ItemRedeemed`), so the Store keeps **no decrement-on-redeem state** for items.
- **ACTIVITY** — an append-only `Event` log. It is the system of record for everything that moves the balance; nothing in it is ever updated or deleted.

Every **definition and identity** table carries last-editor **audit columns** — `created_by` / `created_at` / `updated_by` / `updated_at` — stamped by the Store from the `apply(by, …)` arg and the injected `Clock` (ADR SQUIRE-A-0007). These are **columns only**: the domain types (`User`/`Quest`/`RedeemableItem`/`Achievement`) stay pure and carry no audit fields.

Append-only activity exists because balance, streaks, due lists, and unlock status are *derived* from the log (AR-3) and point values are snapshotted onto approval events (AR-4): rewriting history would corrupt past payouts. The log is the durable source of truth (NFR-4) and must support fast snapshot loads for projections (NFR-8) and raw per-quest/item inspection (NFR-11). The Store owns the per-tenant schema and the `RepoError` enum; it translates each `Change` — `Append`, `PutQuest/PutItem/PutAchievement`, `SetQuestActive/SetItemActive/SetAchievementActive`, and the identity-lifecycle `PutUser`/`SetUserActive` (produced by the Identity component SQUIRE-S-0007, not by `Engine::handle`) — into atomic storage mutations. It holds no business rules — those live in the Domain Core, which produces the `Change` batch the Store merely persists.

## System Context **[CONDITIONAL: System-Level Spec]**

### Actors

All actors are in-process consumers on the computer; the Store has no network surface of its own.

- **Engine (via the Admin App)**: calls `snapshot()` to obtain a `Snapshot` to reason over, then submits the resulting `&[Change]` batch via `apply()`. The Admin App is the host process that drives admin/review commands through the Engine and into the Store.
- **Local API layer**: serves child submissions (claims, redemption requests). Calls `snapshot()` for read state (`StateView`) and routes the resulting `Change` batch through `apply()`. It reads and applies but never bypasses the single-writer entry point.

### External Systems

- **Database backend, via Diesel (`diesel-dual-db`)**: one per-tenant store reached through a single Diesel `Repository`. In **local** mode this is a **SQLite file** (one file = one household = the Keep); in **hosted** mode it is a **Postgres schema** (one schema = one household), selected per-connection (`search_path`). The backend is chosen at startup (ADR SQUIRE-A-0003); the data model is identical either way (ADR SQUIRE-A-0002).
- **Filesystem (export file)**: the destination for the per-tenant single-file backup export — one schema/file = one household = the export (NFR-4).

### Boundaries

- **Inside**: the per-tenant schema (identity/`User` table + definition tables + event log), the Diesel dual-backend `Repository` implementation, event serialization, snapshot loading (incl. `users`), atomic all-or-nothing `apply` of a `Change` batch, append-only enforcement, archive-not-delete semantics (definitions and users), the `RepoError` enum, the `Clock` implementation, raw event-log queries, the Diesel migration set, the per-tenant provisioning/de-provisioning mechanics (create+migrate / drop a schema or file), and per-tenant export.
- **Outside**: business rules and validation (Domain Core / `Engine.handle` — the Store persists whatever `Change`s it is handed without re-checking them); the **tenant registry / household-handle routing** and credential authentication (owned by the Identity component, SQUIRE-S-0007 — the Store is handed an already-resolved connection); projections/derivations (pure functions over a `Snapshot`); and all transport/network concerns (Local API, pairing/auth).

## Requirements **[REQUIRED]**

### Functional Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| REQ-1.1 | `apply(by, &[Change])` commits the whole batch atomically (all-or-nothing) through a single writer; on any failure the store is left unchanged. | AR-1, AR-5 — one validated `Change` batch per command must land indivisibly so the store is never partially mutated. |
| REQ-1.2 | `snapshot()` loads the full `Snapshot` (the household's `users`, all quests, items, achievements, and the ordered event log) for the Engine and projections to read. | AR-3 — every derivation (balance, streaks, due, state view) is computed over a `Snapshot`; `users` backs attribution and household enumeration. |
| REQ-1.3 | Events are append-only: `Change::Append(Event)` inserts a new log row in order; the store never updates or deletes existing event rows. | AR-2, AR-4, NFR-4 — the log is the immutable system of record and snapshotted history must not be rewritten. |
| REQ-1.4 | Definitions are upsertable rows: `PutQuest/PutItem/PutAchievement` insert-or-replace by id; `SetQuestActive/SetItemActive/SetAchievementActive` toggle the `active` flag. Rows are never deleted. | AR-2, FR-A1/A2/A3 — archiving (not deletion) keeps archived definitions referenced by historical events. |
| REQ-1.4b | The store persists the `User` table and accepts `Change::PutUser(User)` (insert-or-replace by id) and `Change::SetUserActive(UserId, bool)` (toggle `active`). These changes are produced by the Identity component (SQUIRE-S-0007), not by `Engine::handle`, but land through the same single writer. Users are deactivated, never deleted, so historical events keep referring to a valid user. | Contract `Change::PutUser`/`SetUserActive`; AR-2 archive-not-delete; identity lives in the store so events have valid referents. |
| REQ-1.4c | `apply` takes `by: Option<UserId>` and every definition/identity table carries last-editor **audit columns** (`created_by`, `created_at`, `updated_by`, `updated_at`). On `PutQuest`/`PutItem`/`PutAchievement`/`PutUser` and `SetQuestActive`/`SetItemActive`/`SetAchievementActive`/`SetUserActive`, the Store stamps `updated_by` = `by` and `updated_at` = `Clock::now()` (setting `created_by`/`created_at` on first insert); `Append(Event)` **ignores `by`** (the event carries its own `actor`/`squire`). Audit lives **only in columns** — the `shared_contract.rs` domain structs stay pure. | ADR SQUIRE-A-0007 — authoring audit; domain types stay pure. |
| REQ-1.4d | The Store keeps **no stored availability counter / decrement-on-redeem state**: an `Availability::Once` item's out-of-stock condition is derived from the event log (first `ItemRedeemed`), not persisted. Items are upserted by id with only their definition columns (plus audit). | ADR SQUIRE-A-0006 — availability is `{ Once, Repeatable }`, out-of-stock derived from the log (AR-3). |
| REQ-1.5 | Provide a **per-tenant** single-file export that writes the entire household store (users + definitions + event log) to one file for backup — one schema/file = one household = one export. | NFR-4 — a single-file backup of a household must be obtainable; export is naturally per-tenant under schema-per-tenant (A-0002). |
| REQ-1.6 | Provide a raw event-log query returning the ordered events touching a given quest or item. | NFR-11 — the admin side must be able to explain how a balance/streak was reached; per-tenant under A-0002. |
| REQ-1.7 | `apply` surfaces failures as `RepoError`: `Conflict` for a write/serialization conflict (e.g. the append-only or single-writer invariant being violated), `Io(String)` for backend/filesystem errors (SQLite or Postgres). | Contract — `apply` returns `Result<(), RepoError>` with exactly these two cases. |
| REQ-1.8 | The `Repository` + `Clock` are implemented **once over Diesel** and run against both the `Sqlite` and `Pg` backends (`diesel-dual-db`); the concrete backend is selected at startup (SQLite local, Postgres hosted). Backend divergences (`RETURNING`, upsert syntax, type affinity) are hidden behind the `Repository`. | ADR SQUIRE-A-0003 — one typed DAL serves both local and hosted; resolves the former rusqlite-vs-sqlx open item. |
| REQ-1.9 | Tenancy is schema-per-tenant with no discriminator columns: each household is its own Postgres schema / SQLite file, selected per-connection (`search_path` on PG, file on SQLite). The Store operates on whatever tenant connection it is handed; `snapshot()`/`apply()` are scoped to that one household. | ADR SQUIRE-A-0002 — full isolation by construction; the registry/routing that picks the connection is owned by SQUIRE-S-0007. |
| REQ-1.10 | Diesel migrations are authored once and **double as per-tenant provisioning**: creating + migrating a fresh schema/file provisions a household; dropping it de-provisions. DDL stays within the portable subset common to both backends. | ADR SQUIRE-A-0002/A-0003 — migrations are the provisioning primitive; portability keeps one DDL set for both backends. |

### Non-Functional Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| NFR-2.1 | The users, event log, and definitions are persisted (to disk locally, to Postgres when hosted) and survive process and host restarts; committed writes are durable. Durability/export are per-tenant. | NFR-4 — the log is the system of record and must not be lost on restart; one schema/file = one household. |
| NFR-2.2 | Schema and indexing support snapshot loads and projection queries that complete in well under 100 ms at single-child scale (dozens of quests, thousands of events accrued over years). | NFR-8 — projections run synchronously and must stay fast as the log grows. |
| NFR-2.3 | Exactly one process writes a given tenant's store; `apply` is the sole mutation path and serializes all writes. Single-writer is per-tenant. | AR-1 — single-writer is a hard architectural constraint, not a convention; scoped to one household under A-0002. |
| NFR-2.4 | Each household is fully isolated: its data lives in its own Postgres schema / SQLite file with no shared tables and no `tenant_id` discriminator, so cross-household queries are structurally impossible. | ADR SQUIRE-A-0002 — full isolation by construction was an explicit requirement. |
| NFR-2.5 | All DDL is kept within the portable subset common to both backends; any unavoidable backend divergence (`RETURNING`, upsert, type affinity) is isolated behind the `Repository` so a single migration set and a single query layer serve both. | ADR SQUIRE-A-0003 — one Diesel DAL and one migration set must run on both SQLite and Postgres. |
| NFR-2.6 | The Repository and migrations are tested against **both** backends; CI exercises SQLite and Postgres. | ADR SQUIRE-A-0003 — two backends in production means both must be covered, or portability regressions go unseen. |

## Architecture Framing **[CONDITIONAL: System-Level Spec]**

### Decision Area: Storage library (`rusqlite` vs `sqlx`) — RESOLVED
- **Context**: Which library backs the Store (PRD §10 open item).
- **Decision**: **RESOLVED — Diesel, dual backend (`diesel-dual-db`).** One `Repository` + `Clock` implemented over Diesel runs against both the `Sqlite` and `Pg` backends, selected at startup (SQLite for the local Keep, Postgres for hosted). rusqlite is disqualified (SQLite-only, blocks the hosted Postgres path); sqlx pushes toward dialect-specific SQL. Backend divergences are isolated behind the `Repository`.
- **ADR**: **SQUIRE-A-0003 (Decided)**.

### Decision Area: Schema-per-tenant migrations & provisioning — RESOLVED
- **Context**: How tenant isolation and household provisioning are realized in the store.
- **Decision**: **RESOLVED — schema-per-tenant, fully isolated (ADR SQUIRE-A-0002).** Each household is its own Postgres schema (hosted) / SQLite file (local); no shared tables, no `tenant_id` discriminator; the connection (`search_path` on PG, file on SQLite) selects the tenant. Diesel migrations are authored once and **double as provisioning**: create + migrate a fresh schema/file to provision a household, drop to de-provision. DDL stays within the portable subset; CI covers both backends. The tenant registry / household-handle routing is **out of scope here** — owned by the Identity component (SQUIRE-S-0007).
- **ADR**: **SQUIRE-A-0002 (Decided)**, with **SQUIRE-A-0003** (Diesel migrations as the mechanism).

### Decision Area: Schema & event serialization (Diesel, portable subset)
- **Context**: Concrete Diesel table layout for the `User` table, definitions, and the event log, and how `Event` variants are stored — typed columns per variant vs a serialized blob (e.g. JSON) plus indexed key columns — within a single tenant schema/file.
- **Constraints**: append-only event log (AR-2); ordered events with snapshotted points preserved verbatim (AR-4); `User`/`Event`/definition types are fixed by `shared_contract.rs`; DDL stays within the portable subset common to `Sqlite` and `Pg`, with backend divergences (`RETURNING`, upsert, type affinity) hidden behind the `Repository` (A-0003).
- **Required Capabilities**: fast full snapshot load incl. `users` and <100 ms projection queries (NFR-8) — implies indices on event ordering and on quest/item keys to back the per-quest/item raw log query (NFR-11); lossless round-trip of every `User`, `Event`, and definition variant; insert-only event writes.
- **ADR**: SQUIRE-A-0003 (Diesel dual-backend) sets the frame; concrete column layout TBD.

### Decision Area: Last-editor audit columns & `apply(by, …)` — framed by ADR SQUIRE-A-0007
- **Context**: ADR SQUIRE-A-0007 adds an authoring audit: `Repository::apply` now takes `by: Option<UserId>`, and every definition/identity table grows last-editor columns (`created_by`, `created_at`, `updated_by`, `updated_at`). The domain types stay pure — audit is a storage concern only.
- **Decision (ADR SQUIRE-A-0007)**: The Store stamps `updated_by` from `by` and `updated_at` from the injected `Clock` on every `PutQuest`/`PutItem`/`PutAchievement`/`PutUser` and `SetXActive` mutation (setting `created_*` on first insert); `Append(Event)` ignores `by` (the event already carries `actor`/`squire`). Audit columns never round-trip into the `shared_contract.rs` structs.
- **Constraints**: Columns only (no contract-struct changes); portable subset DDL across SQLite/Pg; the `Clock` port supplies `created_at`/`updated_at`; per-tenant under A-0002. Open: nullability of `created_by`/`updated_by` (`None`/`by` for system/seed) and whether to store as a separate audit shape vs inline columns.
- **ADR**: SQUIRE-A-0007 (Decided); concrete column layout TBD.

### Decision Area: No stored availability state — ADR SQUIRE-A-0006
- **Context**: With `Availability` simplified to `{ Once, Repeatable }` and out-of-stock derived from the event log, the Store must **not** persist any availability counter or decrement items on redeem.
- **Constraint (ADR SQUIRE-A-0006)**: Items are upserted with their definition columns (plus audit) only; `Once` out-of-stock is computed from the first `ItemRedeemed` in the log (AR-3), so there is no stored stock/limit field and no redeem-time mutation of the item row. `ItemRedeemed` is just another append-only event.
- **ADR**: SQUIRE-A-0006 (Decided).

### Decision Area: Per-tenant export format
- **Context**: How the per-tenant single-file backup is produced — a raw backend copy (e.g. SQLite `VACUUM INTO` / online backup, or `pg_dump` of the schema) vs a serialized snapshot dump — given two backends.
- **Constraints**: must capture one household's entire store (users + definitions + complete event log) in one file (NFR-4); one schema/file = one household = one export (A-0002); must be consistent with respect to the single per-tenant writer (no torn export mid-`apply`).
- **Required Capabilities**: atomic/consistent point-in-time capture per tenant, restorable back into the store, on both backends.
- **ADR**: TBD (framed by A-0002 per-tenant export and A-0003 dual backend).

## Constraints **[CONDITIONAL: Has Constraints]**

### Technical Constraints
- Single-writer (per-tenant): exactly one process mutates a given household's store, via `apply` only (AR-1).
- Activity is append-only: event rows are inserted, never updated or deleted (AR-2).
- Definitions are never deleted — archive via the `active` flag so historical events stay valid (AR-2, FR-A1/A2/A3). The same archive-not-delete rule applies to `User` rows (`SetUserActive`).
- Contract types are fixed: `Snapshot` (incl. `users`), `Change` (incl. `PutUser`/`SetUserActive`), `Event`, `User`, the definition structs, `RepoError`, and the `Repository`/`Clock` traits are defined in `shared_contract.rs` and cannot be altered by the Store. `Repository::apply` now takes `by: Option<UserId>` (ADR SQUIRE-A-0007). `PutUser`/`SetUserActive` are produced by the Identity component (SQUIRE-S-0007), not by `Engine::handle`.
- Last-editor audit is **columns only** (ADR SQUIRE-A-0007): every definition/identity table carries `created_by`/`created_at`/`updated_by`/`updated_at`, stamped from `apply`'s `by` and the `Clock`; `Append(Event)` ignores `by`. The domain structs stay pure — no audit fields leak into `shared_contract.rs` types.
- **No stored availability counter** (ADR SQUIRE-A-0006): `Availability` is `{ Once, Repeatable }`; `Once` out-of-stock is derived from the event log (first `ItemRedeemed`), so the Store persists no stock/limit field and performs no decrement-on-redeem.
- Persistence is **Diesel, dual backend** (`diesel-dual-db`): one `Repository`/`Clock` over both `Sqlite` (local Keep) and `Pg` (hosted), chosen at startup (ADR SQUIRE-A-0003).
- Tenancy is **schema-per-tenant, fully isolated**: each household is its own Postgres schema / SQLite file, selected per-connection (`search_path` / file); no shared tables, no `tenant_id` discriminator (ADR SQUIRE-A-0002). The tenant registry / household-handle routing is owned by SQUIRE-S-0007, not the Store.
- DDL stays within the **portable subset** common to both backends; backend divergences (`RETURNING`, upsert, type affinity) are hidden behind the `Repository`. Diesel migrations are authored once and double as per-tenant provisioning (create+migrate / drop). CI covers both backends.
- Implemented in Rust on the Keep side (AR-6); no I/O dependency leaks into the domain core (AR-7/NFR-10).