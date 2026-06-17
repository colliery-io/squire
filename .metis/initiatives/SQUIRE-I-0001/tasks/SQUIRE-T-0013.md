---
id: store-repository-conformance-raw
level: task
title: "Store: Repository conformance, raw log query & perf"
short_code: "SQUIRE-T-0013"
created_at: 2026-06-17T04:08:45.773954+00:00
updated_at: 2026-06-17T04:57:24.890605+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Store: Repository conformance, raw log query & perf

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0002]] (Persistence & Store)

## Objective

Prove the SQLite-backed Repository is a faithful drop-in for the in-memory one (parity), add the raw per-quest / per-item event-log query, and add a perf check at scale.

## Acceptance Criteria

## Acceptance Criteria

- [ ] Conformance: drive representative scenarios through `DomainEngine.handle` → `apply` against the real SQLite store, then `snapshot()` + projections; assert results match the same scenarios run against the in-memory `Repository` (port a few domain-core flows: claim→approve→balance, redeem, streak/unlock, Race).
- [ ] `raw_log_for_quest(qid)` / `raw_log_for_item(iid)` return the ordered events touching that quest / item (NFR-11).
- [ ] Perf (NFR-2.2 / NFR-8): with thousands of events, `snapshot()` + a full projection sweep completes well under 100 ms (measure; strict gate in release, generous in debug).
- [ ] `cargo test -p store` green; perf gate passes in release.

## Implementation Notes

### Technical Approach
Build a conformance harness that applies engine-produced `Change` batches to the real SQLite store and compares projection outputs against the same scenarios run on the in-memory `Repository`; port a few domain-core flows (claim→approve→balance, redeem, streak/unlock, Race) as the representative set. The raw-log query filters events by quest/item (joining via `claim` where the event does not carry the id directly) and returns them in append order (NFR-11). The perf test seeds a few thousand events, then times `snapshot()` plus a full projection sweep; gate strictly (well under 100 ms) in release and generously in debug.

ENV CAVEAT: conformance, raw-log, and perf all run on the SQLite path (the tested backend); the Postgres path remains compile-only / `DATABASE_URL`-gated.

### Requirements covered
REQ-1.6, NFR-2.2 (NFR-8), NFR-11; conformance across REQ-1.1 / 1.2.

### Dependencies
SQUIRE-T-0008 .. SQUIRE-T-0012 (full store stack: schema, mapping, apply/snapshot, provisioning, export).

## Status Updates

*To be added during implementation*