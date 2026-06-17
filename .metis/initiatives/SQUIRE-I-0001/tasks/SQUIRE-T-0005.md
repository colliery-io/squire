---
id: domain-core-redemption-ledger
level: task
title: "Domain Core: redemption & ledger (balance, can_redeem, redeem, adjust)"
short_code: "SQUIRE-T-0005"
created_at: 2026-06-17T03:01:58.823486+00:00
updated_at: 2026-06-17T03:01:58.823486+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Domain Core: redemption & ledger (balance, can_redeem, redeem, adjust)

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0001]] (Domain Core)

## Objective

Implement the per-Squire `balance` projection, `can_redeem`, and the redemption + adjustment commands — with idempotency, affordability re-checked at commit, actor stamping, and the simplified `Once`/`Repeatable` availability.

## Acceptance Criteria

- [ ] `balance(snap, squire)` = derived per-Squire sum of `CompletionApproved(+points)`, `ItemRedeemed(−cost)`, `AchievementUnlocked(+bonus)`, `PointsAdjusted(±)`; pending claims/requests contribute 0; never stored.
- [ ] `RequestRedemption` emits `RedemptionRequested{squire}`; idempotent on `request_id`; reserves no points.
- [ ] `can_redeem(snap, squire, item, on)` passes only if item active + gating achievement unlocked **for that Squire** + that Squire's balance ≥ cost + (`Once` ⇒ no existing `ItemRedeemed` for the item, else `OutOfStock`); `Repeatable` never availability-blocked; `last_redeemed` (most-recent `ItemRedeemed`) surfaced on `RewardCard`. Failures → `Blocked::{InsufficientPoints|AchievementLocked|OutOfStock}`.
- [ ] `ReviewRedemption(Approve)` and direct `RedeemItem` RE-CHECK `can_redeem` at commit → `ItemRedeemed{squire, actor:Some(knight), request_id|command_id}`; a request that no longer clears fails `InsufficientPoints` at approval (AC-6).
- [ ] Direct `RedeemItem` / `AdjustPoints` are idempotent on `command_id` (a replay is a no-op, deduped from the log).
- [ ] `AdjustPoints` requires a non-empty reason; emits `PointsAdjusted{squire, actor, amount, reason}`; it is the ONLY path to a negative balance.
- [ ] `ReviewRedemption(Reject)` → `RedemptionRejected{actor, reason}`; second review → `AlreadyReviewed`; unknown → `RequestNotFound`.
- [ ] Unit tests for each branch.

## Implementation Notes

### Technical Approach
Pure derivations filtered by `squire`; `Once` out-of-stock and idempotency both derived from the log (no stored counters). `balance` folds the per-Squire event stream; `can_redeem` composes item-active, gate-unlocked (via `is_unlocked` from T-0006), affordability, and `Once`/`Repeatable` availability into a single `Result<(), Blocked>`. Redemption and adjustment commands re-run `can_redeem` at commit time before emitting events, and dedupe replays against the existing log by client id.

### Requirements covered
REQ-1.3.1, REQ-1.3.2, REQ-1.4.5, REQ-1.5.1, REQ-1.5.2, REQ-1.5.3, REQ-1.1.3/1.1.4 (request idempotency); PRD FR-P1..P3, FR-R2..R5, AC-6; ADR SQUIRE-A-0006 (availability), A-0001 (command_id), A-0005 (actor/subject).

### Dependencies
T-0001; gate-unlocked-for-squire uses `is_unlocked` from T-0006 (coordinate); balance feeds T-0006's `PointsEarned` criterion.

## Status Updates

*To be added during implementation*