---
id: domain-core-scheduling-due-logic
level: task
title: "Domain Core: scheduling & due logic (quests_due)"
short_code: "SQUIRE-T-0004"
created_at: 2026-06-17T03:01:57.435794+00:00
updated_at: 2026-06-17T03:37:49.774524+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Domain Core: scheduling & due logic (quests_due)

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0001]] (Domain Core)

## Objective

Implement `quests_due(snap, squire, on)` — cadence resolution + assignment + Race-open gating + per-Squire already-satisfied subtraction, timezone-stable; plus the `QuestStatus` derivation for the view.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] Returns active quests whose cadence matches `on` AND for which `squire` is an assignee, minus those not currently claimable for that Squire (satisfied / pending non-repeatable).
- [x] OneOff (dated → its day only; undated → any day until done); Daily; Weekly{days} per matching weekday; `EveryNDays{n,anchor}` from anchor+interval (not before the anchor).
- [x] A M/W/F quest is due only on those days (Tue/Sat excluded).
- [x] A `Race` quest is due only while its `(quest,on)` occurrence is OPEN; once won it drops from every assignee's due list (winner → `CompletedToday`, others → `TakenByOther`).
- [x] Date/day-boundary math is pure integer arithmetic on the `Clock`-supplied `Date` (tz-resolved upstream), so it's timezone-stable (NFR-1.1.1).
- [x] `quest_status` derives Available/Pending/CompletedToday/TakenByOther per `(squire,quest,on)`.
- [x] `tests/due.rs` (10 tests): each cadence, assignment filter, satisfied/pending subtraction, repeatable-stays-due, Race open→closed + status, archived-never-due.

## Implementation Notes

### Technical Approach
Map `Schedule`→occurrence days from anchor; subtract satisfied occurrences from the log filtered by squire.

### Requirements covered
REQ-1.4.1; PRD FR-Q1/Q2/Q3; NFR-1.1.1; QuestStatus per ADR SQUIRE-A-0005.

### Dependencies
T-0001; "satisfied" depends on claim/approval semantics from T-0003.

## Status Updates

**2026-06-16 — Completed.** Implemented `Projections::quests_due` (`src/projections.rs`): for each quest, `cadence_matches(q, on) && submit_rejection(snap, squire, q, on).is_none()`. **Key refactor:** extracted `submit_rejection` into `src/common.rs` — the single predicate for "would a SubmitClaim be accepted" — and pointed BOTH `claims::submit` and `quests_due` at it, so the due list and the claim gate provably never disagree. Cadence math (`cadence_matches`/`schedule_matches`/`weekday_of`, `Date(0)=Monday` convention) is pure integer arithmetic over the `Clock`-supplied tz-resolved `Date` (tz-stable, NFR-1.1.1). Added `quest_status` (free pub fn, re-exported) deriving Available/Pending/CompletedToday/TakenByOther — incl. the Race winner-vs-sibling distinction. The four previously-dead cadence helpers are now live (warnings cleared). Tests `tests/due.rs` (10). Full suite **39 passed**. Committed.