---
id: store-crate-scaffold-diesel-dual
level: task
title: "Store: crate scaffold, Diesel dual-backend schema & migrations"
short_code: "SQUIRE-T-0008"
created_at: 2026-06-17T04:08:39.757127+00:00
updated_at: 2026-06-17T04:25:45.312982+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Store: crate scaffold, Diesel dual-backend schema & migrations

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0002]] (Persistence & Store)

## Objective

Create the new `crates/store` Rust crate (depending on `domain-core`), wire Diesel for a dual backend (SQLite + Postgres), author the full migration set (all tables including audit columns and indices), generate the Diesel schema, and provide a connection/backend abstraction plus a migration runner. SQLite is the fully tested path; Postgres compiles against portable DDL with live integration gated on `DATABASE_URL`.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] `crates/store` builds; depends on `domain-core`, Diesel 2.2 (`sqlite` + bundled `libsqlite3-sys`; `postgres` is an **optional feature, off by default**), `diesel_migrations`. Default build needs no system libpq.
- [x] One migration creates `users`/`quests`/`items`/`achievements` (each with `active` + audit cols `created_by/created_at/updated_by/updated_at`) and an append-only `events` table + indices (seq order, quest/item keys, squire); **portable subset** DDL.
- [x] Backend abstraction: `AnyConnection` via `#[derive(MultiConnection)]` — `Sqlite` always, `#[cfg(feature="postgres")] Pg`.
- [x] `run_migrations` + `SqliteStore::open` (migrates on open); temp-file SQLite tests migrate clean.
- [x] **Dual-backend tested via Docker Compose** (user directive): `docker-compose.yml` runs Postgres; a `postgres`-gated test (`store::pg::provision_clean`) resets the schema, migrates, and verifies all tables on **real Postgres**. `cargo test -p store` green on SQLite (4); PG migration test green via compose (1). *(This caught a real portability bug — see notes.)*

## Implementation Notes

### Technical Approach
Prefer Diesel 2.x `MultiConnection` to wrap `SqliteConnection`/`PgConnection` behind a single enum so repository code is backend-agnostic. Author migrations as plain portable SQL (the common SQLite/Postgres subset); isolate any unavoidable divergence behind the repository layer rather than in the DDL. RESOLVE the spec's "schema & event serialization" column-layout decision area here and record the chosen layout (table columns, id encoding, event discriminator/typed-column shape) in the task log so T-0009/T-0010 build on a fixed contract.

ENV CAVEAT: SQLite is the fully testable path. Postgres support compiles with portable DDL, but live PG integration tests are gated on `DATABASE_URL` and skipped when it is unset.

### Requirements covered
REQ-1.8, REQ-1.10, NFR-2.5; ADR A-0002 / A-0003.

### Dependencies
domain-core (T-0001..T-0007). PG integration gated on the `DATABASE_URL` env var.

## Status Updates

**2026-06-17 — Completed.** New `crates/store` crate. **Chosen event/serialization layout (resolves the spec's schema decision area):** typed-nullable-columns keyed by a `kind` discriminant — one `events` row per `Event`, union of variant fields across nullable typed columns (`claim_id/quest_id/on_date/points/amount/reason/request_id/command_id/item_id/achievement_id`), every row carries `squire`+`at`, `actor` NULL for auto/system. `seq` is an **application-assigned** `BIGINT PRIMARY KEY` (the single-writer `apply` sets `max(seq)+1`). Ids = decimal-string TEXT; enums = TEXT discriminants; sets (`assignment_squires`, `cadence_weekdays`) = comma-delimited TEXT; bools = INTEGER 0/1; Points/Date/Timestamp = BIGINT. Definition+identity tables carry the four audit columns. Indices: seq (order), quest_id, item_id, squire. `AnyConnection` (`#[derive(MultiConnection)]`, Pg arm under feature); `run_migrations`; `SqliteStore::open`.

**Docker Compose dual-backend testing (per user directive):** added `docker-compose.yml` (Postgres 16 on :55432, tmpfs, healthcheck). `store::pg::provision_clean` (postgres-gated) resets `public`, migrates, and verifies all tables on real Postgres. Run via `docker compose up -d postgres` + `PQ_LIB_DIR=$(pg_config --libdir) DYLD_FALLBACK_LIBRARY_PATH=$(pg_config --libdir) DATABASE_URL=postgres://squire:squire@localhost:55432/squire_test cargo test -p store --features postgres`. The `postgres` feature is optional/off-by-default (libpq from MacPorts `/opt/local`).

**Portability bug caught by the PG run:** the initial migration used SQLite's `INTEGER PRIMARY KEY AUTOINCREMENT`, which Postgres rejects (`syntax error at "AUTOINCREMENT"`). Fixed by making `seq` an app-assigned `BIGINT PRIMARY KEY` — fully portable; the writer owns ordering. Exactly the class of regression NFR-2.6 (test both backends) exists to catch.

Results: SQLite `cargo test -p store` → 4 passed; Postgres migration test via compose → 1 passed; `cargo test --workspace` → all green (domain-core 70 intact). Committed `b81aa82`.