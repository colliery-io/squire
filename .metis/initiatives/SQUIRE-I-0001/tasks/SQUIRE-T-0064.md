---
id: lan-api-knight-quest-authoring
level: task
title: "LAN api: Knight quest-authoring endpoints (RequireKnight) + SDK"
short_code: "SQUIRE-T-0064"
created_at: 2026-06-18T12:14:12.691830+00:00
updated_at: 2026-06-18T14:04:05.280020+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# LAN api: Knight quest-authoring endpoints (RequireKnight) + SDK

## Parent Initiative

[[SQUIRE-I-0001]] · The phone-side of "the Keep should be usable from the app for Knights" (native, per the user's choice). Backend half; the native UI is [[SQUIRE-T-0065]].

## Objective

Expose quest authoring on the **LAN api** (the phone's transport) gated by `RequireKnight`, mirroring the Keep's create/list/archive but with **flat, codegen-friendly DTOs** so the generated Kotlin SDK stays clean. Regenerate `openapi.json` + the SDK.

## Acceptance Criteria

## Acceptance Criteria

- [x] `crate::authoring` (RequireKnight): `POST /admin/quests` (create/edit via `DefineQuest`, audited to the Knight), `GET /admin/quests` (flat `QuestSummaryDto` list with server-computed cadence/assignment labels), `POST /admin/quests/{id}/archive` (`ArchiveQuest`, 404 if absent). Routes registered.
- [x] **Flat DTOs** (no externally-tagged enums → no openapi-generator mangling): `CreateQuestReq { id?, title, reward, category?, cadence: CadenceKind, weekdays[], due?, completion: CompletionDto, assign_all, squires[], repeatable_within_day, auto_approve }` + `CadenceKind`/`WeekdayDto`/`CompletionDto` string enums + `CreatedQuest`/`QuestSummaryDto`. Handler reconstructs the domain `Cadence`/`Assignment`/`Completion`. New quests get a time-based id; edits keep theirs.
- [x] `openapi.json` regenerated (paths + 6 schemas) and the **conformance test passes**; `cargo test -p api` green. SDK regenerated: `CadenceKind` etc. are clean Kotlin string enums; `createQuest`/`listQuests`/`archiveQuest` methods generated.
- [x] Verified live on a throwaway server (Knight token): create daily(all)/weekly(MWF, squire 2, race) → 200; weekly-no-days → 400; `GET` labels render "Daily"/"Mon/Wed/Fri" + "All squires"/"Gawain".

## Implementation Notes

### Technical Approach
New `crates/api/src/authoring.rs` reusing `handle_command(by, cmd)` / `domain_status` / `RequireKnight`. Flat DTOs map → domain `Quest`. `assignment_label` resolves squire ids → display names from the snapshot. Registered in `lib.rs` routes + `openapi.rs` paths/schemas. Regen: `cargo run -p api --example gen_openapi`, then `:sdk:openApiGenerate`. The assignment picker on the phone reuses the squires already in `HouseholdReview` (no new endpoint).

### Dependencies
[[SQUIRE-T-0062]] (the authoring shape the Keep proved). Frozen-contract regen (openapi + SDK). Native UI: [[SQUIRE-T-0065]].

### Risk Considerations
Externally-tagged `Cadence`/`Assignment` would mangle in codegen — avoided via flat DTOs (the established DecisionDto pattern). `openapi.json` is frozen + conformance-tested, so it MUST be regenerated (done). int64 wire ids (QuestId/UserId schema `value_type=i64`).

## Status Updates

**2026-06-18 — Done + verified.** Added `crate::authoring` with the three RequireKnight endpoints and flat DTOs; registered routes + openapi paths/schemas; regenerated `openapi.json` (conformance test green) and the Kotlin SDK (clean string enums, `createQuest`/`listQuests`/`archiveQuest` generated). Live-tested on a throwaway server: create daily/weekly → 200 with id; weekly-no-days → 400; `GET /admin/quests` returns correct labels (incl. squire id → "Gawain"). `cargo test -p api` fully green. Next: [[SQUIRE-T-0065]] native Quests tab in the Knight app.