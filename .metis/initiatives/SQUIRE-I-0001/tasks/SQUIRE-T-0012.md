---
id: store-durability-per-tenant-single
level: task
title: "Store: durability & per-tenant single-file export/import"
short_code: "SQUIRE-T-0012"
created_at: 2026-06-17T04:08:44.915643+00:00
updated_at: 2026-06-17T04:08:44.915643+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Store: durability & per-tenant single-file export/import

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0002]] (Persistence & Store)

## Objective

Prove durability across restart and provide a per-tenant single-file export plus restore/import, consistent with respect to the single writer. RESOLVE the export-format decision area and record the choice.

## Acceptance Criteria

- [ ] Committed writes survive reopening the store (close/reopen → data intact) — NFR-2.1 / NFR-4.
- [ ] `export(path)` writes the whole household store (users + definitions + complete event log) to ONE file; `import(path)` / restore round-trips to an equal store (snapshot equality).
- [ ] Export is consistent (no torn export mid-`apply`).
- [ ] Tests (SQLite): apply → export → import into a fresh store → snapshots equal; reopen-after-write durability.

## Implementation Notes

### Technical Approach
RESOLVE the export-format decision area and document the choice in the task log. SQLite options: a consistent file-level copy (`VACUUM INTO` or the backup API) or a serialized dump; prefer a backend-portable serialized dump if that is simpler to test on SQLite. Export must be consistent with the single writer — no torn export captured mid-`apply` (snapshot/transaction boundary). Note `pg_dump --schema` as the hosted Postgres analog (not run here). Durability is proven by closing and reopening the store and confirming committed data is intact (NFR-2.1 / NFR-4); round-trip is proven by apply → export → import into a fresh store → snapshot equality.

ENV CAVEAT: durability and export/import round-trip are tested on the SQLite path; `pg_dump --schema` is the documented Postgres analog and is not exercised in this task's tests.

### Requirements covered
REQ-1.5, NFR-2.1; the export-format decision area; ADR A-0002.

### Dependencies
SQUIRE-T-0008 (schema/backend), SQUIRE-T-0010 (apply/snapshot for consistent export + round-trip).

## Status Updates

*To be added during implementation*