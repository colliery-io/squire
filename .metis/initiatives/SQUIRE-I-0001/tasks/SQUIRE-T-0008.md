---
id: store-crate-scaffold-diesel-dual
level: task
title: "Store: crate scaffold, Diesel dual-backend schema & migrations"
short_code: "SQUIRE-T-0008"
created_at: 2026-06-17T04:08:39.757127+00:00
updated_at: 2026-06-17T04:14:00.845729+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


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

- [ ] `crates/store` builds; depends on `domain-core` (uses `domain_core::contract` types), Diesel (features `sqlite` bundled + `postgres`), and `diesel_migrations`.
- [ ] One migration set creates `users`, `quests`, `items`, `achievements` (each with `active` + audit cols `created_by/created_at/updated_by/updated_at`) and an append-only `events` table, plus indices for event ordering and quest/item keys; DDL stays in the portable subset (no backend-specific syntax).
- [ ] A backend abstraction (Diesel 2.x `#[derive(MultiConnection)]` enum, or generic-over-`Backend` code) lets the same repository code target `Sqlite` and `Pg`, chosen at startup.
- [ ] A migration runner migrates a fresh connection; a test creates a temp-file/in-memory SQLite DB and migrates it clean.
- [ ] `cargo test -p store` green on SQLite; Postgres path compiles; live PG tests gated on `DATABASE_URL`.

## Implementation Notes

### Technical Approach
Prefer Diesel 2.x `MultiConnection` to wrap `SqliteConnection`/`PgConnection` behind a single enum so repository code is backend-agnostic. Author migrations as plain portable SQL (the common SQLite/Postgres subset); isolate any unavoidable divergence behind the repository layer rather than in the DDL. RESOLVE the spec's "schema & event serialization" column-layout decision area here and record the chosen layout (table columns, id encoding, event discriminator/typed-column shape) in the task log so T-0009/T-0010 build on a fixed contract.

ENV CAVEAT: SQLite is the fully testable path. Postgres support compiles with portable DDL, but live PG integration tests are gated on `DATABASE_URL` and skipped when it is unset.

### Requirements covered
REQ-1.8, REQ-1.10, NFR-2.5; ADR A-0002 / A-0003.

### Dependencies
domain-core (T-0001..T-0007). PG integration gated on the `DATABASE_URL` env var.

## Status Updates

*To be added during implementation*