---
id: domain-core-invariant-property
level: task
title: "Domain Core: invariant property-test suite & projection perf check"
short_code: "SQUIRE-T-0007"
created_at: 2026-06-17T03:02:04.123324+00:00
updated_at: 2026-06-17T03:02:04.123324+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Domain Core: invariant property-test suite & projection perf check

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0001]] (Domain Core)

## Objective

Consolidate the engine-wide property tests (NFR-9) and a projection performance check (NFR-8) exercising the whole core against the in-memory Repository.

## Acceptance Criteria

- [ ] Property tests pass: an approved claim is counted exactly once for its Squire; a Squire's balance is never negative except via explicit `PointsAdjusted`; re-submitting the same client id (`claim_id`/`request_id`/`command_id`) is a no-op.
- [ ] Per-Squire isolation: a generated random history confirms one Squire's actions never move another Squire's balance/streaks/unlocks.
- [ ] `Race`: under many concurrent assignee claims an occurrence yields **exactly one** approved completion / one payout; a **rejected** first claim reopens it (a later claim can win).
- [ ] `Once` item: redeemable exactly once household-wide (first `ItemRedeemed` ⇒ every later `can_redeem` returns `OutOfStock`, derived); `Repeatable` never availability-blocked.
- [ ] A perf test asserts balance/streaks/due/state-view derivations stay well under 100 ms at household scale (a few Squires, dozens of quests, thousands of events).

## Implementation Notes

### Technical Approach
proptest generators over command sequences; replay-equivalence checks; a scale fixture for the perf assertion (`cargo test`/bench). Generators produce randomized but valid command histories across multiple Squires; invariants are asserted by replaying the log through the engine and projections. The perf check builds a household-scale fixture (a few Squires, dozens of quests, thousands of events) and times the core derivations.

### Requirements covered
NFR-1.1.3, NFR-1.1.5, NFR-1.1.2; PRD NFR-8/NFR-9.

### Dependencies
T-0002..T-0006 (exercises the full engine); builds on T-0001's harness.

## Status Updates

*To be added during implementation*