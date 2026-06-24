---
id: squire-goals-card-show-progress
level: task
title: "Squire goals card: show progress and fix the 'how to earn it' line break"
short_code: "SQUIRE-T-0122"
created_at: 2026-06-24T01:17:28.925108+00:00
updated_at: 2026-06-24T01:40:30.002523+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Squire goals card: show progress and fix the 'how to earn it' line break

## Objective

Polish the child home **goals** card (a goal = an achievement not yet earned): (1) show a
kid-friendly progress indicator — "2 / 5 chores", "60 / 100 coins" — matching streak goals; and
(2) in the goal detail, put the "how to earn it" description on its own line instead of crammed
beside its label.

> Collapsed ticket — both defects live in the same goals-card code in
> `clients/squire-android/app/src/main/kotlin/com/squire/app/ui/PlayerHomeScreen.kt` (and `goalDetail`
> in particular), so they land together.

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P2 - Medium (nice to have)

### Impact Assessment
- **Affected Users**: Every child with a non-streak achievement in flight.
- **Expected vs Actual**:
  - (1) Expected a progress bar / "x / y" toward the goal; actual shows only name + static
    description + bonus — no sense of how close they are.
  - (2) Expected the "how to earn it" label above its description; actual renders label (left) and a
    sentence-long description (right) on one `SpaceBetween` row with no separation.

## Verified behavior (root cause)

**(1) No progress data on goals (streaks have it — asymmetry).**
- `GoalView` carries only `{ id, name, description, bonus }`
  (`crates/domain-core/src/contract/api.rs:134-141`); `goals()` populates just those
  (`crates/api/src/squire.rs:304-322`); SDK matches (`GoalView.kt:33-47`).
- `StreakView` carries `current/best/next_milestone` (`api.rs:107-113`) and `StreakCardRow` renders
  `ProgressDots` + "3 / 7" (`PlayerHomeScreen.kt:513-533`); `GoalCardRow` renders name + description
  + bonus only (`:491-510`).
- The numbers are **already computed** in the domain for the unlock gate via `pub` helpers —
  `common::total_completions(...)` (`common.rs:419`) and `common::points_earned(...)` (`common.rs:438`),
  used by `criterion_met` (`achievements.rs:63-66`). "current" = same helper; "target" = the
  `count`/`total` literal on the `Criterion`. The only missing input in `goals()` is `today`, already
  in scope in `assemble_state` (`squire.rs:245`). → **plumb-through, no new projection logic.**

**(2) "How to earn it" shares a row with its value.**
- `goalDetail` (`PlayerHomeScreen.kt:755-763`) adds `"How to earn it" to g.description` into the
  generic label↔value `lines` mechanism, which `DetailCard`/`DetailDialog` render as a
  `SpaceBetween` `Row` per pair (`:618-626`, `:655-663`) — built for short stat pairs, not a
  sentence.

## Acceptance Criteria

## Acceptance Criteria

- [x] Non-streak goals (TotalCompletions, PointsEarned) show current/target progress on the goals
      card and in the detail; numbers match the domain unlock computation. Streaks unchanged.
- [x] The goal's "how to earn it" text renders as a stacked, full-width block on its own line,
      matching quest/reward detail cards.

## Implementation Notes

### Technical Approach
**Progress (1):**
1. Add `current: u32`, `target: u32` to `GoalView` (`api.rs:134`), `#[serde(default)]` for back-compat.
2. `crates/api/src/squire.rs`: thread `today` into `goals` (call site `:253`); per criterion compute
   `TotalCompletions{scope,count}` → `current = common::total_completions(snap,squire,scope,today)`,
   `target = *count`; `PointsEarned{total}` → `current = common::points_earned(snap,squire)`,
   `target = *total`. Streaks already filtered out (`:313`).
3. Regen `crates/api/openapi.json` (`GoalView` already registered, `openapi.rs:103`) + Kotlin SDK.
4. `GoalCardRow` (`PlayerHomeScreen.kt:491-510`): add a progress row mirroring `StreakCardRow` —
   reuse `ProgressDots` (`SquireUi.kt:175-182`) + "current / target" `Text`.

**Line break (2):** in `goalDetail` (`PlayerHomeScreen.kt:755-763`) move the description out of
`lines` into the `description` slot (rendered stacked + full-width, like `questDetail`/`rewardDetail`
at `:673`/`:692`):
```kotlin
internal fun goalDetail(g: GoalView) = DetailContent(
    icon = "🎯", title = g.name,
    description = g.description,                       // own line, stacked
    lines = buildList {
        add("Progress" to "${g.current} / ${g.target}")   // from fix (1)
        if (g.bonus > 0) add("Reward" to "+${g.bonus} coins")
    },
    note = "Not unlocked yet — keep going!",
)
```

### Test Cases
- Rust (`crates/api/tests/squire.rs`): `goals()` returns `current/target` for a `TotalCompletions`
  achievement with completions logged, and for `PointsEarned`.
- Android: a goal-with-progress + goal-detail snapshot in `ScreenshotTests.kt` (locks both the
  progress row and the stacked description).

## Status Updates

### 2026-06-24 — implemented & verified

**Progress (1):** `GoalView` gained `current`/`target` (`#[serde(default)]`); `goals()` threads
`today` and computes them via the existing `total_completions`/`points_earned` helpers (re-exported
from `domain-core/lib.rs`), so "x / y" uses the same math as the unlock gate. Re-froze `openapi.json`;
regenerated the Android SDK; `GoalCardRow` renders `ProgressDots` + "current / target".

**Line break (2):** `goalDetail` moves the description into the stacked, full-width `description`
slot and adds a "Progress" line — matching `questDetail`/`rewardDetail`.

**Verification:** new Rust test `goals_show_progress_toward_threshold` (0/3→1/3 completions, 0/100→
5/100 points); full domain-core + api suites + openapi drift test green. Paparazzi: progress added to
goal fixtures + a focused `squireGoalDetail` golden; re-recorded and `verifyPaparazziDebug` green.
Visually confirmed on the "Me" tab (6/10 dots) and the goal detail ("Complete 10 chores together" on
its own line above "Progress 6 / 10"). Both acceptance criteria met.
</content>