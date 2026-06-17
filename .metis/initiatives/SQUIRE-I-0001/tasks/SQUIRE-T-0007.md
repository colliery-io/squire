---
id: domain-core-invariant-property
level: task
title: "Domain Core: invariant property-test suite & projection perf check"
short_code: "SQUIRE-T-0007"
created_at: 2026-06-17T03:02:04.123324+00:00
updated_at: 2026-06-17T04:04:50.706857+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Domain Core: invariant property-test suite & projection perf check

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0001]] (Domain Core)

## Objective

Consolidate the engine-wide property tests (NFR-9) and a projection performance check (NFR-8) exercising the whole core against the in-memory Repository.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] proptest replay asserts: no duplicate facts per client id (idempotency for `claim_id`/`request_id`/`command_id`); approved⇒has-claim; balance ≥ 0 absent a negative `PointsAdjusted`.
- [x] Per-Squire isolation: random history confirms a Squire's balance + q1 scheduled streak equal the values from a snapshot containing only that Squire's events.
- [x] `Race`: explicit test — 3 squires submit one occurrence, only one `CompletionApproved` (others `OccurrenceTaken`); reject-reopens lets a later squire win; proptest invariant ≤1 payout per occurrence.
- [x] `Once` redeemable exactly once household-wide (second → `Redeem(OutOfStock)`); replay of same `claim_id`/`request_id`/`command_id` is a no-op.
- [x] `tests/perf.rs`: full projection sweep over ~4800 events runs in **~13 ms release** (gate `<100ms` in release; `<1s` debug guard catches regressions). Hot path optimized O(n²)→O(n) via a one-pass `ClaimIndex` behind unchanged public signatures.

## Implementation Notes

### Technical Approach
proptest generators over command sequences; replay-equivalence checks; a scale fixture for the perf assertion (`cargo test`/bench). Generators produce randomized but valid command histories across multiple Squires; invariants are asserted by replaying the log through the engine and projections. The perf check builds a household-scale fixture (a few Squires, dozens of quests, thousands of events) and times the core derivations.

### Requirements covered
NFR-1.1.3, NFR-1.1.5, NFR-1.1.2; PRD NFR-8/NFR-9.

### Dependencies
T-0002..T-0006 (exercises the full engine); builds on T-0001's harness.

## Status Updates

**2026-06-17 — Completed.** `tests/invariants.rs`: a proptest stateful-replay harness generates `Action` sequences (small id spaces → frequent collisions, exercising idempotency) over the q1–q4/i1/i2 + Streak-3 fixture, replays through `DomainEngine.handle`, and asserts six invariants (no-dup-facts per client id; approved⇒claim; Race ≤1 payout per occurrence; Once ≤1 redemption; balance ≥0 absent a negative adjust; per-Squire isolation of balance + scheduled streak). Plus 4 explicit tests: Race single-winner/`OccurrenceTaken`, reject-reopens, household-wide Once, replay-no-op. `tests/perf.rs`: ~4800-event household history, full projection sweep (balance/quests_due/streaks/quest_status/reward_view).

**Perf optimization:** the sweep was O(n²) (per-event `claim_meta` rescans). Added an internal one-pass `ClaimIndex` (`HashMap<ClaimId,(squire,quest,on)>` + resolution map) in `common.rs` with `_with` index-taking helper variants; `quests_due`/`current_streak`/`quest_status`/`streak_view` build it once per call. Public signatures + behavior unchanged (all prior tests pass). Result: ~1.95s → **~13 ms release** (<100 ms NFR-1.1.2), ~260 ms debug. Gate is strict `<100ms` under `--release`, generous `<1s` in debug.

**70 passed, 0 warnings** (debug + release). Committed `3dac686`. This completes all Domain Core tasks (T-0001..T-0007).