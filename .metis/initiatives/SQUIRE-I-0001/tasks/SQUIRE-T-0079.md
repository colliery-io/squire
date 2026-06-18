---
id: refine-the-squire-child-home
level: task
title: "Refine the Squire (child) home: surface earned achievement badges"
short_code: "SQUIRE-T-0079"
created_at: 2026-06-18T22:57:30.107861+00:00
updated_at: 2026-06-18T23:03:28.213147+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Refine the Squire (child) home: surface earned achievement badges

## Parent Initiative

[[SQUIRE-I-0001]]

## Objective

Close a real child-experience gap: a Squire **earns achievements** (and the bonus points) but **never sees the badge**. `StateView` carries `streaks` (progress) but nothing for *unlocked* achievements — the "Badges & Streaks" section only renders streaks. Make the payoff visible: show earned badges on the child home.

## Define-First: the flows we expect

### Flow C1 — Earned badges are visible
- When a Squire has unlocked achievement(s) (an `AchievementUnlocked` event exists for them), the home shows a **Badges** section: a trophy, the achievement name, and the bonus it awarded ("+N ★").
- Each badge appears once (dedup by achievement id), most-recent first.
- A Squire with no unlocks sees no Badges section (or an encouraging empty hint) — streaks still show their in-progress goals.

### Flow C2 — Badges vs streaks are distinct
- **Streaks** = in-progress momentum (current / next milestone). **Badges** = milestones already earned. They are separate sections so a child reads "what I've won" apart from "what I'm working toward".

## Current State

- Domain already emits `Event::AchievementUnlocked { squire, id, bonus, at }` and `Proj::is_unlocked`. No new domain behavior needed — just a read projection + a new view field.
- `StateView { squire, generated_at, balance, quests_today, streaks, rewards, my_claims, my_requests }` — **no badges field**.
- `PlayerHomeScreen` renders `streaks` under "Badges & Streaks" but no earned achievements.

## Acceptance Criteria

- [x] Contract: added `BadgeView { id, name, bonus, at }` and `badges: Vec<BadgeView>` (`#[serde(default)]`) to `StateView`; schema registered; `openapi.json` regenerated + conformance green; SDK regenerated.
- [x] API (`squire.rs`): `assemble_state` populates `badges` from `AchievementUnlocked` events — dedup by achievement id (walking newest→oldest), name resolved from the snapshot.
- [x] API test (`tests/squire.rs`, +1): a fresh Squire has empty `badges`; after seeding a `PutAchievement` + `AchievementUnlocked`, `badges` shows it (id "Century Club" +25). Refactored `test_state` → `test_state_with(extra)`. 7 pass.
- [x] `PlayerHomeScreen`: a "🏅 Badges" section (distinct from the now-separate "Streaks") renders each earned badge (trophy + name + "+N ★"). Updated the preview + both screenshot `StateView` seeds; handled the nullable generated field via `orEmpty()`.
- [x] Paparazzi `squirePlayerHomeHistory` extended + re-recorded — image-validated (Badges: Century Club +25★, Chore Champion +50★, above Streaks).
- [x] `cargo test -p api`, conformance, and `:app:verifyPaparazziDebug` green.

## Implementation Notes

- `badges(snap, squire)`: iterate `snap.events` for `AchievementUnlocked { squire, id, bonus, at }`, keep the latest per `id`, join `name` from `snap.achievements` (fallback to `#id`), sort by `at` desc.
- Keep it a pure projection (no clock needed).

## Dependencies
- Builds on the achievement engine (T-0006) and authoring (T-0071/0072).

## Status Updates

**2026-06-18 — Done.**
- Contract `api.rs`: `BadgeView` + `StateView.badges` (`#[serde(default)]`). `pub use api::*` auto-exports it; only `assemble_state` constructs `StateView`.
- api `squire.rs`: `badges(snap, squire)` pure projection over `AchievementUnlocked` events (rev-walk + dedup on `id.0`, name from `snap.achievements`). Wired into `assemble_state`. openapi.rs schema registered; `openapi.json` regenerated; conformance + SDK regenerated.
- api test: `earned_achievements_surface_as_badges` (+ `test_state_with` refactor). 7 pass.
- Android `PlayerHomeScreen`: new "🏅 Badges" section + `BadgeCardRow`; renamed the streaks section "Badges & Streaks" → "Streaks" (now genuinely distinct). Generated `badges` is nullable (serde default) → `orEmpty()`. Updated preview + 2 snapshot seeds.
- Image-validated: the child now sees earned badges (Century Club +25★, Chore Champion +50★) above their in-progress streaks. `verifyPaparazziDebug` green.
- Closes the real gap: a child earned achievements (and the bonus) but never saw the badge.