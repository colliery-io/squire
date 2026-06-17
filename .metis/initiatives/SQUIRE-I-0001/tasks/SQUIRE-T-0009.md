---
id: store-domain-row-mapping-lossless
level: task
title: "Store: domain ↔ row mapping (lossless serialization)"
short_code: "SQUIRE-T-0009"
created_at: 2026-06-17T04:08:41.165831+00:00
updated_at: 2026-06-17T04:33:52.313670+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

## Acceptance Criteria

- [x] `User`/`Quest`/`RedeemableItem`/`Achievement` ↔ row (`UserRow`/`QuestRow`/`ItemRow`/`AchievementRow`): ids as decimal TEXT, all enums as TEXT tags, `BTreeSet<UserId>`/`BTreeSet<Weekday>` as sorted comma-delimited TEXT (empty ⇄ `""`), `Option`s handled; audit columns carried by the row (via an `Audit{by,at}` param), not the pure domain struct.
- [x] All 8 `Event` variants ↔ `EventRow` (`from_event(seq,&Event)`/`to_event`), preserving every field + the snapshotted points/cost (AR-4); rows insert-only; `seq` caller-assigned.
- [x] proptest pure round-trip (decode∘encode == original via debug-string equality) for every domain type + every event variant.
- [x] Backend-agnostic encoding, **verified on both backends**: a parametrized `each_backend` harness runs the DB insert→select round-trips on SQLite (always) AND on the compose Postgres (`--features postgres` + `DATABASE_URL`). Decoding is total (`RowError`, never panics).

## Implementation Notes

### Technical Approach
Build mapping on the column layout fixed in T-0008. Encode `u128` ids portably as TEXT (or `BLOB(16)`); represent small enums as TEXT tags; serialize sets (`BTreeSet<UserId>`, `BTreeSet<Weekday>`) as sorted delimited text or via a child table — whichever is lossless and portable. Event variants map to a `kind` discriminator plus typed/nullable columns capturing the full field set; rows are insert-only and must preserve append order and exact snapshotted values (AR-4). Drive correctness with property/round-trip tests over randomized values.

ENV CAVEAT: round-trip and property tests run on the SQLite path; the encoding is backend-agnostic so it applies equally to Postgres (live PG verification gated on `DATABASE_URL`).

### Requirements covered
REQ-1.2 / 1.3 / 1.4 mapping; the serialization decision area; A-0004 / A-0005 / A-0006 field set.

### Dependencies
SQUIRE-T-0008 (column layout / schema must be fixed first).

## Status Updates

**2026-06-17 — Completed.** New `crates/store/src/rows.rs`: `Insertable`/`Queryable`/`Selectable`/`AsChangeset` row structs (`UserRow`/`QuestRow`/`ItemRow`/`AchievementRow`/`EventRow`) matching `schema.rs`; per-type `encode`/`decode` (definition encoders take an `Audit{created_by/at, updated_by/at}` so T-0010 can stamp; decoders ignore audit → domain stays pure); helpers for id⇄TEXT, enum⇄tag (every enum), set⇄comma-delimited, bool⇄0/1, BigInt scalars, `Option`. Decode is total → `RowError{BadId,BadTag,BadInt,MissingField,BadWeekday}`, never panics. Added `proptest` to store dev-deps.

Tests `tests/mapping.rs` (11): proptest pure round-trips for all domain types + all 8 event variants (debug-string equality, no contract derives added), and a `each_backend` DB insert→select round-trip harness — SQLite temp-file always + the compose Postgres under `#[cfg(feature="postgres")]`+`DATABASE_URL` (resets via `store::pg::provision_clean`, serialized behind a `Mutex` since cargo runs tests concurrently and they share the PG `public` schema). Identical insert/select code runs on both backends via `AnyConnection`.

Results: SQLite `cargo test -p store` → mapping 11 + migrations 4 green; Postgres run (`--features postgres` + compose DB) → the 11 DB round-trips run on Postgres too, green; `cargo test --workspace` → domain-core 70 intact. Committed.