---
id: keep-item-achievement-authoring
level: task
title: "Keep: item & achievement authoring (define/archive)"
short_code: "SQUIRE-T-0027"
created_at: 2026-06-17T11:09:57.777171+00:00
updated_at: 2026-06-17T11:09:57.777171+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Keep: item & achievement authoring (define/archive)

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0004]] · ADR: [[SQUIRE-A-0008]]

## Objective

Keep authoring for **RedeemableItems** (cost, optional achievement `gate`, availability `Once`/`Repeatable` — no rate math) and **Achievements** (criterion, scope, bonus points), submitted engine-direct as Define/Archive, audited by the acting Knight.

## Acceptance Criteria

- [ ] Item form + endpoint: create/edit with cost, optional `gate` (achievement), `availability` ∈ {Once, Repeatable} only (no per-day/week/total fields) → `DefineItem`; archive → `ArchiveItem` (REQ-1.1.2, A-0006).
- [ ] Achievement form + endpoint: create/edit choosing criterion, scope, and bonus points → `DefineAchievement`; archive → `ArchiveAchievement` (REQ-1.1.3).
- [ ] Archive never deletes (`SetXActive(_, false)`); historical events still resolve (AR-2).
- [ ] Authoring commits pass `by` = acting Knight; forms surface last-editor metadata (A-0007); the item view surfaces `last_redeemed` and out-of-stock (for a redeemed `Once`), derived from the log (A-0006, REQ-1.4.1).
- [ ] Engine validation errors render as form errors, not 500s.
- [ ] Tests: define/list/edit/archive for both types; gate + availability round-trip; audit `by` stamped; `last_redeemed`/out-of-stock surfaced for a `Once` item after a redemption.
- [ ] `cargo test --workspace` green and warning-free.

## Implementation Notes

### Technical Approach
Same engine-direct form-post pattern as [[SQUIRE-T-0026]]; `store::item_audit` / `store::achievement_audit` for last-editor; derive `last_redeemed`/out-of-stock from the snapshot / `store::raw_log_for_item`.

### Dependencies
[[SQUIRE-T-0025]], [[SQUIRE-T-0026]] (shared authoring patterns). domain-core item/achievement contract. Spec REQ-1.1.2, REQ-1.1.3, REQ-1.4.1, REQ-1.4.2; A-0006, A-0007.

## Status Updates

*To be added during implementation*