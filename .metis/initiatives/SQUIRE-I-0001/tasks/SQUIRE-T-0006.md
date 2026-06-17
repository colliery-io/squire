---
id: domain-core-streaks-achievements
level: task
title: "Domain Core: streaks & achievements (current_streak, is_unlocked, unlock emission)"
short_code: "SQUIRE-T-0006"
created_at: 2026-06-17T03:02:02.897657+00:00
updated_at: 2026-06-17T03:54:21.078437+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Domain Core: streaks & achievements (current_streak, is_unlocked, unlock emission)

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0001]] (Domain Core)

## Objective

Implement `current_streak`, `is_unlocked`, achievement criterion evaluation with sticky unlock emission on approval (bonus + gate), all per-Squire; plus `StreakView` field derivation for the view.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] `current_streak`: `ScheduledOccurrences`+Quest counts consecutive completed scheduled occurrences (non-scheduled gaps don't break; pending-today forgiven) — M/W/F × 3 reads 3 despite weekends (AC-4); else `CalendarDays` counts consecutive days with ≥1 in-scope completion (repeatable = once/day); per-Squire.
- [x] A qualifying approval emits `AchievementUnlocked{squire}` **exactly once** when a `Streak`/`TotalCompletions`/`PointsEarned` criterion is first met (fixpoint over cascading bonuses); sticky; awards `bonus_points` to that Squire; flips a gated item's `can_redeem` from `AchievementLocked`→Ok.
- [x] `is_unlocked(snap, squire, id)` true iff that Squire has the unlock event (per-Squire).
- [x] `PointsEarned` evaluates the Squire's lifetime earned total (approvals + bonuses), not net balance.
- [x] `streak_view` derives `current`/`best`/`alive`/`next_milestone`.
- [x] `tests/achievements.rs` (15 tests) per criterion type + both streak bases.

## Implementation Notes

### Technical Approach
Walk the per-Squire event log by scope/date; evaluate criteria as part of the approval flow (hooks into T-0003's approve/auto-approve path) using the claim's `on` date. `current_streak` walks scheduled occurrences or calendar days backward from `asof`, collapsing repeatable completions to one/day. Unlock emission checks each achievement's `Criterion` after a qualifying approval and emits `AchievementUnlocked` only when first satisfied and not already present (sticky). `StreakView` derives display fields from the same walk.

### Requirements covered
REQ-1.4.2, REQ-1.4.3, REQ-1.4.4, REQ-1.2.6; PRD FR-S1..S6, AC-4.

### Dependencies
T-0001; consumes approvals from T-0003 (unlock emission runs on approval) and balance from T-0005 (PointsEarned).

## Status Updates

**2026-06-16 — Completed.** Implemented `current_streak` (`scheduled_streak` walks the quest's scheduled days back from `asof`, weekends skipped not broken, pending-today forgiven; `calendar_streak` walks calendar days with ≥1 in-scope completion) + `streak_view` (current/best/alive/next_milestone, `scopes_eq` since `Scope` has no `PartialEq`) in `src/projections.rs`. New `src/achievements.rs`: `unlocks_after` runs a fixpoint over a projected post-approval `Snapshot`, emitting `AchievementUnlocked` once per newly-met, not-yet-unlocked active achievement (cascades on bonus → `PointsEarned`); `criterion_met` dispatches Streak/TotalCompletions/PointsEarned. Added `src/common.rs` helpers `quest_in_scope`, `squire_completed_in_scope_on`, `total_completions` (distinct `(quest,day)`), `points_earned` (lifetime), `latest_scheduled_on_or_before`. Wired into `claims::approval_events` (now takes the claim's `on` + a `pending` slice so the auto-approve path threads its just-minted `CompletionClaimed` for the unlock join). `is_unlocked` (from T-0005) is the sticky lookup; gated `can_redeem` flips on unlock.

Tests `tests/achievements.rs` (15): AC-4 M/W/F streak, missed-day reset, pending-today, calendar streak + repeatable-once/day, per-Squire isolation, once-only sticky emission via the real approve flow, bonus→balance, PointsEarned earned-total, cascade, gated-item flip, auto-approve unlock, streak_view fields. Full suite **64 passed, 0 warnings**. Committed.