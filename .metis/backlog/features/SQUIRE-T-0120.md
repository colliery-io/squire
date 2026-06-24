---
id: edit-authored-objects-in-place
level: task
title: "Edit authored objects in place (quests/rewards/achievements/members)"
short_code: "SQUIRE-T-0120"
created_at: 2026-06-24T01:17:25.724248+00:00
updated_at: 2026-06-24T02:09:49.763952+00:00
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

# Edit authored objects in place (quests/rewards/achievements/members)

## Objective

Let parents **edit** an authored object (fix a quest title/reward, rename a reward, tweak an
achievement, rename a member) instead of only create + archive. Today every authoring surface is
effectively CRD — there is no "U".

## Backlog Item Details

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P1 - High (important for user experience)

### Business Justification
- **User Value**: A typo or wrong point value currently forces archive-and-recreate, which loses the
  object's identity and clutters the catalog. Editing in place is table-stakes admin UX.
- **Effort Estimate**: **S–M** for quests/rewards/achievements (UI-only). **M** for members (needs a
  new backend rename path).

## Verified behavior (current state)

**The backend already fully supports update-in-place via "define with existing id" (upsert). The
gap is purely the UI never reuses an id.**

- Domain: `crates/domain-core/src/authoring.rs:1-2` — "Define is upsert (create or edit)".
  `DefineQuest`/`DefineItem`/`DefineAchievement` each emit a single `Put*` change.
- Store: `crates/store/src/lib.rs:573-624` — `upsert_*` implemented as Diesel
  `on_conflict(id).do_update()`; same id replaces the row, preserving `created_*` audit and moving
  only `updated_*`.
- Proof: `crates/domain-core/tests/authoring.rs:115-126` (`reward_edit_upserts_definition`) —
  redefining quest id 10 yields `quests.len() == 1` with the new reward (edit, not append).
- API accepts an existing id already: `CreateQuestReq.id: Option<QuestId>`
  (`crates/api/src/authoring.rs:90`), same for `CreateItemReq.id` (`:560`) and
  `CreateAchievementReq.id` (`:353`). The Kotlin SDK mirrors these (`id: Long?`).

**Where the id is thrown away (the actual defect):**
- Keep web: `crates/keep/assets/keep.js:670` hardcodes `id: Date.now()`; library import uses
  `freshId()` (`:709`). List rows render an **Archive** button only (`keep.js:614-633`, `:403-431`)
  — no Edit.
- Android Knight: every submit passes `id = null` —
  `clients/squire-android/app/src/main/kotlin/com/squire/knight/app/ui/QuestAdminScreen.kt:162,298`,
  `RewardAdminScreen.kt:141,266`, `AchievementAdminScreen.kt:326`. Lists offer Archive only.

**Members are the one true backend gap:** only `add_member` (create) + `set_member_active`
(de/reactivate) exist (`crates/api/src/authoring.rs:780`, `crates/identity/src/lib.rs`). The store
`PutUser` path is an upsert (`store/src/lib.rs:542,581-595`) but no command/endpoint re-puts an
existing member's `display_name`/`role`.

**History-safety confirmed (important):** rewards are snapshotted into the append-only log at
approval (`Event::CompletionApproved { points, at }`, `crates/domain-core/src/claims.rs:99-116`);
balances sum those immutable events, not the current definition. So editing a reward is forward-only
— past approvals keep their original points. Editing is safe (already documented at
`keep/src/quests.rs:6-8`).

## Acceptance Criteria

## Acceptance Criteria

**Scoped (in this ticket): Keep-web edit for the daily-driver objects — quests + rewards.** The
remainder (Keep achievements, Android, member rename) split to [[SQUIRE-T-0126]].

- [x] Keep web: quest and reward list rows have an **Edit** action that loads the row back into its
      form (prefilled) and submits with the **existing id** (upsert), not a fresh one. E2E covered.
- [x] Editing upserts rather than duplicating — verified by the existing domain test
      (`reward_edit_upserts_definition`) and two new Keep E2E tests (rename + cost/reward change yield
      one row, old name gone).
- [x] Edited object keeps its id + `created_*` audit; only `updated_*` moves (store `on_conflict`
      upsert, unchanged).
- [→] Keep achievements edit — [[SQUIRE-T-0126]].
- [→] Android Knight edit (quests/rewards/achievements) — [[SQUIRE-T-0126]] (needs summary-DTO
      round-trip work).
- [→] Member rename (backend + UI) — [[SQUIRE-T-0126]].

## Implementation Notes

### Technical Approach
1. **Quests/Rewards/Achievements (UI-only):**
   - Keep `keep.js`: add Edit buttons; stash the row id; send it on submit instead of `Date.now()`;
     clear on form reset.
   - Android `*AdminScreen.kt`: add Edit on list cards → populate form state + remember id → pass it
     into the existing `Create*Req`.
   - Caveat: list summaries currently carry pre-rendered labels (`cadence_label`,
     `assignment_label`, availability/gate summaries). For a faithful edit form the GET may need to
     return the **raw** cadence/assignment/gate fields so the form round-trips. `QuestSummaryDto`
     already added `description` for this reason (`api/src/authoring.rs:131-133`); extend the summary
     DTOs (or echo the full domain object on GET) where a label can't round-trip.
2. **Members (backend + UI):** add a member-update operation (rename ± role) — natural home is the
   identity control-plane alongside `add_member` (members are identity-owned, not engine-commanded,
   per `api/src/authoring.rs:710`). Add `PUT/POST /admin/members/{id}` (RequireKnight), regen
   openapi/SDK, surface rename in `keep/src/members.rs` + `keep.js` + `MemberAdminScreen.kt`.
3. **Hazards:** no work — they're a single shared config list, already edited wholesale.

### Dependencies
- Member-rename touches the generated SDK → regen `crates/api/openapi.json` + Kotlin SDK.

### Risk Considerations
- Round-trip fidelity: an Edit form that can't reproduce all fields would silently drop them on
  save. Prefer returning raw fields over reverse-parsing labels.
- If role changes are allowed, guard against removing the last active Knight (self-lockout).

## Status Updates

### 2026-06-24 — Keep-web edit (quests + rewards) shipped; remainder split to SQUIRE-T-0126

**Key discovery that shaped scope:** the round-trip-fidelity caveat applies to the *phone* API
(flattened summary DTOs) and Android — **not** the Keep, whose `GET /api/quests` / `/api/items`
return the **raw domain objects** (`row.quest` / `row.item`). So Keep edit round-trips every field
with zero DTO work. I scoped this ticket to the Keep-web edit for the two daily-driver objects
(quests + rewards) and split the rest to [[SQUIRE-T-0126]].

**Implemented (Keep web, `crates/keep/assets/keep.js` + `keep.css`):**
- `makeEditable(form)` — a reusable edit-mode helper: stashes the id, relabels submit to "Save
  changes", shows a **Cancel**, and clears on `reset`.
- Quests: `fillQuestForm(q)` round-trips title/reward/cash/category/completion, cadence (Daily /
  Weekly-days / OneOff with a `fromDomainDate` inverse), assignment (All / specific squires), and the
  auto-approve / repeatable flags; an **Edit** button per active quest row; submit uses
  `questEdit.id ?? Date.now()`.
- Rewards: `fillItemForm(it)` round-trips name/description/cost/availability/gate; **Edit** button per
  active reward row; submit uses `itemEdit.id ?? Date.now()`.

**Verification:** two new Playwright E2E tests (`edit a quest in place`, `edit a reward in place`)
assert prefill, the "Save changes" relabel, that the rename + value change yield **one** updated row
(old name gone — upsert, not duplicate), and a return to create mode. Full keep + keep-rewards specs
(12 tests) green — no regressions. The upsert/audit guarantee is the unchanged store `on_conflict`
path, already covered by `reward_edit_upserts_definition`.

**Deferred to [[SQUIRE-T-0126]]:** Keep achievements edit (same pattern + `fillAchForm`); Android
edit for all three (needs summary-DTO round-trip fields); member rename (the one true backend gap).
</content>