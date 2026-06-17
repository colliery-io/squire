---
id: api-control-plane-endpoints
level: task
title: "API: control-plane endpoints (register / login / add-member)"
short_code: "SQUIRE-T-0017"
created_at: 2026-06-17T05:13:27.752950+00:00
updated_at: 2026-06-17T05:37:50.012016+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


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

- [ ] `POST /register` (unauthenticated): `RegisterHouseholdReq` → Identity creates the household, provisions an isolated schema via `store::Provisioner`, and seeds the first Knight → `RegisterHouseholdResp{household, admin UserId, AuthToken}`.
- [ ] `POST /login`: `LoginReq{household, user, secret}` → resolve tenant → Identity verifies the member → `LoginResp{AuthToken, Role}`.
- [ ] Knight-only add-member: `AddMemberReq{role, display_name, initial_secret}` → Identity creates the member in the caller's tenant → `AddMemberResp{UserId}`; Knight token required, Squire token → 403.
- [ ] Dev Identity implements these minimally: provision the tenant store, seed the first Knight, hold an in-memory secret→token map, and seed members via `PutUser` through the store's single writer. Secrets and tokens stay opaque on the wire; hashes are never returned. (S-0007 hardens with real hashing, registry, and multi-tenant.)
- [ ] Tests: register → login → token works; add-member by a Knight succeeds; add-member by a Squire → 403.

## Implementation Notes

### Technical Approach
Handlers stay thin, delegating to the `Identity` port. The dev `Identity` wires `store::Provisioner` for tenant provisioning, maintains a secret→token map, and seeds users through `apply(PutUser)` so all writes pass through the single writer (the Keep). The production identity remains owned by SQUIRE-S-0007.

### Requirements covered
REQ-1.4.1..1.4.3; control-plane decision area; ADR A-0004, A-0002.

### Dependencies
SQUIRE-T-0014 (crate scaffold, app state, Identity port) and `store::Provisioner` (T-0011).

## Status Updates

*To be added during implementation*