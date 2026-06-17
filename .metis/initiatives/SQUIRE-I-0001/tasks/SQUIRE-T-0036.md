---
id: keep-authoring-ui-expose-streak
level: task
title: "Keep authoring UI: expose streak/total achievement criteria + reward gating (no model change)"
short_code: "SQUIRE-T-0036"
created_at: 2026-06-17T15:04:58.657210+00:00
updated_at: 2026-06-17T15:17:18.290243+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Keep authoring UI: expose streak/total achievement criteria + reward gating (no model change)

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0004]] (the Keep) · prompted by operator feedback on achievements

## Objective

Surface the achievement model's existing power in the Keep authoring UI: an achievement can be a **Streak** (e.g. 7 consecutive scheduled days of a quest) or **TotalCompletions**, with a **scope** (quest / category / any), awarding **bonus points** on unlock, and a reward can be **gated** on an achievement. The minimal T-0027 form only exposed `PointsEarned` + no gate, so this pattern looked impossible. **No domain/contract change** — the engine + API already support all of it (`Criterion`, `Scope`, `StreakBasis`, `RedeemableItem.gate`).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] **Keep achievement form**: criterion picker (Streak: scope+length+basis / TotalCompletions: scope+count / PointsEarned: total) + bonus points; scope sub-control Any / Quest (dropdown from `GET /api/quests`) / Category (text); basis select. Posts the correct externally-tagged `Criterion` JSON to `POST /api/achievements` (no API change).
- [x] **Keep item form**: a "requires achievement (gate)" picker (None / achievements dropdown) → `gate` on `POST /api/items`.
- [x] List views summarize the criterion (e.g. `Streak · Quest:<title> · 7 (ScheduledOccurrences) · +50`) and show items' gate (`needs: <achievement>`).
- [x] **Verified end-to-end on the emulator:** `serve_demo` seeds a Streak achievement ("Room Master", 3× Tidy-your-room → +50) + a gated reward ("Movie night"); the Squire app renders **"Movie night — Locked: Room Master"** + the **Room Master streak** — exactly the streak-gates-a-reward pattern.
- [x] `cargo test -p keep` green & warning-free; no new wire endpoints (pure embedded-assets).

## Implementation Notes

### Technical Approach
Pure embedded-assets work (`crates/keep/assets/{index.html,keep.js}`) — the API already accepts the full shapes. Externally-tagged serde JSON to build:
- Criterion: `{"PointsEarned":{"total":100}}` · `{"TotalCompletions":{"scope":<scope>,"count":5}}` · `{"Streak":{"scope":<scope>,"length":7,"basis":"ScheduledOccurrences"}}`
- Scope: `"Any"` · `{"Quest":<questId number>}` · `{"Category":"Chores"}` · StreakBasis: `"ScheduledOccurrences"`|`"CalendarDays"`
- Achievement body: `{id, name, description:null, criterion:<above>, bonus_points:50, active:true}`; Item `gate`: an achievement id number or null.
Populate the quest/achievement dropdowns from `GET /api/quests` / `GET /api/achievements`.

### Dependencies
[[SQUIRE-T-0027]] (the forms being enriched). Domain `Criterion`/`Scope`/`StreakBasis`/`gate` already shipped (T-0002/T-0005/T-0006).

### Risk Considerations / follow-up
**Surfacing EARNED achievements (badges) on the Squire app** needs a small additive `StateView` field (an `achievements` list) — filed/handled separately; streaks already ride in `StateView.streaks`. This task is Keep authoring only.

## Status Updates

**2026-06-17 — Done (`23c4dc5`; demo seed `ec247d1`).** The Keep authoring UI now exposes the full achievement model (no domain change — the gap was purely UI from T-0027).

- **`crates/keep/assets/{index.html,keep.js}`**: achievement form with a criterion picker (Streak / TotalCompletions / PointsEarned) + scope sub-control (Any / Quest dropdown / Category) + basis; item form with a gate picker; list views summarize the criterion + gate. Builds the externally-tagged `Criterion`/`Scope` JSON the API already accepts.
- **`serve_demo`**: seeds a Streak achievement ("Room Master" = 3 scheduled days of *Tidy your room* → +50) + a reward ("Movie night") gated on it.
- **Eyeballed live:** the Squire app shows *Movie night — Locked: Room Master* and the *Room Master* streak (current/best/next-3). This is the "streak that imparts its own gate + currency (bonus points)" the operator described.

**Verified:** `cargo test -p keep` green & warning-free.

**Follow-up (filed):** surface EARNED achievements (badges) on the Squire app — needs a small additive `StateView.achievements` field. Streaks already ride in `StateView.streaks` (shown).