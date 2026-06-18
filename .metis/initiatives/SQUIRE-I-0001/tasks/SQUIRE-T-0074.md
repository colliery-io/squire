---
id: rewards-authoring-on-the-phone-lan
level: task
title: "Rewards authoring on the phone (LAN endpoints + native Manage Rewards + library)"
short_code: "SQUIRE-T-0074"
created_at: 2026-06-18T18:15:36.113290+00:00
updated_at: 2026-06-18T18:27:40.355729+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Rewards authoring on the phone (LAN endpoints + native Manage Rewards + library)

## Parent Initiative

[[SQUIRE-I-0001]]

## Objective

Complete the authoring trio on the native phone app: after Quests (T-0065) and Achievements (T-0072), add **Rewards** authoring. A Knight can create, import-from-library, list, and archive redeemable rewards directly from the phone over the LAN api — no Keep/browser required.

## Define-First: Expected Flows & Outcomes

### Flow R1 — Create a reward (Knight, phone)
- **Inputs**: name (required), cost in points (required, ≥ 1), availability (`Once` | `Repeatable`), optional gate (an existing achievement that must be unlocked first), optional icon (emoji, like quests).
- **Outcome**: `POST /admin/items` → `DefineItem`; reward appears in the on-screen catalog and in each child's reward shop.
- **Validation**: name non-blank + cost ≥ 1 at the form; gate→unknown achievement returns 400 (engine `validate_item`), surfaced as a friendly error.

### Flow R2 — Import from the starter library
- The same `rewards-library.json` bundled into `app/src/main/assets/` (parity with quests/achievements); one-tap Import per row → `DefineItem`.

### Flow R3 — List / archive
- Show every reward (active + archived) with availability + cost labels; Archive (never delete).

### Flow R4 — Gate selection
- The gate picker lists current achievements (fetched from `/admin/achievements`); "None" is the default. A freshly created achievement is selectable (re-fetched on screen entry).

## Acceptance Criteria

- [x] LAN api exposes `RequireKnight` `/admin/items` (GET list / POST create) and `/admin/items/{id}/archive` (POST), using the flat-DTO pattern (`AvailabilityKind` + `CreateItemReq` + `ItemSummaryDto`) with `Option<_>` request fields (no 422 on explicit nulls).
- [x] Engine gate validation flows through `domain_status` (gate→unknown achievement = **404** `AchievementNotFound`; the define-first draft said 400 — the engine maps it to NotFound, matching the Keep).
- [x] `openapi.json` regenerated; conformance tests pass; SDK regenerated.
- [x] `KnightApiAdapter` gains `listItems` / `createItem` / `archiveItem`.
- [x] Native `RewardAdminScreen` (Compose): name, description, cost, availability (Repeatable / Once) chips, gate picker (None + achievements as chips), icon; starter-library import rows; catalog list with Archive. Reuses `ChoiceChip`/`SectionTitle`.
- [x] Reachable from `KnightHomeScreen` overflow ⋮ menu ("Manage rewards"); wired through `KnightHomeHost`.
- [x] Bundled `rewards-library.json` in `app/src/main/assets/`.
- [x] Paparazzi snapshot (`knightManageRewards`) renders the screen for image validation.
- [x] App builds (`:app:assembleDebug` SUCCESSFUL).
- [x] api integration tests in `crates/api/tests/knight.rs` cover the explicit-null create, gate-404, archive + missing-404, and Squire-403 boundary (15 pass).

## Implementation Notes

### Technical Approach
- **api** (`crates/api/src/authoring.rs`): append an items section mirroring quests/achievements — `AvailabilityKind { Once, Repeatable }`, `CreateItemReq { id?, name, cost, availability, gate?, icon? }`, `ItemSummaryDto { id, name, summary, cost, active }`, handler rebuilds `RedeemableItem` and issues `Command::DefineItem` / `ArchiveItem`; server-computed `availability_label`. Routes in `lib.rs`; schemas/paths in `openapi.rs`.
- **SDK**: `./gradlew :sdk:openApiGenerate`.
- **Android**: `KnightApiAdapter` item methods; `RewardAdminScreen.kt`; overflow menu entry; `KnightHomeHost` `managingRewards` state; test seams `initialItems`/`libraryOverride` + `gateOptions` for Paparazzi.

### Dependencies
- Builds on T-0064/0065 (quest authoring), T-0072 (achievement authoring), and T-0073 (Keep rewards refine + library JSON, which this reuses).

## Status Updates

**2026-06-18 — Done.**
- api (`crates/api/src/authoring.rs`): appended the items section — `AvailabilityKind` flat enum (+`From` both ways), `CreateItemReq` (all-`Option` optionals), `CreatedItem`, `ItemSummaryDto` with server-computed `summary` ("Repeatable" / "Once · needs: <gate>"). Handler rebuilds `RedeemableItem` → `Command::DefineItem` / `ArchiveItem` via `domain_status`. Routes in `lib.rs`; paths+schemas in `openapi.rs`.
- `cargo build -p api` SUCCESS; `openapi.json` regenerated; conformance (2) PASS.
- **api integration tests** (`tests/knight.rs`, +4): explicit-null create → 200 (the T-0065 422 guard), gate→missing = 404, archive 204 + missing 404, Squire token = 403. `cargo test -p api --test knight` → 15 pass.
- SDK regenerated (`AvailabilityKind`, `CreateItemReq`, `CreatedItem`, `ItemSummaryDto`; `KnightApi.{listItems,createItem,archiveItem}`).
- Android: `KnightApiAdapter` item methods; `RewardAdminScreen.kt` (reuses `ChoiceChip`/`SectionTitle`, gate chips from `listAchievements()`); overflow "Manage rewards"; `KnightHomeHost` `managingRewards`. Bundled `rewards-library.json`. `:app:assembleDebug` SUCCESSFUL.
- Paparazzi `knightManageRewards` recorded — image-validated (form, availability+gate chips, library import rows).
- Validated via api oneshot tests + Paparazzi (no live curl / squire-home rebuild needed this pass).