---
id: api-squire-endpoints-per-squire
level: task
title: "API: Squire endpoints + per-Squire StateView assembly"
short_code: "SQUIRE-T-0015"
created_at: 2026-06-17T05:13:25.400963+00:00
updated_at: 2026-06-17T05:30:49.285649+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# API: Squire endpoints + per-Squire StateView assembly

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0003]] (Local API & Trust Boundary)

## Objective

Implement the three Squire-role endpoints and assemble the per-Squire `StateView` from `Projections` plus the domain-core view helpers (`quest_status`, `reward_view`, `streak_view`). A Squire can read only their own scoped state and propose their own activity; the Keep remains the only writer.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] `GET /state` (RequireSquire): the authenticated Squire's `StateView` (carries `squire`) from one scoped snapshot — `quests_today` (`QuestCard`+`quest_status`), `balance`, `streaks` (`streak_view`), `rewards` (`reward_view`: affordable/lock/`last_redeemed`), `my_claims`, `my_requests`; never another Squire's state (`squire` from token, never a query param).
- [x] `POST /claims`: `SubmitClaimReq` (omits squire) → fill `squire` from token → `SubmitClaim` via shared `handle_command` (snapshot→engine→apply, `by=None`) → `SubmitClaimResp{claim_id, state}` (Pending / Approved{points} on auto-approve).
- [x] `POST /redemption-requests`: `RequestRedemptionReq` → `RequestRedemption` → `RequestRedemptionResp{request_id, Pending}`; no affordability check.
- [x] Idempotency: replayed `claim_id`/`request_id` → engine emits no changes; handler re-reads + returns current state (no error, no duplicate event).
- [x] RequireSquire (Knight token → 403, no token → 401); `tests/squire.rs` (5) via `oneshot` over a real seeded SQLite tenant.
- [x] **Serde:** added an optional `serde` feature to `domain-core` (`dep:serde`, off by default → core stays pure; `cargo test -p domain-core` still 70 without serde) with cfg-gated derives on the API DTOs + ids/enums; `api` enables it.

## Implementation Notes

### Technical Approach
The API crate owns the `StateView` DTO assembly; domain-core provides the projections and the `quest_status`/`reward_view`/`streak_view` helpers. `quests_today` is derived from the Squire's scheduled quests for `clock.today()`, each annotated with its per-quest status. `my_claims` and `my_requests` are read directly from the scoped snapshot. Handlers fill `squire` from the verified `Principal` rather than trusting any client-supplied identity.

### Requirements covered
REQ-1.1.1..1.1.5, REQ-1.2.1; FR-API1, FR-API2, FR-API3, FR-PL1; ADR A-0005, A-0006.

### Dependencies
SQUIRE-T-0014 (crate scaffold, auth/tenant middleware, app state).

## Status Updates

**2026-06-17 — Completed.** `crates/api/src/squire.rs`: `get_state` (assembles the per-Squire `StateView` purely over one snapshot — quests_today via `quest_status`, rewards via `reward_view`, streaks via `streak_view`, balance via `Proj::balance` clamped ≥0, my_claims/my_requests read from the scoped log), `submit_claim`, `request_redemption`, plus a reusable `handle_command(state, by, cmd)` (lock store → snapshot → `engine.handle` → `apply`) that T-0016 reuses, and `DomainError`→4xx mapping. `squire` is always taken from the verified `Principal`; submissions apply with `by=None`. Wired `GET /state`, `POST /claims`, `POST /redemption-requests` in `lib.rs`.

**Serde:** `domain-core` gained an optional `serde` feature (`serde` optional dep + `serde = ["dep:serde"]`; `#[cfg_attr(feature="serde", derive(Serialize,Deserialize))]` on the API DTOs and the ids/`Role`/`Category`/`Date`/`Timestamp` they contain). `dep:` syntax means no implicit feature — default `cargo build/test -p domain-core` pulls in zero serde and compiles the derives away (still 70 tests). The `api` crate opts in via `features=["serde"]`.

Results: `cargo test -p api` → lib 3 + health 7 + squire 5 green; `cargo test -p domain-core` (default) → 70, no serde; `cargo test --workspace` → green; warning-free. Committed.