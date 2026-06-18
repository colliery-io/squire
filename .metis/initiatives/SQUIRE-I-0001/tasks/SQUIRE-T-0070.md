---
id: android-screenshot-test-harness
level: task
title: "Android screenshot-test harness (Paparazzi) for the native screens"
short_code: "SQUIRE-T-0070"
created_at: 2026-06-18T16:25:49.728145+00:00
updated_at: 2026-06-18T16:25:49.728145+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Android screenshot-test harness (Paparazzi) for the native screens

## Parent Initiative

[[SQUIRE-I-0001]] · User wants a **tight iteration loop**: present a plan → I enact it → validate outcomes with images. The native side (Compose) can't be driven by Playwright ([[SQUIRE-T-0069]] covers the web Keep) and the emulator/adb loop was painfully flaky — this gives JVM-rendered screenshots with no device.

## Objective

Stand up **Paparazzi** screenshot tests for the app's Compose screens: render to PNGs in a plain `./gradlew test` (no emulator/adb/server), so any UI change is validated visually and regressions fail the build on a pixel diff.

## Acceptance Criteria

- [x] Paparazzi wired (`libs.versions.toml` + root `apply false` + `:app` plugin, v1.3.5 for AGP 8.5/Kotlin 2.0). `:app:recordPaparazziDebug` / `:app:verifyPaparazziDebug` work on the JVM.
- [x] `ScreenshotTests` renders the three key native screens to PNGs: **Knight review home**, **Squire player home**, **Manage Quests**. Stateless screens take sample state; `QuestAdminScreen` gets `initialQuests`/`libraryOverride` test seams (null → live in production) so it renders populated.
- [x] Goldens committed under `app/src/test/snapshots/images/`; `app/src/test/README.md` documents record/verify + the loop. Full app still builds.
- [x] **Caught a real bug:** `PlayerHomeScreen`'s `LazyColumn` keyed quests by `questId` and rewards by `itemId` in one list — a numeric `QuestId == ItemId` collides and crashes. Fixed by namespacing keys (`"q"`/`"i"`/`"s"`).

## Implementation Notes

### Technical Approach
`Paparazzi(deviceConfig = PIXEL_6, NOTNIGHT)`; `paparazzi.snapshot { SquireTheme { Screen(...) } }`. Samples reuse the screens' `@Preview` data. `QuestAdminScreen` made `internal` (exposes the now-`internal` `LibraryQuest`) and gained injectable data seams that also short-circuit its `LaunchedEffect` fetch when provided.

### Dependencies
The native screens ([[SQUIRE-T-0056]]/[[SQUIRE-T-0057]]/[[SQUIRE-T-0065]]). Pairs with [[SQUIRE-T-0069]] (Playwright/web).

### Risk Considerations
Paparazzi is snapshot-only (no interaction-driving — that'd be Roborazzi/instrumented). For "validate a state" it's ideal: render any state, including post-action, by passing it in. Keep goldens in sync (record after intended UI changes). Emoji/serif render via the bundled layoutlib fonts.

## Status Updates

**2026-06-18 — Done.** Paparazzi harness up; 3 screens snapshot to PNG on the JVM (`recordPaparazziDebug`, ~7s, no device). Refactored `QuestAdminScreen` with injectable `initialQuests`/`libraryOverride` seams. The harness immediately **found + fixed a real crash** (LazyColumn key collision in the player home). Goldens committed; README documents the loop. Now: present a UI plan → I implement → record → review the image.
