---
id: domain-core-scheduling-due-logic
level: task
title: "Domain Core: scheduling & due logic (quests_due)"
short_code: "SQUIRE-T-0004"
created_at: 2026-06-17T03:01:57.435794+00:00
updated_at: 2026-06-17T03:01:57.435794+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Domain Core: scheduling & due logic (quests_due)

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0001]] (Domain Core)

## Objective

Implement `quests_due(snap, squire, on)` — cadence resolution + assignment + Race-open gating + per-Squire already-satisfied subtraction, timezone-stable; plus the `QuestStatus` derivation for the view.

## Acceptance Criteria

- [ ] Returns active quests whose cadence matches `on` AND for which `squire` is an assignee, minus those already satisfied **for that Squire** on `on`.
- [ ] OneOff due until that Squire has an approved completion; Daily/Weekly{days} per matching day; `EveryNDays{n,anchor}` computed from anchor+interval.
- [ ] A M/W/F quest is due only on those days (weekend gap doesn't make it due).
- [ ] A `Race` quest is due only while its `(quest,on)` occurrence is OPEN; once won it drops from every assignee's due list (non-winners see `QuestStatus::TakenByOther`).
- [ ] Date/day-boundary math is timezone-stable (DST/travel) via the `Clock` port.
- [ ] `QuestStatus` (Available/Pending/CompletedToday/TakenByOther) derived per `(squire,quest,on)`.
- [ ] Unit tests per cadence, assignment, and Race-open.

## Implementation Notes

### Technical Approach
Map `Schedule`→occurrence days from anchor; subtract satisfied occurrences from the log filtered by squire.

### Requirements covered
REQ-1.4.1; PRD FR-Q1/Q2/Q3; NFR-1.1.1; QuestStatus per ADR SQUIRE-A-0005.

### Dependencies
T-0001; "satisfied" depends on claim/approval semantics from T-0003.

## Status Updates

*To be added during implementation*