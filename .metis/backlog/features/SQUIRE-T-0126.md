---
id: edit-in-place-remainder-keep
level: task
title: "Edit-in-place, remainder: Keep achievements + Android (quests/rewards/achievements) + member rename"
short_code: "SQUIRE-T-0126"
created_at: 2026-06-24T02:08:37.035258+00:00
updated_at: 2026-06-24T02:58:21.789878+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Edit-in-place, remainder: Keep achievements + Android + member rename

## Objective

Finish the edit-in-place feature begun in [[SQUIRE-T-0120]] (which shipped **Keep-web edit for quests
and rewards**). Three pieces remain:

1. **Keep web — achievements edit.** Same pattern as quests/rewards (already built in `keep.js`:
   `makeEditable`, an `Edit` row button, `id: <edit>.id ?? Date.now()` on submit). The only extra work
   is `fillAchForm(a)` — round-tripping the criterion (`PointsEarned | TotalCompletions | Streak`) and
   its scope sub-fields (`Any | Quest | Category`) back into the form selects, then `syncAchFields()`.
   The Keep's `GET /api/achievements` already returns the raw achievement, so all fields are available.
2. **Android (Knight) — edit for quests/rewards/achievements.** The 3 `*AdminScreen.kt` submit with
   `id = null` today; add an Edit affordance on the "Current …" list cards that prefills the form and
   reuses the id. **Round-trip caveat (real here, unlike the Keep):** the phone API
   (`crates/api/src/authoring.rs`) returns *flattened summary* DTOs with pre-rendered labels
   (`cadence_label`, `assignment_label`, availability/gate summaries), not raw fields — so the GET
   summaries must be extended to carry the raw cadence/assignment/completion/gate (or echo the full
   object) for the form to round-trip. Then regen openapi/SDK.
3. **Member rename (the one true backend gap).** No command/endpoint updates an existing member's
   `display_name`/`role` today (only `add_member` + `set_member_active`). Add a member-update
   operation — natural home is the identity control-plane alongside `add_member` (members are
   identity-owned, not engine-commanded). `PUT/POST /admin/members/{id}` (RequireKnight), regen
   openapi/SDK, surface rename in `keep/src/members.rs` + `keep.js` + `MemberAdminScreen.kt`.

## Backlog Item Details

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P2 - Medium (nice to have)

### Business Justification
- **User Value**: Completes "edit, don't archive-and-recreate" across the achievement catalog, the
  phone, and member names — the remainder of the daily-driver quests/rewards already shipped.
- **Effort Estimate**: **S** (Keep achievements) + **M** (Android, needs DTO round-trip work) + **M**
  (member rename backend + 2 UIs).

## Acceptance Criteria

## Acceptance Criteria

- [x] Keep web: achievement list rows have an **Edit** action that round-trips the criterion/scope and
      upserts with the existing id. E2E covered.
- [x] Members: a parent can rename a member — endpoint on **both** surfaces (phone API
      `/admin/members/{id}/name` + Keep `/api/members/{id}/name`, reusing the `PutUser` upsert), Keep UI
      (Rename button), and **Android UI** (Rename dialog in `MemberAdminScreen`). Rust + E2E + snapshot
      covered. (Rename only; ± role and last-Knight guard not in scope — rename doesn't change role.)
- [x] Edited objects keep their id + `created_*` audit; only `updated_*` moves (store `on_conflict`
      upsert, unchanged).
- [x] Android Knight **catalog** edit (quests/rewards/achievements): the 3 flat summary DTOs extended
      with raw round-trip fields (openapi + SDK regenerated); an **Edit** affordance on each "Current …"
      card prefills the form and reuses the id (relabel + Cancel). Server round-trip Rust-tested;
      affordance snapshotted.

## Implementation Notes

- The Keep-web pattern is already in place from [[SQUIRE-T-0120]] — copy the quest/reward approach for
  achievements (`makeEditable`, Edit button in `loadCatalog`, `fillAchForm`).
- History-safety holds (rewards snapshotted at approval; see [[SQUIRE-T-0120]]).

## Status Updates

### 2026-06-24 — Keep achievements + member rename (all surfaces) done; Android catalog edit remains

**Keep achievements edit** (`keep.js`): added `fillAchForm(a)` round-tripping the criterion
(PointsEarned / TotalCompletions / Streak) and scope (Any / Quest / Category) into the form selects;
`makeEditable(achForm)`; an **Edit** button on achievement rows in `loadCatalog`; submit uses
`achEdit.id ?? Date.now()`. E2E: `edit an achievement in place` (round-trips criterion, renames in
place, no duplicate).

**Member rename (complete on all surfaces):** reuses the `PutUser` upsert (no new domain/store
variant) — read the user, replace `display_name`, apply `Change::PutUser` (keeps id/role/active +
`created_*` audit). Added:
- phone API `POST /admin/members/{id}/name` (`RenameMemberReq`, RequireKnight) — registered in router
  + openapi; SDK regenerated (`KnightApi.renameMember`, `RenameMemberReq`).
- Keep `POST /api/members/{id}/name` + a **Rename** button per member row (`keep.js`, prompt-based).
- Android `MemberAdminScreen` **Rename** dialog + `KnightApiAdapter.renameMember`.
- Tests: Rust `knight_renames_a_member_in_place` (204 / 400 blank / 404 unknown / role preserved /
  no duplicate); E2E `rename a member in place` (role + id kept, old name gone); Paparazzi
  `knightManageMembers` re-recorded (Rename button on every row).

**Verification:** 40 Rust suites green; Paparazzi verify green; 11 keep-achievements + keep-admin E2E
green (plus the earlier quest/reward edit E2E).

### 2026-06-24 (cont.) — Android catalog edit done; ticket complete

**Server:** extended the 3 flat summary DTOs with raw round-trip fields — `QuestSummaryDto` (+cash,
cadence, weekdays, due, assign_all, squires, icon), `ItemSummaryDto` (+description, availability,
gate, icon), `AchievementSummaryDto` (+criterion, scope, scope_quest/category, length, basis, count,
total). Added inverse-mapping helpers (`quest_cadence_flat`, `quest_assignment_flat`, `ach_flat`) and
a `Weekday → WeekdayDto` `From`; derived `Default` on the 4 flat enums for `serde(default)`
back-compat. Re-froze `openapi.json`; regenerated the Kotlin SDK (new nullable summary fields).

**Android:** all 3 `*AdminScreen.kt` gained an `editingId` + a `startEdit(summary)` that prefills the
form from the raw fields, an **Edit** button on each "Current …" card, submit reusing the id (relabel
to "Save changes" + a Cancel). Achievements reuse `CreateAchievementReq.copy(id = editingId)`.

**Verification:** Rust `quest_summary_carries_raw_round_trip_fields` (create a Weekly / specific-squire
/ cash / auto-approve quest → `GET /admin/quests` round-trips cadence, weekdays, assign_all, squires,
cash, auto_approve). New Paparazzi `knightEditCatalog` golden shows the Edit affordance on a reward
row. App compiles; 40 Rust suites + snapshot verify green.

**Ticket complete — the full SQUIRE-T-0120 scope is now closed on both surfaces.**
</content>