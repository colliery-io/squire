---
id: ui-polish-1-playful-quest-theme
level: task
title: "UI polish 1: 'playful quest' theme foundation + child (Squire) home restyle"
short_code: "SQUIRE-T-0056"
created_at: 2026-06-18T03:10:00+00:00
updated_at: 2026-06-18T03:18:39.602965+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# UI polish 1: 'playful quest' theme foundation + child (Squire) home restyle

## Parent Initiative

[[SQUIRE-I-0001]] · App is **usable on real phones** ([[SQUIRE-T-0050]]); user asked for "serious UI polish on everything" + chose the **playful medieval-quest** direction. Foundation for [[SQUIRE-T-0057]]/[[SQUIRE-T-0058]].

## Objective

Establish the app's **"playful quest" design system** and apply it to the **hero screen** (the child's player home) so we can lock the look before propagating. Direction: lean into the Squire/Knight theme — chores are quests, points are gold, streaks are badges; warm parchment + royal + gold palette, display-serif headers, rounded cards. Build theme + shared components, restyle the Squire home, screenshot, get a thumbs-up before the parent UI / Keep / icon.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] `SquireTheme` (`ui/theme/Theme.kt`): custom `ColorScheme` (parchment bg, royal-indigo primary, **gold** secondary, herald-green tertiary for "done", crimson error, slate text), serif display `Typography`, rounded `Shapes`. Applied at `MainActivity.setContent` (replaced `MaterialTheme`) so both role UIs inherit it.
- [x] Reusable themed components (`ui/components/SquireUi.kt`): `GoldPill` (`★ N`), `StatusChip`, `SectionTitle` (royal serif), `ProgressDots` (●●●○), `Banner` (offline/update).
- [x] `PlayerHomeScreen` restyled: royal top bar with `⚔ name` + gold balance pill, parchment quest/reward/streak **cards**, gold **"Do it!"** CTAs, `+N★` rewards, pending/done(✓)/locked(🔒)/out-of-stock/"Need more ★" chips, streak badges with progress dots, friendly empty states, recent claims/requests. Reused by the "Acting as <name>" assume view.
- [x] `:app:assembleDebug` builds; `:core` unaffected; **screenshot captured + sent for sign-off**. Pure presentation (no callback/behaviour change).

## Implementation Notes

### Technical Approach
New `app/.../ui/theme/` (`Color`, `Type`, `Shape`, `Theme` → `SquireTheme`) + a `components/` file. `FontFamily.Serif` for display headers (zero new asset). Keep `PlayerHomeScreen` stateless — swap visuals only, not callbacks. Wrap `MainActivity.setContent { SquireTheme { … } }`. Material3 `Card`/`FilledTonalButton`/`AssistChip` with themed colors/shapes.

### Dependencies
[[SQUIRE-T-0034]] (PlayerHomeScreen), [[SQUIRE-T-0054]] (merged app / one theme entry).

### Risk Considerations
Pure presentation. Keep legibility (gold for accents/chips, not body text, on parchment). Emoji icons are a pragmatic v1. Get sign-off on the hero before the other surfaces (subjective).

## Status Updates

**2026-06-18 — Done (hero screen; awaiting look sign-off before propagating).** Built `SquireTheme` (parchment/royal/gold palette, serif headers, rounded shapes) + shared components (`GoldPill`, `StatusChip`, `SectionTitle`, `ProgressDots`, `Banner`); wrapped the app in it. Fully restyled the child `PlayerHomeScreen` to the "playful quest" look — royal header + gold balance, parchment cards, "Do it!" gold CTAs, ★ rewards, lock/done/pending chips, streak badges. Verified live on the emulator (paired as Squire against the demo) and **sent the screenshot for sign-off**. Pure presentation — no transport/state change, `:core` untouched. Next: propagate to the parent UI + pairing ([[SQUIRE-T-0057]]) and the icon + Keep ([[SQUIRE-T-0058]]), then package one OTA update (user will pull it via the in-app banner — no contract change, so the running server stays compatible).