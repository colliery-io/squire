---
id: knight-assume-squire-server-read
level: task
title: "Knight 'assume Squire' server read: GET any Squire's StateView (Knight-gated)"
short_code: "SQUIRE-T-0053"
created_at: 2026-06-17T23:00:00+00:00
updated_at: 2026-06-18T01:01:16.288054+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Knight 'assume Squire' server read: GET any Squire's StateView (Knight-gated)

## Parent Initiative

[[SQUIRE-I-0001]] · Spec [[SQUIRE-S-0006]] (Knight) / [[SQUIRE-S-0003]] (API) · enables the "assume Squire" UI ([[SQUIRE-T-0055]])

## Objective

For a Knight to "assume" a Squire (see their full player home), the server needs to return **any**
Squire's `StateView`, not just the caller's own. `GET /state` is self-scoped (the authenticated
user). Add a **Knight-gated** read of a chosen Squire's `StateView` — the same assembly `GET /state`
uses, for a `squire` id from the path. Acting on the assumed Squire still goes through the existing
Knight privileged surface (mark-done / redeem / adjust), so this task is **read-only**.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] `GET /admin/squire/{id}/state` (`RequireKnight`) returns that Squire's `StateView` (reuses `squire::assemble_state`, now `pub(crate)`, with the path id + clock today/now). Unknown id → 404; a non-Squire id (e.g. the Knight's own) → 404; a Squire token → 403 (extractor). Tenant-scoped (one snapshot of the Knight's tenant).
- [x] Wired in the api router + `#[utoipa::path]`; `openapi.json` re-frozen; SDK regenerated (`KnightApi.squireState`).
- [x] `cargo test --workspace` green (48 groups) incl. 4 new knight tests: Knight reads a Squire's state (squire id + quests_today present); unknown → 404; non-Squire id → 404; Squire token → 403.

## Implementation Notes

### Technical Approach
In `crates/api/src/knight.rs`, add a handler that parses the path id to a `UserId`, checks the snapshot has an active Squire with that id (else 404), and returns `Json(squire::assemble_state(&snap, id, today, now))`. `assemble_state` is already `pub(crate)`-ish in `squire.rs` — re-use it (make it visible if needed). Add the route `/admin/squire/{id}/state` and register the path in `openapi.rs`. Re-freeze + regen SDK per the T-0031 pipeline.

### Dependencies
[[SQUIRE-T-0015]] (`assemble_state`/`StateView`), [[SQUIRE-T-0016]] (Knight surface + `RequireKnight`).

### Risk Considerations
Read-only — no new write path, so the audit/single-writer model is untouched (acting is still the Knight's privileged commands). Keep it tenant-scoped (the Knight's household only). Path id parsing must reject non-numeric / cross-tenant.

## Status Updates

**2026-06-18 — Done.** Made `squire::assemble_state` `pub(crate)`; added `knight::squire_state` (`RequireKnight`, `Path<u64>` → `UserId`, 404 unless an active Squire, else `Json(assemble_state(...))`). Route `/admin/squire/{id}/state`, `#[utoipa::path]`, registered in `openapi.rs`; re-froze `openapi.json`; SDK regenerated (`KnightApi.squireState`). 4 new tests in `crates/api/tests/knight.rs` (read OK + squire-scoped fields; unknown 404; non-Squire 404; Squire token 403). `cargo test --workspace` green (48 groups). Read-only — acting on an assumed Squire still uses the Knight's privileged commands (audit intact). Unblocks the assume-Squire UI ([[SQUIRE-T-0055]]).