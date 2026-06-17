---
id: keep-cross-squire-review-queue
level: task
title: "Keep: cross-Squire review queue, direct redeem & reason-required adjust"
short_code: "SQUIRE-T-0029"
created_at: 2026-06-17T11:10:00.094770+00:00
updated_at: 2026-06-17T11:43:34.296025+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Keep: cross-Squire review queue, direct redeem & reason-required adjust

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0004]] · ADR: [[SQUIRE-A-0008]]

## Objective

The Keep's **batched cross-Squire review queue** and the privileged action subset — approve/reject claims & redemption requests in place, direct redeem, and reason-required adjust — all engine-direct and actor-stamped.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] One batched queue lists ALL pending `CompletionClaimed` + `RedemptionRequested` across every Squire, each labeled with its Squire, approvable/rejectable in place (REQ-1.2.1, NFR-1.1.2). Backed by the cross-Squire `HouseholdReview` (A-0005).
- [x] Approve/reject a claim → `ReviewClaim{decision}` (approve snapshots the quest's current reward → `CompletionApproved`; reject → `CompletionRejected` + optional reason); actor = acting Knight (REQ-1.2.2).
- [x] Approve/reject a redemption → `ReviewRedemption{decision}` (approve → `ItemRedeemed` carrying the `request_id`; reject → `RedemptionRejected` + optional reason); actor stamped (REQ-1.2.3).
- [x] Direct redeem with no prior request → `RedeemItem{item_id}` → `ItemRedeemed` (no `request_id`); actor stamped (REQ-1.3.1).
- [x] Manual adjust → `AdjustPoints{amount, reason}`; the reason is **required** — the UI blocks submit without it (REQ-1.3.2). *(server enforces blank-reason → 400; UI prompts for a reason.)*
- [x] Engine affordability/lock/stock failures (e.g. a balance drained before approval) render as actionable errors, not 500s. *(affordability at approval → 409.)*
- [x] Tests: a mixed queue across 2 squires; approve credits + clears pending; reject with reason; direct redeem; adjust requires a reason; an affordability failure at approval is surfaced.
- [x] `cargo test --workspace` green and warning-free.

## Implementation Notes

### Technical Approach
Read the cross-Squire `HouseholdReview` for the queue; each action builds the privileged `Command` and calls `commit(Some(knight), cmd)`. Mirrors the api Knight surface logic ([[SQUIRE-T-0016]]) but engine-direct (no network call). Redeem/adjust carry a `command_id` for idempotency (A-0001).

### Dependencies
[[SQUIRE-T-0025]]; meaningful queues need quests/items/members ([[SQUIRE-T-0026]]/[[SQUIRE-T-0027]]/[[SQUIRE-T-0028]]). domain-core review/redemption/adjust contract + `HouseholdReview`. Spec REQ-1.2.*, REQ-1.3.*; A-0005, A-0001.

## Status Updates

**2026-06-17 — Done (`630b55e`).** Cross-Squire review queue + redeem + adjust, engine-direct.

- **`keep::review`**: `GET /api/review` (the cross-Squire `HouseholdReview` assembled from one snapshot — squire summaries + pending claims + pending requests, each labelled), `POST /api/review/claim` + `/api/review/redemption` (`ReviewClaim`/`ReviewRedemption`, actor = operating Knight), `POST /api/redeem` (`RedeemItem`, idempotent on `command_id`), `POST /api/adjust` (`AdjustPoints`, blank reason → 400 up front, idempotent). All Knight-only; affordability-at-approval → 409 (`Redeem` error). Mirrors the api Knight surface but commits via `KeepState::commit` (no network).
- **UI**: a review-queue panel — claims/requests with approve/reject (reason via prompt) in place, plus squire balances with an Adjust control.
- **Tests** (`tests/review.rs`, 7): cross-squire queue listing; approve credits + clears; reject (with reason) credits nothing; direct redeem idempotent (6−3=3, not 6−6); adjust reason-required + idempotent; redemption drained-before-review → 409 and stays pending; a Squire token is 403 on every review/redeem/adjust route.

**Verification.** `cargo test --workspace` green & warning-free (40 binaries).