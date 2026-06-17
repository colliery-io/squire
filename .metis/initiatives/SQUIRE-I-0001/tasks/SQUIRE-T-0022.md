---
id: identity-production-identity-impl
level: task
title: "Identity: production Identity impl (register/login/add-member/verify/authorize)"
short_code: "SQUIRE-T-0022"
created_at: 2026-06-17T09:52:24.150966+00:00
updated_at: 2026-06-17T10:10:18.595392+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Identity: production Identity impl (register/login/add-member/verify/authorize)

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0007]] (Identity, Tenancy & Registration)

## Objective

In `crates/identity`, implement the production `Identity` impl that ties together hashing (T-0020), tokens (T-0020), the tenant registry (T-0021), and the `Store` writer. Users and their hashed credentials live IN the tenant schema; there is no global user directory.

## Acceptance Criteria

## Acceptance Criteria

- [ ] `register(req)`: provision a tenant, seed the first **Knight** with a hashed credential via `PutUser` through `Repository::apply(by=None, …)` (system seed), mint a token → `RegisterHouseholdResp{household, admin UserId, token}`.
- [ ] `add_member(caller, req)`: Knight-only; create the member (Knight|Squire) via `PutUser` with `apply(by=Some(caller.user))` (audited); store the hashed `initial_secret` → `AddMemberResp{user}`. No assumed counts or fixed shape.
- [ ] `login(req)`: resolve tenant → verify the member's secret against the stored hash → look up `Role` from the tenant `users` → issue token → `LoginResp{token, role}`.
- [ ] `verify(handle, token)`: verify token (tamper/expiry) → resolve `(tenant, user, role)`; invalid is rejected; the token's tenant must match the presented handle.
- [ ] Authorize by `(tenant, user, role)`: Knight = full surface; Squire = read + propose for itself; credentials stored only as hashes; users in-tenant; no global directory.
- [ ] Audit: identity `Change`s flow through `apply(by, …)` — `by`=caller for add-member, `None` for the register seed; the users-table audit columns answer "who added member X".
- [ ] Tests: register→login (hashed cred); add-member by Knight (audited) / by Squire → Forbidden; wrong secret → reject; token tamper/expiry → reject; cross-tenant token → reject.

## Implementation Notes

### Technical Approach
Store each member's hashed secret in-tenant by **adding an in-tenant `credentials` table (user_id → hash) to the store migrations** (coordinate with S-0002's migration set; this is the in-tenant credential storage REQ-1.6 needs). Register and add-member write it; login reads and verifies it. Compose hashing (T-0020) + tokens (T-0020) + registry (T-0021) + the `Store` writer into a single `Identity` implementation.

### Requirements covered
REQ-1.1, REQ-1.2, REQ-1.3, REQ-1.4, REQ-1.5, REQ-1.9, REQ-1.10, REQ-1.11, REQ-1.12; NFR-2.2, NFR-2.3, NFR-2.5; ADR A-0004, A-0005, A-0007.

### Dependencies
SQUIRE-T-0019, SQUIRE-T-0020, SQUIRE-T-0021.

## Status Updates

*To be added during implementation*