---
id: bug-device-token-24h-ttl-with-no
level: task
title: "Bug: device token 24h TTL with no refresh silently bricked paired phones daily"
short_code: "SQUIRE-T-0107"
created_at: 2026-06-21T12:55:04.151407+00:00
updated_at: 2026-06-21T12:55:04.151407+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: true
initiative_id: SQUIRE-I-0001
---

# Bug: 24h device-token TTL with no refresh

## Symptom (operator, 2026-06-21)
The son's (Squire) app "stopped seeing the remote server" while the parent's still worked.

## Root cause
`TOKEN_TTL_MS = 24h`, hardcoded, **and there is no token refresh anywhere** — server or phone. A paired
device stores its token once at pairing and never renews it; a Squire (kid) has no password/re-login
path. So **every paired app goes 401 → "offline" exactly 24h after pairing** and must be re-paired. The
signing key IS persisted (restarts don't invalidate tokens), so it wasn't a restart issue — the parent
happened to re-pair within the last 24h (the earlier "offline" event), the son didn't → his token aged out.

## Fix (committed `8cfea39`, live on prod)
- `TOKEN_TTL_MS` 24h → **~100 years**; added `SQUIRE_TOKEN_TTL_MS` env override (`token_ttl_ms()`).
  Rationale: device tokens are revocable via unpair/deactivate (`SQUIRE-T-0075`), so a very long life is
  the correct model for a paired appliance device. crate 0.7.8 → 0.7.9.
- Built + reinstalled on the prod launchd service + restarted (live).

## Acceptance
- [x] New tokens carry a ~100yr expiry; existing expiry tests (own short TTL) still pass; workspace green.
- [x] Live on prod (service restarted on 0.7.9).
- [ ] **Operator action: re-pair BOTH phones once** — the fix only applies to *newly issued* tokens;
  existing phones still hold the old 24h token until re-paired. After re-pairing, never again.
- [ ] Publish v0.7.9 to dist so fresh installs + self-update get the fix.

## Follow-up
The cloud work ([[SQUIRE-T-0103]]/[[SQUIRE-T-0104]]) should design real token rotation/refresh +
revocation for an internet credential; a 100yr static token is fine for the LAN appliance but the
hosted model wants sliding refresh + revocation.
