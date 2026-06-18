---
id: achievement-authoring-on-the-phone
level: task
title: "Achievement authoring on the phone (LAN endpoints + native Manage Achievements + library)"
short_code: "SQUIRE-T-0072"
created_at: 2026-06-18T17:17:50.724992+00:00
updated_at: 2026-06-18T17:30:36.817242+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Achievement authoring on the phone (LAN endpoints + native Manage Achievements + library)

## Parent Initiative

[[SQUIRE-I-0001]]

## Objective

Bring achievement authoring to the native phone app at parity with quest authoring (T-0064/0065): a Knight can create, import-from-library, list, and archive achievements directly from the phone over the LAN api — no Keep/browser required.

## Acceptance Criteria

- [x] LAN api exposes `RequireKnight` `/admin/achievements` (GET list / POST create) and `/admin/achievements/{id}/archive` (POST), using the flat-DTO pattern (`AchCriterionKind`/`AchScopeKind`/`AchBasisKind`) to survive openapi-generator enum mangling.
- [x] `Option<Vec<…>>` / `Option<…>` request fields so the SDK's explicit JSON nulls don't 422.
- [x] `openapi.json` regenerated; conformance tests pass; SDK regenerated.
- [x] Native `AchievementAdminScreen` (Compose): name, criterion (Streak / TotalCompletions / PointsEarned), scope (Any / Category), per-criterion fields, bonus; starter-library import rows; current-achievements list with Archive.
- [x] Reachable from `KnightHomeScreen` overflow ⋮ menu ("Manage achievements"); wired through `KnightHomeHost`.
- [x] Bundled `achievements-library.json` (category-scoped to the quest library) in `app/src/main/assets/`.
- [x] Paparazzi snapshot (`knightManageAchievements`) renders the screen for image validation.
- [x] App builds (`:app:assembleDebug` SUCCESSFUL).

## Implementation Notes

### Technical Approach
- **api** (`crates/api/src/authoring.rs`): appended the achievement section mirroring the quest authoring shape — flat discriminant DTOs + a handler that reconstructs the domain `Achievement` (Scope/Criterion) and issues `Command::DefineAchievement` / `Command::ArchiveAchievement`. Domain validation (zero length/count/total, blank category, missing quest) flows through `domain_status`. Routes registered in `lib.rs`; schemas/paths in `openapi.rs`.
- **SDK**: `./gradlew :sdk:openApiGenerate` → `KnightApi.{listAchievements,createAchievement,archiveAchievement}` + the new models.
- **Android**: `KnightApiAdapter` gained the three achievement methods; `AchievementAdminScreen.kt` (reuses `ChoiceChip`/`SectionTitle`, made `internal`); `KnightHomeScreen` overflow menu; `KnightHomeHost` `managingAchievements` state. Test seams `initialAchievements`/`libraryOverride` skip the live fetch for Paparazzi.

### Dependencies
- Builds on T-0064/0065 (quest authoring) and T-0071 (Keep achievement authoring + starter library).

## Status Updates

**2026-06-18 — Done.**
- api: achievement DTOs + handlers + routes; `cargo build -p api` SUCCESS; `openapi.json` regenerated (conformance `frozen_openapi_matches_generated` + `frozen_openapi_has_key_schemas_and_paths` PASS).
- Curl-verified against rebuilt `squire-home`: create points (with explicit nulls) → 200; streak + Category → 200; empty category → 400; GET list returns server-computed labels.
- SDK regenerated; `KnightApiAdapter` + `AchievementAdminScreen` + overflow menu + host wiring added; `:app:assembleDebug` BUILD SUCCESSFUL.
- Paparazzi `knightManageAchievements` recorded — screen renders correctly (form, criterion/scope chips, streak fields, bonus, starter-library import rows). Image validated.
- Quest-scoped achievements remain Keep-only on the phone for now (Any/Category native); follow-up if needed.