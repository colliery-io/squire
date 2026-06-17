---
id: store-snapshot-atomic-apply-by
level: task
title: "Store: snapshot + atomic apply(by) + audit columns"
short_code: "SQUIRE-T-0010"
created_at: 2026-06-17T04:08:42.088609+00:00
updated_at: 2026-06-17T04:45:10.110145+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Store: snapshot + atomic apply(by) + audit columns

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0002]] (Persistence & Store)

## Objective

Implement `domain_core::contract::Repository` over Diesel: `snapshot()` loads the full `Snapshot` (users + definitions + ordered events) and `apply(by, &[Change])` commits the batch atomically, plus provide the `Clock` impl. Stamp audit columns, enforce append-only and archive-not-delete semantics, and map failures to `RepoError`.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] `snapshot()` returns the full `Snapshot` (users, quests, items, achievements, events ordered by `seq`), decoded via the T-0009 mapping.
- [x] `apply(by, changes)` runs in ONE `conn.transaction` (all-or-nothing): `Append`→insert (seq = max+1; never update/delete); `Put*`→upsert by id; `SetXActive`/`SetUserActive`→toggle `active` (+`updated_*`). Any error rolls back → `RepoError`.
- [x] Audit (A-0007): `Put*`/Set* stamp `updated_by=by`,`updated_at=now`; `created_*` preserved on update (omitted from `.set(...)`); `Append` ignores `by`; `by=None`→NULL.
- [x] No stored availability state (A-0006): items upsert definition+audit columns only; no decrement-on-redeem.
- [x] `RepoError::{Conflict, Io}` mapped (unique/serialization/check/FK→Conflict, else Io); real `SystemClock` + injectable `FixedClock`.
- [x] Tests on **both backends** (`tests/repository.rs`, 5): snapshot round-trip, atomic rollback (store byte-identical), audit stamping (created-vs-updated, None), append-only/seq, Clock Monday-alignment. Same `apply`/`snapshot` over `AnyConnection` run on SQLite + the compose Postgres.

## Implementation Notes

### Technical Approach
Wrap the whole `Change` batch in a single Diesel transaction so it is all-or-nothing; any error rolls back leaving the store unchanged and is mapped to `RepoError::{Conflict, Io}`. Implement upsert portably (`ON CONFLICT` where common, else check-then-insert) behind the repository so backend differences stay hidden. `Append` is insert-only — events are never updated or deleted. Audit columns are stamped per A-0007: `Put*` and the `SetXActive`/`SetUserActive` toggles set `updated_by=by` and `updated_at=Clock::now()` (and `created_*` on first insert), while `Append(Event)` ignores `by`. Per A-0006 there is no stored availability state — items persist definition + audit columns only, with no decrement-on-redeem. The `Clock` impl supplies audit timestamps and is injectable for deterministic tests.

ENV CAVEAT: behavioral tests (snapshot round-trip, atomic rollback, audit stamping, append-only) run on the SQLite path; the same transactional/upsert logic targets Postgres with live verification gated on `DATABASE_URL`.

### Requirements covered
REQ-1.1 / 1.2 / 1.3 / 1.4 / 1.4b / 1.4c / 1.4d / 1.7, NFR-2.3; ADR A-0006 / A-0007.

### Dependencies
SQUIRE-T-0008 (schema/backend), SQUIRE-T-0009 (domain ↔ row mapping).

## Status Updates

**2026-06-17 — Completed.** Rewrote `store::lib`: `Store<C: Clock>` over a `RefCell<AnyConnection>` (single-writer, so no concurrent borrow) implements `Repository`. `snapshot()` selects all five tables (events ordered by `seq`), decodes via T-0009. `apply(by, &[Change])` wraps the batch in `conn.transaction`; `seq=max+1` computed once and incremented per append; `Put*` upsert with audit; Set* toggles `active`+`updated_*` (`Conflict` if id missing). Diesel errors → `Conflict` (unique/serialization/check/FK) else `Io`; a `TxnError` bridges `RepoError` through Diesel's transaction `From<diesel::Error>`.

**Upsert preserves `created_*`:** `insert_into(t).values(row).on_conflict(id).do_update().set(<mutable cols + updated_by/updated_at>)` — `created_*` omitted from `.set`, so a new INSERT takes `created_*=(by,now)` from the row while a conflicting row keeps its original. Wrinkle: Diesel's `MultiConnection` reports no on-conflict support, so upserts dispatch via a `run_upsert!` macro to the concrete `SqliteConnection`/`PgConnection` (both support Pg-style upsert in Diesel 2); plain insert/update/select run on `AnyConnection` directly.

**Clock:** `SystemClock::now()`=unix millis; `today()`=`days_since_unix + 3` (epoch realigned to Monday 1969-12-29) so `Date(0)==Monday` matches the engine's `weekday_of` (unix epoch is a Thursday — without the +3, Weekly schedules would drift 3 days). `FixedClock` for deterministic tests. Audit accessors (`quest_audit`/`user_audit`) added for tests (audit cols aren't in the pure `Snapshot`).

Results: SQLite `cargo test -p store` → 24 (11 mapping + 4 migrations + 5 repository); Postgres (`--features postgres` + compose) → repository (5) + postgres (1) green on PG; `cargo test --workspace` → domain-core 70 intact. Committed.