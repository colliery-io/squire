---
id: polish-the-squire-child-views
level: task
title: "Polish the Squire (child) views: color-coded activity, reward progress, warmer empty states"
short_code: "SQUIRE-T-0081"
created_at: 2026-06-18T23:19:59.358548+00:00
updated_at: 2026-06-18T23:22:48.797711+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Polish the Squire (child) views: color-coded activity, reward progress, warmer empty states

## Parent Initiative

[[SQUIRE-I-0001]]

## Objective

A cosmetic-tier pass on the child home (`PlayerHomeScreen`). The data is all correct (T-0079); this makes it *read* like a kid's app — outcomes scannable at a glance, rewards feel attainable, empty states encouraging. No new data/contract/SDK — UI only, using the existing theme palette.

## Define-First: the visual outcomes we want

### Flow P1 — Outcomes are color-coded, not flat gray
- The "Recent" section currently renders each claim/request as one line of **muted gray text** with an inline emoji. Replace with compact cards carrying a **semantic status chip**:
  - **Approved** → green (`tertiaryContainer`), "✓ +N ★" / "✓ Redeemed".
  - **Rejected** → red (`errorContainer`), "✗ Rejected"; the **reason shown as a subtitle** under the title (still legible, not crammed into the label).
  - **Pending** → amber (`secondaryContainer`), "⏳ Pending".
- Rename the section to "Recent activity".

### Flow P2 — Rewards feel attainable
- An unaffordable reward currently says a flat "Need more ★". Show **how close**: "N more ★" (where N = cost − balance), so the child sees a concrete goal.

### Flow P3 — Warmer empty states
- Friendlier copy + emoji for the no-quests / no-rewards / nothing-recent states (kid-appropriate encouragement).

## Current State

- `PlayerHomeScreen`: `RecentRow(title, label)` is a flat `Row` with a muted-gray label; `RewardCardRow` shows a static "Need more ★"; empty hints are plain.
- Theme already provides the semantic containers (`tertiaryContainer`/`errorContainer`/`secondaryContainer`) and `StatusChip`.

## Acceptance Criteria

- [x] Recent activity: each claim/request renders as a `RecentCard` with a semantic status chip — Approved green (`tertiaryContainer`), Rejected red (`errorContainer`) with the **reason as a subtitle**, Pending amber (`secondaryContainer`). Section titled "Recent activity".
- [x] Unaffordable reward shows "N more ★" (= `cost - balance`, clamped ≥ 1) — e.g. Movie night 15★ at balance 12 → "3 more ★".
- [x] Empty states warmer: rewards "ask a grown-up! 🛒", recent "go finish a quest! 💪".
- [x] Paparazzi: re-recorded `squirePlayerHomeHistory` + `squirePlayerHome` — image-validated.
- [x] `:app:assembleDebug` + `:app:verifyPaparazziDebug` green.

## Implementation Notes

- UI-only in `PlayerHomeScreen.kt`. Add a `RecentCard(title, subtitle?, chipText, container, content)` helper; map claim/request state → chip color + text. Pass `view.balance` into `RewardCardRow` for the "N more ★" hint. Reuse `StatusChip` + `QuestCard`.
- No backend/SDK/contract change.

## Dependencies
- Builds on T-0079 (badges) — same child home; the snapshot already seeds rejected claim + request with reasons.

## Status Updates

**2026-06-18 — Done.**
- `PlayerHomeScreen` (UI-only): replaced flat-gray `RecentRow` with a color-coded `RecentCard(title, subtitle?, chipText, container, content)` — claim/request state → green/red/amber `StatusChip`; rejection reason now a legible subtitle. Section renamed "Recent activity".
- Reward: `RewardCardRow` takes `balance`; unaffordable shows "N more ★" (concrete goal) instead of "Need more ★".
- Warmer empty states (rewards / recent).
- Image-validated both snapshots: Recent activity reads at a glance (✓ +10★ green / ✗ Rejected + reason red / ⏳ Pending amber), and Movie night shows "3 more ★". `verifyPaparazziDebug` green.
- No backend/SDK/contract change — pure cosmetic tier.