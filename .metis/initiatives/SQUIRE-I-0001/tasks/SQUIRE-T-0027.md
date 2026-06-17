---
id: keep-item-achievement-authoring
level: task
title: "Keep: item & achievement authoring (define/archive)"
short_code: "SQUIRE-T-0027"
created_at: 2026-06-17T11:09:57.777171+00:00
updated_at: 2026-06-17T11:35:33.712793+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Keep: item & achievement authoring (define/archive)

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0004]] · ADR: [[SQUIRE-A-0008]]

## Objective

Keep authoring for **RedeemableItems** (cost, optional achievement `gate`, availability `Once`/`Repeatable` — no rate math) and **Achievements** (criterion, scope, bonus points), submitted engine-direct as Define/Archive, audited by the acting Knight.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] Item form + endpoint: create/edit with cost, optional `gate` (achievement), `availability` ∈ {Once, Repeatable} only (no per-day/week/total fields) → `DefineItem`; archive → `ArchiveItem` (REQ-1.1.2, A-0006).
- [x] Achievement form + endpoint: create/edit choosing criterion, scope, and bonus points → `DefineAchievement`; archive → `ArchiveAchievement` (REQ-1.1.3). *(UI form covers the PointsEarned criterion; the API accepts any criterion/scope via serde.)*
- [x] Archive never deletes (`SetXActive(_, false)`); historical events still resolve (AR-2).
- [x] Authoring commits pass `by` = acting Knight; forms surface last-editor metadata (A-0007); the item view surfaces `last_redeemed` and out-of-stock (for a redeemed `Once`), derived from the log (A-0006, REQ-1.4.1).
- [x] Engine validation errors render as form errors, not 500s. *(item gate→missing = 404; achievement invalid criterion = 400.)*
- [x] Tests: define/list/edit/archive for both types; gate + availability round-trip; audit `by` stamped; `last_redeemed`/out-of-stock surfaced for a `Once` item after a redemption.
- [x] `cargo test --workspace` green and warning-free.

## Implementation Notes

### Technical Approach
Same engine-direct form-post pattern as [[SQUIRE-T-0026]]; `store::item_audit` / `store::achievement_audit` for last-editor; derive `last_redeemed`/out-of-stock from the snapshot / `store::raw_log_for_item`.

### Dependencies
[[SQUIRE-T-0025]], [[SQUIRE-T-0026]] (shared authoring patterns). domain-core item/achievement contract. Spec REQ-1.1.2, REQ-1.1.3, REQ-1.4.1, REQ-1.4.2; A-0006, A-0007.

## Status Updates

**2026-06-17 — Done (`9d76cf0`).** Item + achievement authoring, engine-direct.

- **store**: added `item_audit` / `achievement_audit` accessors (mirror `quest_audit`/`user_audit`).
- **`keep::items`**: `GET /api/items` (list + audit + log-derived `last_redeemed` & out-of-stock for a redeemed `Once`, per A-0006/AR-3), `POST /api/items` (`DefineItem` upsert, `by` = Knight), `POST /api/items/{id}/archive`. A gate to a missing achievement → 404.
- **`keep::achievements`**: `GET/POST /api/achievements` + `/{id}/archive` (`Define`/`ArchiveAchievement`). Invalid criterion (e.g. `PointsEarned{total:0}`) → 400.
- **UI**: rewards + achievements panels (create/list/archive) in the embedded shell.
- **Tests** (`tests/catalog.rs`, 8): item create+audit; gate-missing 404; 401 unauth; `Once` out-of-stock vs `Repeatable` after redemption; archive + 404; achievement create+audit; invalid 400; valid-gate + archive + 404.

**Verification.** `cargo test --workspace` green & warning-free (38 binaries).