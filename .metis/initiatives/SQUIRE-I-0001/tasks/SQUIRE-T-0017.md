---
id: api-control-plane-endpoints
level: task
title: "API: control-plane endpoints (register / login / add-member)"
short_code: "SQUIRE-T-0017"
created_at: 2026-06-17T05:13:27.752950+00:00
updated_at: 2026-06-17T05:44:52.075937+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# API: control-plane endpoints (register / login / add-member)

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0003]] (Local API & Trust Boundary)

## Objective

Expose the three control-plane endpoints — register, login, add-member — by delegating to the `Identity` port, and flesh out the minimal dev `Identity` so the LAN-local MVP can bootstrap households and members end to end.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] `POST /register` (unauthenticated): `RegisterHouseholdReq` → dev Identity mints a `HouseholdHandle` + the first **Knight** (seeded via `PutUser` through the single writer) + a token → `RegisterHouseholdResp{household, admin UserId, AuthToken}`. *(MVP single-tenant seeds into the app's one store; `store::Provisioner` multi-tenant provisioning is the hosted model owned by S-0007 — noted.)*
- [x] `POST /login` (unauthenticated): `LoginReq{household, user, secret}` → verify secret → look up role from the store → `LoginResp{AuthToken, Role}`; bad secret → 401.
- [x] `POST /members` (RequireKnight): `AddMemberReq{role, display_name, initial_secret}` → seed the member (any Knight/Squire) via `PutUser` → `AddMemberResp{UserId}`; Squire token → 403.
- [x] Dev Identity shares the store `Arc<Mutex<…>>` with `AppState` (members visible to handlers immediately); in-memory `secrets`/`tokens` maps + a `u128` counter (no rand/clock-now); secrets/tokens opaque on the wire. (S-0007 hardens: real hashing, registry, multi-tenant.)
- [x] `tests/control.rs` (4): register→Knight-token-works; add Squire→login→Squire-token reads `/state`; add-member by Squire→403; wrong secret→401.

## Implementation Notes

### Technical Approach
Handlers stay thin, delegating to the `Identity` port. The dev `Identity` wires `store::Provisioner` for tenant provisioning, maintains a secret→token map, and seeds users through `apply(PutUser)` so all writes pass through the single writer (the Keep). The production identity remains owned by SQUIRE-S-0007.

### Requirements covered
REQ-1.4.1..1.4.3; control-plane decision area; ADR A-0004, A-0002.

### Dependencies
SQUIRE-T-0014 (crate scaffold, app state, Identity port) and `store::Provisioner` (T-0011).

## Status Updates

**2026-06-17 — Completed.** Added serde derives to the 8 identity DTOs (`HouseholdHandle`/`AuthToken`/`RegisterHouseholdReq/Resp`/`LoginReq/Resp`/`AddMemberReq/Resp`). Refactored `AppState.store` to a shared `SharedStore = Arc<Mutex<Store<SystemClock>>>` held by both `AppState` and `DevIdentity`, so identity's `PutUser` seeds are immediately visible to the request handlers (single writer). `DevIdentity` fleshed out: `register` (mint handle + first Knight via PutUser + token), `login` (verify in-memory secret → role from store → token), `add_member` (Knight-only, PutUser, secret recorded); in-memory `secrets`/`tokens` maps + a `Mutex<u128>` counter (deterministic, no rand/clock-now). `crates/api/src/control.rs`: `register`/`login` unauthenticated, `POST /members` `RequireKnight`; `AuthError`→status (Forbidden 403, others 401). T-0015/16/health test setup updated to share the store Arc.

Results: `cargo test -p api` → lib 6 + control 4 + health 7 + knight 7 + squire 5 green; `cargo test -p domain-core` (default) → 70; `cargo test --workspace` green; warning-free. Committed.