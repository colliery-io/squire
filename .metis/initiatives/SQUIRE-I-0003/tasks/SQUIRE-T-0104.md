---
id: internet-pairing-phone-cloud
level: task
title: "Internet pairing + phone cloud config: cloud-endpoint QR (drop mDNS), phone targets cloud, pairing-code hardening"
short_code: "SQUIRE-T-0104"
created_at: 2026-06-20T18:45:25.270599+00:00
updated_at: 2026-06-20T18:45:25.270599+00:00
parent: SQUIRE-I-0003
blocked_by: ["SQUIRE-T-0103"]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0003
---

# Internet pairing + phone cloud config

Move pairing from LAN (QR = host:port + mDNS, [[SQUIRE-A-0010]]) to a fixed cloud rendezvous.

## Scope
- **QR shrinks to `{cloud URL, household handle, pairing code}`** — no host:port/IP. The phone stores the
  cloud endpoint and pairs against it (the pairing-code mint/consume flow already exists).
- **Drop mDNS/NSD discovery** in cloud mode; the endpoint is configured, not discovered.
- **Phone cloud config**: a build flavor / runtime setting targeting the cloud endpoint; keep sideload
  distribution (no Play Store). The existing relocate-on-offline logic becomes "reconnect to the cloud
  endpoint."
- **Pairing-code hardening for a public endpoint**: short TTL, sufficient entropy, single-use, rate-limited
  — it's now internet-reachable, not LAN-gated.
- Device token issued through the hardened identity ([[SQUIRE-T-0103]]) and revocable.

## Acceptance
- [ ] Parent generates a QR in the (cloud) Keep; kid's phone scans → pairs over the internet → reaches
  `/state` via the cloud endpoint.
- [ ] No LAN/mDNS dependency remains in cloud mode.
- [ ] Pairing codes are single-use, short-TTL, rate-limited; a stale/replayed code is rejected.
- [ ] Issued device token is revocable (ties to [[SQUIRE-T-0103]]).

## Notes
Blocked by [[SQUIRE-T-0103]] (issues the device token) + needs the runtime/edge ([[SQUIRE-T-0102]]/[[SQUIRE-T-0101]]).
The existing background poll-notifications keep working over the internet — push (FCM/SNS) stays deferred.
