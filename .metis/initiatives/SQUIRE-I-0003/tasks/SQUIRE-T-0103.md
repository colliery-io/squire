---
id: accounts-provisioned-onboarding
level: task
title: "Accounts + provisioned onboarding: ProdIdentity email verify/reset, token revocation, invite-based tenant creation"
short_code: "SQUIRE-T-0103"
created_at: 2026-06-20T18:45:20.403595+00:00
updated_at: 2026-06-20T18:45:20.403595+00:00
parent: SQUIRE-I-0003
blocked_by: ["SQUIRE-T-0102"]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0003
---

# Accounts + provisioned onboarding

Per [[SQUIRE-A-0015]] (extend `ProdIdentity`, provisioned/invite onboarding) + [[SQUIRE-A-0004]]. Parent =
account = tenant owner; kids have no account ([[squire-cloud-direction]]).

## Scope
- **ProdIdentity hardening for the internet**: email **verification** + **password reset** (SES,
  [[SQUIRE-T-0100]] secrets); session/token issuance fit for a public endpoint.
- **Token revocation + rotation** — the device token graduates from a LAN bearer to a real credential;
  extend unpair/deactivate ([[SQUIRE-T-0075]]) so a lost/abused token can be killed.
- **Provisioned/invite onboarding**: an admin action creates a household (tenant) via the existing
  `Provisioner` and issues an invite; the parent sets their password (no open self-service signup).
- **Tenant provisioning on account creation**: map account → household handle; immediately routable
  (coordinate the provision↔first-request race with [[SQUIRE-T-0024]]).
- **Keep account UI**: login/session + basic account management (today the Keep assumes one bootstrapped
  admin).
- Rate-limit auth endpoints (login/reset) at the edge/app.

## Acceptance
- [ ] Invite → parent sets password → email verified → can log in and operate their (only) household.
- [ ] Password reset works end-to-end via SES.
- [ ] A device/session token can be revoked; revoked tokens are rejected.
- [ ] New tenant is provisioned + routable without racing a concurrent first request.
- [ ] No open signup endpoint exists (provisioned only).

## Notes
Blocked by [[SQUIRE-T-0102]] (runtime) and pairs with [[SQUIRE-T-0024]] (tenant routing). Feeds
[[SQUIRE-T-0104]] (pairing issues device tokens through this).
