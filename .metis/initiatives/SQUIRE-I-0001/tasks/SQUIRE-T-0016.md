---
id: api-knight-quick-action-surface
level: task
title: "API: Knight quick-action surface + HouseholdReview"
short_code: "SQUIRE-T-0016"
created_at: 2026-06-17T05:13:26.850593+00:00
updated_at: 2026-06-17T05:13:26.850593+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# API: Knight quick-action surface + HouseholdReview

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0003]] (Local API & Trust Boundary)

## Objective

Implement the Knight-role privileged quick-action surface plus the cross-Squire `HouseholdReview` read. The Knight token authorizes review and adjustment actions; the actor is always taken from the verified token, never from the client. The Keep remains the only writer.

## Acceptance Criteria

- [ ] `POST /admin/*` (Knight token): dispatch `ReviewClaim{Approve/Reject}`, `ReviewRedemption{Approve/Reject}`, `RedeemItem`(target squire), `AdjustPoints`(target squire, non-empty reason), and mark-done (mint `claim_id` → SubmitClaim + approve). `actor` is filled from the Knight token, never client-supplied.
- [ ] `GET /household-review` (Knight token): returns `HouseholdReview` — `pending_claims` + `pending_requests` across ALL squires (each labeled `squire`) plus per-Squire balances (`SquireSummary`); cacheable.
- [ ] Idempotency: reviews keyed on `claim_id`/`request_id`, redeem/adjust on `command_id`; replay re-returns the prior outcome with no double-apply.
- [ ] `AdjustPoints` with empty reason → 400; engine errors (`OccurrenceTaken`, `AlreadyReviewed`, `Redeem(Blocked)`, `NotFound`) map to sensible HTTP statuses.
- [ ] A Squire-role token on `/admin/*` or `/household-review` → 403; covered by `oneshot` tests.

## Implementation Notes

### Technical Approach
Handlers construct the privileged `Command` with `actor` taken from the token, apply it via the engine, and map engine errors to HTTP statuses. `HouseholdReview` iterates `snapshot.users` (role Squire), computes each Squire's balance, and collects each Squire's pending claims and requests into the labeled aggregate.

### Requirements covered
REQ-1.3.1..1.3.4; FR-ADM1; ADR A-0001, A-0005.

### Dependencies
SQUIRE-T-0014 (crate scaffold, auth/tenant middleware, app state).

## Status Updates

*To be added during implementation*