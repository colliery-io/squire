---
id: domain-core-crate-scaffold-ports
level: task
title: "Domain Core: crate scaffold, ports & handle dispatch"
short_code: "SQUIRE-T-0001"
created_at: 2026-06-17T03:01:53.286693+00:00
updated_at: 2026-06-17T03:12:51.102151+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Domain Core: crate scaffold, ports & handle dispatch

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0001]] (Domain Core)

## Objective

Stand up the pure `domain-core` crate (contract types as the domain crate, no behavior), implement `Engine`/`Projections` trait skeletons, in-memory `Repository` + controllable fake `Clock` test doubles, `handle`'s command-dispatch and actor/path routing, and a property-test harness. This is the foundation for all other Domain Core tasks.

## Acceptance Criteria

## Acceptance Criteria

- [ ] Crate builds with NO storage/transport/UI/clock deps (pure); depends only on the `Clock`/`Repository` ports.
- [ ] `handle` dispatches every `Command` variant to a per-family handler (stubs OK) and applies nothing itself (returns `Vec<Change>`/`DomainError`).
- [ ] An admin-only command arriving on the child path returns `BadCommandForActor`.
- [ ] In-memory `Repository` (apply records Changes; snapshot returns state incl. `users`) and a fake `Clock` exist; an apply→snapshot round-trip test passes.
- [ ] proptest (or equiv) harness wired; a smoke property test runs.

## Implementation Notes

### Technical Approach
Match on `Command`; thread `&dyn Clock`; model the caller path/role so child-path admin commands are rejected. Keep handlers as stubs to be filled by T-0002..T-0006.

### Requirements covered
REQ-1.1.1; REQ-1.1.2 (dispatch + BadCommandForActor only); NFR-1.1.1 (Clock injection), NFR-1.1.3 (in-memory repo + proptest), NFR-1.1.4 (purity).

### Dependencies
None — unblocks T-0002..T-0007.

## Status Updates

*To be added during implementation*