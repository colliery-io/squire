---
id: store-repository-conformance-raw
level: task
title: "Store: Repository conformance, raw log query & perf"
short_code: "SQUIRE-T-0013"
created_at: 2026-06-17T04:08:45.773954+00:00
updated_at: 2026-06-17T05:05:14.302107+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Store: Repository conformance, raw log query & perf

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0002]] (Persistence & Store)

## Objective

Prove the SQLite-backed Repository is a faithful drop-in for the in-memory one (parity), add the raw per-quest / per-item event-log query, and add a perf check at scale.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] Conformance: a full scenario (authoring, claim→approve→balance, streak→`AchievementUnlocked`+bonus+gate-lift, adjust, redemption request→approve, Race) driven through `DomainEngine.handle`→`apply` against BOTH the Diesel `Store` and `InMemoryRepository` with the same `FixedClock`; after each step `store.snapshot()` debug-equals the in-memory snapshot AND all five projections agree. Runs on SQLite + the compose Postgres.
- [x] `Store::raw_log_for_quest(qid)` (claims for the quest + their approvals/rejections) and `raw_log_for_item(iid)` (requests + redemptions) return the touching events in `seq` order (NFR-11).
- [x] Perf (NFR-2.2/NFR-8): 3 squires, 20 quests, ~4,800 events → `snapshot()` + full projection sweep in **7.33 ms release** (strict `<100ms` under `--release`; `<1.5s` debug guard).
- [x] `cargo test -p store` green (27 SQLite tests, 0 warnings); perf gate passes in `--release`; Postgres run green too.

## Implementation Notes

### Technical Approach
Build a conformance harness that applies engine-produced `Change` batches to the real SQLite store and compares projection outputs against the same scenarios run on the in-memory `Repository`; port a few domain-core flows (claim→approve→balance, redeem, streak/unlock, Race) as the representative set. The raw-log query filters events by quest/item (joining via `claim` where the event does not carry the id directly) and returns them in append order (NFR-11). The perf test seeds a few thousand events, then times `snapshot()` plus a full projection sweep; gate strictly (well under 100 ms) in release and generously in debug.

ENV CAVEAT: conformance, raw-log, and perf all run on the SQLite path (the tested backend); the Postgres path remains compile-only / `DATABASE_URL`-gated.

### Requirements covered
REQ-1.6, NFR-2.2 (NFR-8), NFR-11; conformance across REQ-1.1 / 1.2.

### Dependencies
SQUIRE-T-0008 .. SQUIRE-T-0012 (full store stack: schema, mapping, apply/snapshot, provisioning, export).

## Status Updates

**2026-06-17 — Completed.** `tests/conformance.rs`: drives one representative end-to-end household scenario (seed 3 users; author daily + Race quests, a 2-day scheduled-streak achievement, a gated item; claim→approve→balance; a 2nd day that fires `AchievementUnlocked` +25 bonus + gate lift; `AdjustPoints`; redemption request→approve of the now-unlocked item; a Race Alice beats Boris) through the real `DomainEngine` against BOTH the Diesel `Store` and `InMemoryRepository` with one shared `FixedClock`; after every step asserts `store.snapshot()` debug-equals the in-memory snapshot and that all five projections agree for both squires — the faithful-Repository proof. `Store::raw_log_for_quest` (resolve the quest's claim ids, then events with that `quest_id` or `claim_id`, seq-ordered) + `raw_log_for_item` (events with `item_id`, seq-ordered) added for NFR-11, tested. `tests/perf.rs`: 3 squires × 20 quests × ~4,800 events, `snapshot()`+full sweep timed — **7.33 ms release** (strict `<100ms` `#[cfg(not(debug_assertions))]`, `<1.5s` debug).

Cleared the last warning (a SQLite-only-build irrefutable `let-else` in `tests/mapping.rs` → cfg-aware `match`). Results: SQLite `cargo test -p store` → **27 passed, 0 warnings**; `--release` perf gate passes; Postgres run (`--features postgres` + compose) → conformance + perf + all suites green on PG; `cargo test --workspace` → domain-core 70 intact. Committed. **This completes the Persistence & Store component (T-0008..T-0013).**