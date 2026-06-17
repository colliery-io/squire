---
id: domain-core-authoring-commands
level: task
title: "Domain Core: authoring commands (define/archive quest, item, achievement)"
short_code: "SQUIRE-T-0002"
created_at: 2026-06-17T03:01:54.601472+00:00
updated_at: 2026-06-17T03:27:50.148568+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Domain Core: authoring commands (define/archive quest, item, achievement)

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0001]] (Domain Core)

## Objective

Implement `handle` for the six authoring commands → validated `Change`s (`PutQuest`/`PutItem`/`PutAchievement`, `SetQuestActive`/`SetItemActive`/`SetAchievementActive`).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] DefineQuest/DefineItem/DefineAchievement emit the matching `PutX` Change.
- [x] Quest validation: assignment is non-empty; an explicit `Squires(set)` references existing **active Squires** (unknown → `UserNotFound`, non-Squire/inactive → `NotASquire`); `AllSquires` always valid; cadence well-formed (Weekly non-empty days, EveryNDays n≥1).
- [x] ArchiveQuest/ArchiveItem/ArchiveAchievement emit `SetXActive(id,false)`; unknown id → the matching `…NotFound`; definitions are never deleted.
- [x] A reward edit upserts the definition only (`PutQuest` replaces by id; past `CompletionApproved` payouts untouched — full snapshot-at-approval proof lands in T-0003).
- [x] Unit tests per command, valid + invalid inputs (12 tests in `tests/authoring.rs`).

## Implementation Notes

### Technical Approach
Pure validation against `Snapshot` (incl. `users` for assignee checks); emit Put/SetActive only.

### Requirements covered
REQ-1.1.2; PRD FR-A1..A3; assignment validation per ADR SQUIRE-A-0005.

### Dependencies
T-0001.

## Status Updates

**2026-06-16 — Completed.** Implemented `crate::authoring::handle` (new `src/authoring.rs`) for all six commands. Define = upsert → `PutQuest`/`PutItem`/`PutAchievement`; Archive → `SetXActive(id,false)` after an existence check (`…NotFound` otherwise), never deletes. Validation: quest `Squires(set)` must be non-empty and all active Squires (`UserNotFound`/`NotASquire`), `AllSquires` always valid; degenerate schedules rejected; item `gate` must reference an existing achievement; achievement `Quest`-scope must reference an existing quest; zero-length/-count/-total criteria rejected. Shared lookups/validation live in new `src/common.rs` (`find_*`, `require_active_squire`, `is_assignee`).

Surfaced a contract gap → added **`DomainError::InvalidDefinition`** (malformed authoring input) to both `crate::contract::errors` and the root `shared_contract.rs`.

Refactor (also per a mid-task user request): the contract is now a `contract/` module with per-area submodules (primitives, identity, definitions, events, commands, ports, errors, api), re-exported so `crate::contract::*` is unchanged. Per-family handlers moved to their own modules (`authoring`/`claims`/`redemption`); `engine.rs` dispatch delegates to them.

Tests: `tests/authoring.rs` (12) + scaffold (6) → **all green**. Committed `647405c`.