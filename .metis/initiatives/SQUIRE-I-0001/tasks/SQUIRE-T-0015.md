---
id: api-squire-endpoints-per-squire
level: task
title: "API: Squire endpoints + per-Squire StateView assembly"
short_code: "SQUIRE-T-0015"
created_at: 2026-06-17T05:13:25.400963+00:00
updated_at: 2026-06-17T05:13:25.400963+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# API: Squire endpoints + per-Squire StateView assembly

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0003]] (Local API & Trust Boundary)

## Objective

Implement the three Squire-role endpoints and assemble the per-Squire `StateView` from `Projections` plus the domain-core view helpers (`quest_status`, `reward_view`, `streak_view`). A Squire can read only their own scoped state and propose their own activity; the Keep remains the only writer.

## Acceptance Criteria

- [ ] `GET /state` (Squire token): returns the authenticated Squire's `StateView` (carries `squire`) built from a Snapshot scoped to that Squire — `quests_today` (`QuestCard` + `quest_status`, including `TakenByOther`), `balance`, `streaks` (`streak_view`), `rewards` (`reward_view`: affordable/lock/`last_redeemed`, `LockReason` NeedsAchievement|OutOfStock), `my_claims` (`ClaimStatus`), `my_requests` (`RedemptionStatus`); never another Squire's state.
- [ ] `POST /claims`: parse `SubmitClaimReq` (omits squire), fill `squire` from the token, run `Engine::handle(SubmitClaim)` → apply → `SubmitClaimResp{claim_id, state}` (Pending, or Approved{points} on auto-approve).
- [ ] `POST /redemption-requests`: parse `RequestRedemptionReq`, fill squire, run `RequestRedemption` → apply → `RequestRedemptionResp{request_id, Pending}`; no affordability check at request time.
- [ ] Idempotency: re-POST of the same `claim_id`/`request_id` produces no duplicate event and re-returns the current state.
- [ ] Squire-role only; covered by `oneshot` tests.

## Implementation Notes

### Technical Approach
The API crate owns the `StateView` DTO assembly; domain-core provides the projections and the `quest_status`/`reward_view`/`streak_view` helpers. `quests_today` is derived from the Squire's scheduled quests for `clock.today()`, each annotated with its per-quest status. `my_claims` and `my_requests` are read directly from the scoped snapshot. Handlers fill `squire` from the verified `Principal` rather than trusting any client-supplied identity.

### Requirements covered
REQ-1.1.1..1.1.5, REQ-1.2.1; FR-API1, FR-API2, FR-API3, FR-PL1; ADR A-0005, A-0006.

### Dependencies
SQUIRE-T-0014 (crate scaffold, auth/tenant middleware, app state).

## Status Updates

*To be added during implementation*