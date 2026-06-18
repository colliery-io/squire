---
id: knight-app-native-quests-tab
level: task
title: "Knight app: native Quests tab (create / assign / schedule / import)"
short_code: "SQUIRE-T-0065"
created_at: 2026-06-18T12:14:13.850423+00:00
updated_at: 2026-06-18T14:32:41.782904+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Knight app: native Quests tab (create / assign / schedule / import)

## Parent Initiative

[[SQUIRE-I-0001]] · The UI half of "the Keep usable from the app for Knights" (native, per the user's choice). Consumes the [[SQUIRE-T-0064]] endpoints. **Closes the quest-creation initiative.**

## Objective

A native **Manage Quests** screen in the Knight app: create quests (assign all/specific squires, daily/weekly/one-time + due, completion, multiple-per-day, auto-approve), import from a bundled starter library, and archive — all over the LAN api's `RequireKnight` `/admin/quests`.

## Acceptance Criteria

## Acceptance Criteria

- [x] `QuestAdminScreen` (themed): a create form (title, reward, category, cadence chips Daily/Weekly/One-time + weekday chips + due-date picker, completion chips, All/Specific squires from the review, multiple-per-day + auto-approve toggles), a **starter library** (bundled `assets/library.json`) with one-tap Import, and a **Current quests** list with Archive.
- [x] `KnightApiAdapter` gains `listQuests`/`createQuest`/`archiveQuest` (generated `KnightApi`). Wired into `KnightHomeHost` via a `managingQuests` state + a **"Quests"** action in the Round Table top bar; `BackHandler` returns to the review home (not exit). `:app` gains the kotlinx-serialization plugin (for the bundled library model).
- [x] Verified live on the emulator (paired as Knight): screen renders; `listQuests` round-trips ("Current quests" shows the household quests with correct labels); **create succeeds end-to-end** (app POST → server quest created, confirmed in the list + server).
- [x] **Bug found + fixed during integration:** the SDK sent explicit `"weekdays": null` / `"squires": null`, but the server's `#[serde(default)] Vec<…>` only tolerated *absent* fields → **422**. Changed `CreateQuestReq.weekdays`/`squires` to `Option<Vec<…>>` (handler `unwrap_or_default()`); regenerated `openapi.json` (conformance green) + SDK. Null case now 200.

## Implementation Notes

### Technical Approach
`app/.../knight/app/ui/QuestAdminScreen.kt` — Compose form with `FilterChip` selectors + a Material3 `DatePicker` (due → Monday-aligned day-count). `assets/library.json` bundled (copy of the Keep's); parsed with kotlinx-serialization (plugin added to `:app`). Assignment picker reuses `HouseholdReview.squires`. Reuses `SquireTheme`/`SquireUi`. Diagnosed the 422 via a temporary auto-create + `Log.e` (removed).

### Dependencies
[[SQUIRE-T-0064]] (endpoints + SDK), [[SQUIRE-T-0056]] (theme). Frozen-contract regen.

### Risk Considerations
adb tap automation on Compose is unreliable (no stable hit targets) — verification leaned on the deterministic `listQuests` round-trip + a temporary on-open create. The 422 null-tolerance fix is the key correctness item. Optional follow-ups: in-app quest edit (id reuse already supported server-side), squire-token impersonation.

## Status Updates

**2026-06-18 — Done + verified end-to-end.** Built the native Manage Quests screen (form + library import + list/archive), wired the adapter + a "Quests" top-bar action + BackHandler, added the serialization plugin + bundled library to `:app`. Live on the emulator: renders, `listQuests` round-trips, and **create works** (confirmed app→server). Found + fixed a real 422 (SDK sent explicit nulls for `weekdays`/`squires`; made them `Option<Vec>` server-side, regen openapi+SDK, conformance green). Completes the quest-creation initiative (Keep cycle + timezone + native phone authoring).