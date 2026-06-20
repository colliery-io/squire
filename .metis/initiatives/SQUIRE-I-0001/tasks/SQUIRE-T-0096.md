---
id: hazards-negative-behaviors-that
level: task
title: "Hazards: negative behaviors that deduct coins/progress"
short_code: "SQUIRE-T-0096"
created_at: 2026-06-20T02:05:00.755604+00:00
updated_at: 2026-06-20T02:05:00.755604+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Hazards: negative behaviors that deduct coins/progress

## Request (operator, 2026-06-19)

A **hazards list** for the child — defined negative behaviors that can lose him coins (and/or
progress). The mirror image of quests: a catalog of named bad behaviors the parent can apply, each
deducting a set amount.

## Shape

- A **Hazard** definition (parent-authored, like a quest/achievement): name, icon, penalty (coins),
  maybe a category, active/archived.
- A parent action: **apply a hazard to a child** → emits a penalty event → deducts coins (clamped so
  the balance can't go negative? or allow debt? — decide).
- **History/audit:** shows in the child's "Recent activity" as a clearly-negative entry (red), with
  the reason — transparent, not a silent deduction. Mirrors the existing reject/reason flow.
- **Progress impact (optional):** could also break a streak or dent achievement progress. Start with
  coins-only; revisit progress effects later.

## Open design questions

- **Floor at zero or allow negative balance (debt)?** (Recommend clamp at 0 to start — matches the
  existing `clamp_balance`.)
- **Authoring surface:** Keep + phone (config must not be phone-only, per the operator).
- **Notify the child** when a hazard is applied (ties into the new coin/approval notification work —
  the child should know *why* coins dropped).
- **Idempotency / audit:** apply-hazard is a privileged command — same audited `by`-Knight pattern as
  adjustments.

## Likely scope

Domain (`Hazard` definition + `DefineHazard` / `ApplyHazard` / `ArchiveHazard` commands + a penalty
event), projections (balance deduction + activity entry), api/SDK, Keep + phone authoring, child
activity display (negative styling), notification on apply.

## Status

Captured. Needs a design pass with the operator (floor-vs-debt, progress effects) before
implementation. Related: [[SQUIRE-T-0095]] (real-money currency) and the child notification work.
