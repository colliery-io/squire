---
id: keep-quest-authoring-define
level: task
title: "Keep: quest authoring (define/archive, assignment, completion mode, forward-only reward)"
short_code: "SQUIRE-T-0026"
created_at: 2026-06-17T11:09:56.310648+00:00
updated_at: 2026-06-17T11:09:56.310648+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Keep: quest authoring (define/archive, assignment, completion mode, forward-only reward)

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0004]] · ADR: [[SQUIRE-A-0008]]

## Objective

The Keep's **quest authoring** surface: create/edit/archive quests with all fields — including `assignment` (AllSquires or an explicit Squire subset) and `completion` mode (EachAssignee vs Race) — submitted **engine-direct** as `DefineQuest` / `ArchiveQuest`. Reward edits are forward-only; archive never deletes; every commit is audited by the acting Knight.

## Acceptance Criteria

- [ ] Authoring form + endpoint to create/edit a quest with title, description, inline free-form category, reward, cadence (all §6 cadence kinds), `auto_approve`, `repeatable_within_day`, icon, **assignment** (AllSquires or explicit subset) and **completion** (EachAssignee/Race) → committed as `DefineQuest` via the engine-direct `commit` (REQ-1.1.1, A-0005).
- [ ] Archive a quest via `ArchiveQuest` (a `SetQuestActive(_, false)`) — never a delete; archived quests stay referenced by historical events (AR-2). UI distinguishes active vs archived.
- [ ] Reward edits are **forward-only**: editing reward affects only future approvals; no UI path rewrites past payouts (REQ-1.1.4, AR-4).
- [ ] Every authoring commit passes `by` = acting Knight; the form surfaces last-editor metadata ("who set / last changed this") for the edited quest (REQ-1.4.2, A-0007).
- [ ] Engine validation errors (bad definition) render as actionable form errors, not 500s.
- [ ] Tests: define→list shows the active quest; edit reward affects only future approvals; archive → inactive but historical events still resolve; audit `by` stamped; assignment + completion round-trip (including a Race quest).
- [ ] `cargo test --workspace` green and warning-free.

## Implementation Notes

### Technical Approach
HTML form posts → handler builds `Command::DefineQuest{…}` / `ArchiveQuest{…}` and calls `KeepState::commit(Some(knight), cmd)`. Read current quests via `Repository::snapshot`; surface audit via `store::quest_audit`. Keep JS minimal (form posts / progressive enhancement).

### Dependencies
[[SQUIRE-T-0025]] (scaffold + `commit` seam). domain-core quest contract (`Assignment`/`Completion`/`Cadence`); `store::quest_audit`. Spec REQ-1.1.1, REQ-1.1.4, REQ-1.4.2; A-0005, A-0006, A-0007.

## Status Updates

*To be added during implementation*