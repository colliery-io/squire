---
id: store-domain-row-mapping-lossless
level: task
title: "Store: domain ↔ row mapping (lossless serialization)"
short_code: "SQUIRE-T-0009"
created_at: 2026-06-17T04:08:41.165831+00:00
updated_at: 2026-06-17T04:26:16.554410+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Store: domain ↔ row mapping (lossless serialization)

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0002]] (Persistence & Store)

## Objective

Implement lossless encode/decode between the domain types and their rows for every `User`, every definition (and the nested enums `Cadence`/`Schedule`/`Assignment`/`Completion`/`Availability`/`Criterion`/`Scope`/`StreakBasis`/`Role`/`Category`/`Weekday`), and every `Event` variant with all of its fields (`claim_id`/`request_id`/`command_id`/`squire`/`actor`/`points`/`cost`/`bonus`/`amount`/`reason`/`on`/`at`). Encoding must be backend-portable.

## Acceptance Criteria

## Acceptance Criteria

- [ ] Each `User`/`Quest`/`RedeemableItem`/`Achievement` maps to/from its row (ids encoded portably, enums encoded portably, `BTreeSet<UserId>`/`BTreeSet<Weekday>` serialized losslessly, `Option`s handled).
- [ ] Every `Event` variant round-trips (a `kind` discriminator + typed/nullable columns, or a portable encoding), preserving order and snapshotted values exactly (AR-4); event rows are insert-only.
- [ ] Property/round-trip tests: encode→decode == original for randomized values of every type and every event variant.
- [ ] No backend-specific encoding.

## Implementation Notes

### Technical Approach
Build mapping on the column layout fixed in T-0008. Encode `u128` ids portably as TEXT (or `BLOB(16)`); represent small enums as TEXT tags; serialize sets (`BTreeSet<UserId>`, `BTreeSet<Weekday>`) as sorted delimited text or via a child table — whichever is lossless and portable. Event variants map to a `kind` discriminator plus typed/nullable columns capturing the full field set; rows are insert-only and must preserve append order and exact snapshotted values (AR-4). Drive correctness with property/round-trip tests over randomized values.

ENV CAVEAT: round-trip and property tests run on the SQLite path; the encoding is backend-agnostic so it applies equally to Postgres (live PG verification gated on `DATABASE_URL`).

### Requirements covered
REQ-1.2 / 1.3 / 1.4 mapping; the serialization decision area; A-0004 / A-0005 / A-0006 field set.

### Dependencies
SQUIRE-T-0008 (column layout / schema must be fixed first).

## Status Updates

*To be added during implementation*