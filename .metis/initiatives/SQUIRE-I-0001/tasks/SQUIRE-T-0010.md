---
id: store-snapshot-atomic-apply-by
level: task
title: "Store: snapshot + atomic apply(by) + audit columns"
short_code: "SQUIRE-T-0010"
created_at: 2026-06-17T04:08:42.088609+00:00
updated_at: 2026-06-17T04:34:34.165904+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


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

- [ ] `snapshot()` returns the full `Snapshot` (users, quests, items, achievements, events in append order).
- [ ] `apply(by, changes)` runs in ONE transaction (all-or-nothing): `Append`→insert event (never update/delete); `PutQuest/Item/Achievement` / `PutUser`→upsert by id; `SetXActive` / `SetUserActive`→toggle `active`. Any error rolls back (store unchanged) → `RepoError`.
- [ ] Audit (A-0007): `Put*` / `SetXActive` / `SetUserActive` stamp `updated_by=by`, `updated_at=Clock::now()` (+ `created_*` on first insert); `Append(Event)` ignores `by`.
- [ ] No stored availability state (A-0006): items upsert definition + audit columns only; no decrement-on-redeem.
- [ ] `RepoError::{Conflict, Io}` surfaced; a real injectable `Clock` impl provided.
- [ ] Tests (SQLite): snapshot round-trip after applies; atomic rollback on a forced mid-batch failure; audit stamping (created vs updated, `None` for seed); append-only (events never mutated).

## Implementation Notes

### Technical Approach
Wrap the whole `Change` batch in a single Diesel transaction so it is all-or-nothing; any error rolls back leaving the store unchanged and is mapped to `RepoError::{Conflict, Io}`. Implement upsert portably (`ON CONFLICT` where common, else check-then-insert) behind the repository so backend differences stay hidden. `Append` is insert-only — events are never updated or deleted. Audit columns are stamped per A-0007: `Put*` and the `SetXActive`/`SetUserActive` toggles set `updated_by=by` and `updated_at=Clock::now()` (and `created_*` on first insert), while `Append(Event)` ignores `by`. Per A-0006 there is no stored availability state — items persist definition + audit columns only, with no decrement-on-redeem. The `Clock` impl supplies audit timestamps and is injectable for deterministic tests.

ENV CAVEAT: behavioral tests (snapshot round-trip, atomic rollback, audit stamping, append-only) run on the SQLite path; the same transactional/upsert logic targets Postgres with live verification gated on `DATABASE_URL`.

### Requirements covered
REQ-1.1 / 1.2 / 1.3 / 1.4 / 1.4b / 1.4c / 1.4d / 1.7, NFR-2.3; ADR A-0006 / A-0007.

### Dependencies
SQUIRE-T-0008 (schema/backend), SQUIRE-T-0009 (domain ↔ row mapping).

## Status Updates

*To be added during implementation*