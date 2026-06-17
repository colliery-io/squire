---
id: domain-core-redemption-ledger
level: task
title: "Domain Core: redemption & ledger (balance, can_redeem, redeem, adjust)"
short_code: "SQUIRE-T-0005"
created_at: 2026-06-17T03:01:58.823486+00:00
updated_at: 2026-06-17T03:45:24.474186+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Domain Core: redemption & ledger (balance, can_redeem, redeem, adjust)

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0001]] (Domain Core)

## Objective

Implement the per-Squire `balance` projection, `can_redeem`, and the redemption + adjustment commands — with idempotency, affordability re-checked at commit, actor stamping, and the simplified `Once`/`Repeatable` availability.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] `balance(snap, squire)` = derived per-Squire sum of `CompletionApproved(+)`, `ItemRedeemed(−)`, `AchievementUnlocked(+bonus)`, `PointsAdjusted(±)`; pending contribute 0; never stored.
- [x] `RequestRedemption` emits `RedemptionRequested{squire}`; idempotent on `request_id`; reserves nothing.
- [x] `can_redeem` passes only if item active + gate unlocked **for that Squire** + balance ≥ cost + (`Once` ⇒ not already redeemed, else `OutOfStock`); `Repeatable` never blocked; `reward_view` exposes affordable/lock/`last_redeemed` for the card. Failures → `Blocked::{InsufficientPoints|AchievementLocked|OutOfStock}`.
- [x] `ReviewRedemption(Approve)` + direct `RedeemItem` RE-CHECK `can_redeem` at commit → `ItemRedeemed{squire, actor:Some(knight), request_id|command_id}`; a request that no longer clears fails `InsufficientPoints` at approval (AC-6, tested).
- [x] Direct `RedeemItem` / `AdjustPoints` idempotent on `command_id` (replay = no-op, deduped from the log).
- [x] `AdjustPoints` requires a non-empty reason (empty → `InvalidDefinition`); emits `PointsAdjusted{squire, actor, amount, reason}`; only path to a negative balance.
- [x] `ReviewRedemption(Reject)` → `RedemptionRejected{actor, reason}`; second review → `AlreadyReviewed`; unknown → `RequestNotFound`.
- [x] `tests/redemption.rs` (10 tests) covering every branch.

## Implementation Notes

### Technical Approach
Pure derivations filtered by `squire`; `Once` out-of-stock and idempotency both derived from the log (no stored counters). `balance` folds the per-Squire event stream; `can_redeem` composes item-active, gate-unlocked (via `is_unlocked` from T-0006), affordability, and `Once`/`Repeatable` availability into a single `Result<(), Blocked>`. Redemption and adjustment commands re-run `can_redeem` at commit time before emitting events, and dedupe replays against the existing log by client id.

### Requirements covered
REQ-1.3.1, REQ-1.3.2, REQ-1.4.5, REQ-1.5.1, REQ-1.5.2, REQ-1.5.3, REQ-1.1.3/1.1.4 (request idempotency); PRD FR-P1..P3, FR-R2..R5, AC-6; ADR SQUIRE-A-0006 (availability), A-0001 (command_id), A-0005 (actor/subject).

### Dependencies
T-0001; gate-unlocked-for-squire uses `is_unlocked` from T-0006 (coordinate); balance feeds T-0006's `PointsEarned` criterion.

## Status Updates

**2026-06-16 — Completed.** Implemented `Projections::balance` (per-Squire signed fold over the log), `is_unlocked` (sticky per-Squire lookup — emission is T-0006), and `can_redeem` (active + gate-unlocked + balance + `Once`-out-of-stock → `Blocked`), all in `src/projections.rs`; plus `reward_view` (affordable / lock / `last_redeemed`) for the StateView card (re-exported). `src/redemption.rs` handles `RequestRedemption` (idempotent on `request_id`, no reservation), `ReviewRedemption` (approve→`commit_redeem`, reject→`RedemptionRejected`, `RequestNotFound`/`AlreadyReviewed`), direct `RedeemItem` (idempotent on `command_id`), and `AdjustPoints` (idempotent on `command_id`, non-empty reason). Shared `commit_redeem` re-checks `can_redeem` at commit (AC-6) and snapshots cost; idempotency + Once-stock + last_redeemed all derived from the log via new `src/common.rs` helpers (`request_meta`/`request_resolved`/`command_already_applied`/`item_ever_redeemed`/`last_redeemed`).

**Note:** empty-reason `AdjustPoints` reuses `DomainError::InvalidDefinition` (generic "malformed command input") rather than adding another variant.

Tests `tests/redemption.rs` (10): balance per-Squire/signed, request/approve(actor,cost)/reject, idempotent request & command_id, can_redeem insufficient/gate/Once, AC-6 affordability-at-commit, negative-only-via-adjust. Full suite **49 passed**. Committed.