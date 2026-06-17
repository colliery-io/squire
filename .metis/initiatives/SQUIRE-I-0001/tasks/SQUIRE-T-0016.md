---
id: api-knight-quick-action-surface
level: task
title: "API: Knight quick-action surface + HouseholdReview"
short_code: "SQUIRE-T-0016"
created_at: 2026-06-17T05:13:26.850593+00:00
updated_at: 2026-06-17T05:37:19.381022+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# API: Knight quick-action surface + HouseholdReview

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0003]] (Local API & Trust Boundary)

## Objective

Implement the Knight-role privileged quick-action surface plus the cross-Squire `HouseholdReview` read. The Knight token authorizes review and adjustment actions; the actor is always taken from the verified token, never from the client. The Keep remains the only writer.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] `POST /admin/{review-claim,review-redemption,redeem,adjust,mark-done}` (RequireKnight): each dispatches the matching `Command` with `actor` filled from the Knight token — never in the body; mark-done = SubmitClaim then approve. (5 handlers in `knight.rs`.)
- [x] `GET /household-review` (RequireKnight): `HouseholdReview` — `pending_claims`+`pending_requests` across all squires (labeled `squire`) + per-Squire balances (`SquireSummary`), from one snapshot.
- [x] Idempotency: redeem/adjust replay (same `command_id`) → engine empty change set → handler re-returns `{ok}` (no double-spend, tested); a genuine second review → engine `AlreadyReviewed` → 409.
- [x] `/admin/adjust` empty/whitespace reason → 400 (up front); shared `domain_status` maps engine errors (OccurrenceTaken/NotAssigned/BadCommandForActor→403, …NotFound→404, AlreadyReviewed/AlreadyClaimedToday/Redeem→409, NotASquire/InvalidDefinition/Inactive→400).
- [x] Squire token on every `/admin/*` + `/household-review` → 403; missing token → 401; `tests/knight.rs` (7) via `oneshot`.

## Implementation Notes

### Technical Approach
Handlers construct the privileged `Command` with `actor` taken from the token, apply it via the engine, and map engine errors to HTTP statuses. `HouseholdReview` iterates `snapshot.users` (role Squire), computes each Squire's balance, and collects each Squire's pending claims and requests into the labeled aggregate.

### Requirements covered
REQ-1.3.1..1.3.4; FR-ADM1; ADR A-0001, A-0005.

### Dependencies
SQUIRE-T-0014 (crate scaffold, auth/tenant middleware, app state).

## Status Updates

**2026-06-17 — Completed.** `crates/api/src/knight.rs`: 5 quick-action handlers (`/admin/review-claim`, `/admin/review-redemption`, `/admin/redeem`, `/admin/adjust`, `/admin/mark-done`) under `RequireKnight`, each building the privileged `Command` with `actor = principal.user` (never from the body) and running through the shared `handle_command` (by=None); `/admin/mark-done` = SubmitClaim then approve. `GET /household-review` assembles `HouseholdReview` (per-Squire `SquireSummary` balances + cross-Squire pending claims/requests) from one snapshot. Added serde derives to `HouseholdReview`/`SquireSummary`/`PendingClaim`/`PendingRequest` + `CommandId`. Shared `domain_status` error mapping refined (Redeem→409). Idempotent redeem/adjust replays (empty change set) treated as success; genuine double-review → `AlreadyReviewed`→409.

Tests `tests/knight.rs` (7): household-review labeling + balances, approve credits + review-removed, double-approve→409, adjust empty-reason→400 + idempotent replay, redeem idempotent (no double-spend), and the trust-boundary sweep (Squire token → 403 on every admin route + review; missing → 401). Results: `cargo test -p api` → lib 3 + health 7 + knight 7 + squire 5 green; `cargo test -p domain-core` (default) → 70; `cargo test --workspace` green; 0 build warnings. Committed.