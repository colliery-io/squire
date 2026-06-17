---
id: store-durability-per-tenant-single
level: task
title: "Store: durability & per-tenant single-file export/import"
short_code: "SQUIRE-T-0012"
created_at: 2026-06-17T04:08:44.915643+00:00
updated_at: 2026-06-17T04:57:02.178731+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Store: durability & per-tenant single-file export/import

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0002]] (Persistence & Store)

## Objective

Prove durability across restart and provide a per-tenant single-file export plus restore/import, consistent with respect to the single writer. RESOLVE the export-format decision area and record the choice.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] Committed writes survive reopening the store (close conn / reopen same file or PG schema → data intact) — NFR-2.1/NFR-4.
- [x] `export(path)` writes the whole household store (users + definitions + complete event log, **incl. audit columns + `seq`**) to ONE JSON file; `import(path)` bulk-restores verbatim into a fresh store → snapshot equality (+ audit/`seq` preserved, not re-stamped).
- [x] Export reads inside a single read transaction → coherent point-in-time, no torn export mid-`apply` (single-writer makes it safe; the read-txn makes it explicit).
- [x] Tests on **both backends** (`tests/backup.rs`): durability across reopen (SQLite file + PG schema reconnect), apply→export→import→snapshot-equal incl. audit/`seq` spot-check, single non-empty export file.

## Implementation Notes

### Technical Approach
RESOLVE the export-format decision area and document the choice in the task log. SQLite options: a consistent file-level copy (`VACUUM INTO` or the backup API) or a serialized dump; prefer a backend-portable serialized dump if that is simpler to test on SQLite. Export must be consistent with the single writer — no torn export captured mid-`apply` (snapshot/transaction boundary). Note `pg_dump --schema` as the hosted Postgres analog (not run here). Durability is proven by closing and reopening the store and confirming committed data is intact (NFR-2.1 / NFR-4); round-trip is proven by apply → export → import into a fresh store → snapshot equality.

ENV CAVEAT: durability and export/import round-trip are tested on the SQLite path; `pg_dump --schema` is the documented Postgres analog and is not exercised in this task's tests.

### Requirements covered
REQ-1.5, NFR-2.1; the export-format decision area; ADR A-0002.

### Dependencies
SQUIRE-T-0008 (schema/backend), SQUIRE-T-0010 (apply/snapshot for consistent export + round-trip).

## Status Updates

**2026-06-17 — Completed.** **Export format (resolves the decision area):** a backend-portable single-file **JSON dump of the five row structs** (`Dump { users, quests, items, achievements, events }`), capturing every column incl. audit (`created_by/at`, `updated_by/at`) and the append-only `seq`. Portable because it serializes at the *row* layer (the same shape Diesel reads on both backends) rather than a backend-specific artifact — a SQLite tenant's dump can restore into a Postgres tenant and vice-versa. (`pg_dump --schema` / SQLite `VACUUM INTO` are the native alternatives; rejected as non-portable.) New `crates/store/src/backup.rs` (`export`/`import`/`Dump`/`BackupError`); `serde`+`serde_json` added; `Serialize/Deserialize` derived on the row structs (domain contract untouched); `Store::export` + `Provisioner::import` conveniences.

**Consistency:** `export` reads all five tables inside one `conn.transaction` read txn (coherent point-in-time; never captures a half-applied batch). `import` is a verbatim bulk INSERT (not `apply`) in one transaction, so audit/`seq` restore exactly. (Multi-row insert dispatches to the concrete `SqliteConnection`/`PgConnection` like the existing upsert macro, since erased `MultiConnection` can't type-check `BatchInsert`.)

Tests `tests/backup.rs` (both backends): durability across reopen (SQLite file + PG schema reconnect), apply→export→import→snapshot-equal with audit/`seq` spot-checks, single non-empty file. Also gated a `postgres`-only import (`SimpleConnection`) so the default build is warning-free. Results: SQLite `cargo test -p store` → 25 green, 0 warnings (default + `--features postgres`); Postgres run → backup 4 (incl. PG durability + round-trip) green; `cargo test --workspace` → domain-core 70 intact. Committed.