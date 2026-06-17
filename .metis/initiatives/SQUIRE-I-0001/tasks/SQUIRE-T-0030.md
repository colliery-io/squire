---
id: keep-event-log-inspector
level: task
title: "Keep: event-log inspector, definition audit surfacing & integration tests"
short_code: "SQUIRE-T-0030"
created_at: 2026-06-17T11:10:01.147679+00:00
updated_at: 2026-06-17T11:10:01.147679+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Keep: event-log inspector, definition audit surfacing & integration tests

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0004]] · ADR: [[SQUIRE-A-0008]]

## Objective

The read-only **event-log inspector** and **definition-audit surfacing**, plus the Keep's end-to-end integration suite proving authoring→review→state and the no-network-authoring / loopback guarantees.

## Acceptance Criteria

- [ ] Event-log inspector: list raw ordered `Event`s for a given quest or item, including the committing `actor` for Knight-committed facts, read-only over the snapshot / raw log (REQ-1.4.1, NFR-1.1.1).
- [ ] Definition inspector surfaces last-editor metadata (`created_by`/`updated_by` + timestamps) for quests/items/achievements, and a reward's `last_redeemed` + out-of-stock (A-0007, A-0006).
- [ ] Integration suite (oneshot): full flow — author quest/item → add a squire → squire claim (seeded) → Keep approves → state reflects the credit; "who set X" and "who added member X" are answerable.
- [ ] Guarantee tests: the Keep's command path never calls the network `api` (the `keep` crate has **no `api` dependency** and makes no loopback HTTP self-call), and the admin server binds loopback-only (A-0008, AR-8).
- [ ] `cargo test --workspace` green and warning-free; the `keep` crate is documented.

## Implementation Notes

### Technical Approach
Use `store::raw_log_for_quest` / `store::raw_log_for_item` + the `*_audit` accessors to render read-only views. Integration tests drive the Keep router via `oneshot`; assert engine-direct (the crate's dependency graph excludes `api`) and the loopback bind address.

### Dependencies
[[SQUIRE-T-0025]]..[[SQUIRE-T-0029]]. store raw-log + audit accessors. Spec REQ-1.4.1, NFR-1.1.1; A-0006, A-0007, A-0008.

## Status Updates

*To be added during implementation*