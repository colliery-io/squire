---
id: domain-core-streaks-achievements
level: task
title: "Domain Core: streaks & achievements (current_streak, is_unlocked, unlock emission)"
short_code: "SQUIRE-T-0006"
created_at: 2026-06-17T03:02:02.897657+00:00
updated_at: 2026-06-17T03:02:02.897657+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Domain Core: streaks & achievements (current_streak, is_unlocked, unlock emission)

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0001]] (Domain Core)

## Objective

Implement `current_streak`, `is_unlocked`, achievement criterion evaluation with sticky unlock emission on approval (bonus + gate), all per-Squire; plus `StreakView` field derivation for the view.

## Acceptance Criteria

- [ ] `current_streak(snap, squire, scope, basis, asof)`: `ScheduledOccurrences` counts consecutive completed scheduled occurrences for a quest scope (a non-scheduled-day gap does NOT break it) — a M/W/F quest done 3 scheduled days reads 3 despite weekends (AC-4); `CalendarDays` counts consecutive calendar days with ≥1 in-scope completion; a repeatable quest counts once/day; per-Squire.
- [ ] On a qualifying approval (evaluated within `handle`), the engine emits `AchievementUnlocked{squire}` **exactly once** for that Squire when a `Criterion` (`Streak`/`TotalCompletions`/`PointsEarned`) is first satisfied; the unlock is sticky (survives a later streak break); awards `bonus_points` to that Squire and unlocks any item gated on it for that Squire.
- [ ] `is_unlocked(snap, squire, id)` true iff that Squire has the `AchievementUnlocked` event.
- [ ] `PointsEarned` criterion evaluates that Squire's earned total.
- [ ] `StreakView` `current`/`best`/`alive`/`next_milestone` derived correctly (alive=false once an occurrence has lapsed; next_milestone = next achievement length in scope).
- [ ] Unit tests per criterion type and streak basis.

## Implementation Notes

### Technical Approach
Walk the per-Squire event log by scope/date; evaluate criteria as part of the approval flow (hooks into T-0003's approve/auto-approve path) using the claim's `on` date. `current_streak` walks scheduled occurrences or calendar days backward from `asof`, collapsing repeatable completions to one/day. Unlock emission checks each achievement's `Criterion` after a qualifying approval and emits `AchievementUnlocked` only when first satisfied and not already present (sticky). `StreakView` derives display fields from the same walk.

### Requirements covered
REQ-1.4.2, REQ-1.4.3, REQ-1.4.4, REQ-1.2.6; PRD FR-S1..S6, AC-4.

### Dependencies
T-0001; consumes approvals from T-0003 (unlock emission runs on approval) and balance from T-0005 (PointsEarned).

## Status Updates

*To be added during implementation*