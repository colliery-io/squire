---
id: squire-goals-detail-line-break
level: task
title: "Squire goals detail: line break between 'How to earn it' and the description"
short_code: "SQUIRE-T-0123"
created_at: 2026-06-24T01:17:29.607283+00:00
updated_at: 2026-06-24T01:17:29.607283+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/backlog"
  - "#bug"


exit_criteria_met: false
initiative_id: NULL
---

# Squire goals detail: line break between 'How to earn it' and the description

## Objective

In the child goal detail, put the "How to earn it" description on its own line (stacked, full-width)
instead of crammed onto the same row as its label.

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P3 - Low (when time permits)

### Impact Assessment
- **Affected Users**: Every child tapping a goal for detail.
- **Expected vs Actual**: Expected the label "How to earn it" above its description text; actual
  renders label (left) and the full description (right) on one `SpaceBetween` row, so a sentence-long
  value runs against the label with no separation.

## Verified behavior (root cause)

- `goalDetail` (`clients/squire-android/app/src/main/kotlin/com/squire/app/ui/PlayerHomeScreen.kt:755-763`)
  puts the description into the generic label↔value `lines` mechanism:
  `add("How to earn it" to g.description)`.
- `DetailCard`/`DetailDialog` render each `lines` pair as one `Row` with
  `Arrangement.SpaceBetween` (`PlayerHomeScreen.kt:618-626`, `:655-663`) — designed for short stat
  pairs ("Cost" / "12 coins"), not a descriptive sentence. So label and description share a line.

## Acceptance Criteria

## Acceptance Criteria

- [ ] The goal's "how to earn it" text renders as a stacked block on its own line(s), visually
      separated from its label.
- [ ] Matches the layout already used by quest/reward detail cards.

## Implementation Notes

### Technical Approach
Pure Android one-file change in `PlayerHomeScreen.kt`. Move the goal description out of `lines` and
into the `description` slot, which `DetailCard`/`DetailDialog` already render stacked and full-width
with a spacer (`PlayerHomeScreen.kt:613-616`, `:651-654`) — the same pattern `questDetail`/
`rewardDetail` use (`:673`, `:692`):

```kotlin
internal fun goalDetail(g: GoalView) = DetailContent(
    icon = "🎯",
    title = g.name,
    description = g.description,        // stacked, full-width, own line
    lines = buildList {
        // (with SQUIRE-T-0122) add("Progress" to "${g.current} / ${g.target}")
        if (g.bonus > 0) add("Reward" to "+${g.bonus} coins")
    },
    note = "Not unlocked yet — keep going!",
)
```

### Dependencies
- Same function as [[SQUIRE-T-0122]] (`goalDetail`) — sequence the two so the progress line and the
  description move land together.

### Test Cases
- Add a goal-detail snapshot to `ScreenshotTests.kt` (`DetailCard` is built to be snapshot-able,
  `PlayerHomeScreen.kt:595-597`) to lock the stacked layout.

## Status Updates

*To be added during implementation*
</content>