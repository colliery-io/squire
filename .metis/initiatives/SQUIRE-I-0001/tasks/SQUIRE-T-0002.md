---
id: domain-core-authoring-commands
level: task
title: "Domain Core: authoring commands (define/archive quest, item, achievement)"
short_code: "SQUIRE-T-0002"
created_at: 2026-06-17T03:01:54.601472+00:00
updated_at: 2026-06-17T03:18:49.682855+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


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

- [ ] DefineQuest/DefineItem/DefineAchievement emit the matching `PutX` Change.
- [ ] Quest validation: assignment is non-empty; an explicit `Squires(set)` references existing **active Squires** (unknown → `UserNotFound`, non-Squire/inactive → `NotASquire`); `AllSquires` always valid; cadence + completion mode well-formed.
- [ ] ArchiveQuest/ArchiveItem/ArchiveAchievement emit `SetXActive(id,false)`; unknown id → the matching `…NotFound`; definitions are never deleted.
- [ ] A reward edit updates only the definition (past payouts untouched — verified jointly with T-0003's snapshot-at-approval).
- [ ] Unit tests per command, valid + invalid inputs.

## Implementation Notes

### Technical Approach
Pure validation against `Snapshot` (incl. `users` for assignee checks); emit Put/SetActive only.

### Requirements covered
REQ-1.1.2; PRD FR-A1..A3; assignment validation per ADR SQUIRE-A-0005.

### Dependencies
T-0001.

## Status Updates

*To be added during implementation*