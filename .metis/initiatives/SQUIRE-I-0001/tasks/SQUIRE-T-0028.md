---
id: keep-member-administration-add
level: task
title: "Keep: member administration (add Knights/Squires, mint tokens, audit)"
short_code: "SQUIRE-T-0028"
created_at: 2026-06-17T11:09:58.675568+00:00
updated_at: 2026-06-17T11:39:17.870518+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Keep: member administration (add Knights/Squires, mint tokens, audit)

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0004]] · Identity: [[SQUIRE-S-0007]]

## Objective

Keep **member administration**: add Knights & Squires and mint their tenant-scoped tokens via the Identity component, and list members with audit ("who added / last changed X"). All member writes go through the one single-writer store the Keep holds.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] Add a member (Knight or Squire) with display name + initial secret via the `identity` add-member path; the new member's tenant-scoped token can be minted/shown so a phone can be provisioned (A-0004).
- [x] List members with role + last-editor audit ("who added / last changed", from the `users` audit columns) (REQ-1.4.2, A-0007).
- [x] Member writes go through the **single writer** (the same `SharedStore` the Keep holds); the operation is Knight-only (the operator must be a Knight).
- [x] De/reactivate a member via `SetUserActive` (archive-not-delete).
- [x] Tests: add a Knight + a Squire; audit `created_by` = acting Knight; token mint → login round-trip; a non-Knight operator is refused.
- [x] `cargo test --workspace` green and warning-free.

## Implementation Notes

### Technical Approach
Reuse `identity` (`Identity::add_member`, token issuance) over the SAME `SharedStore` the Keep holds, so all member writes serialize on the one writer (consistent with [[SQUIRE-T-0023]]'s single-writer wiring). Read members via `Repository::snapshot` + `store::user_audit`.

### Dependencies
[[SQUIRE-T-0025]]; identity (`add_member`, `TokenSigner`). Spec member-authoring (REQ-1.4.2); A-0004, A-0007.

## Status Updates

**2026-06-17 — Done (`93a1f19`).** Member administration, single-writer + audited.

- **`KeepState::apply_changes(by, changes)`**: a direct single-writer apply for member writes (`PutUser`/`SetUserActive`) — these are NOT engine commands (mirrors how `identity` writes members); authoring/claim/redemption still go through `commit`.
- **`keep::members`**: `GET /api/members` (role + active + last-editor audit via `store::user_audit`), `POST /api/members` (add via `identity.add_member` audited to the Knight, then mint a pairing token via `login`), `POST /api/members/{id}/active` (`SetUserActive`, archive-not-delete; missing → 404). All Knight-only via the `Operator` extractor (and `add_member` re-checks defense-in-depth).
- **UI**: members panel — add form (shows the minted pairing token) + activate/deactivate.
- **Tests** (`tests/members.rs`, 4): add Knight + Squire mint working tokens and audit `created_by` = admin; 401 unauth; a Squire's minted token is **403** on member admin; deactivate/reactivate + 404 missing.

**Verification.** `cargo test --workspace` green & warning-free (39 binaries).