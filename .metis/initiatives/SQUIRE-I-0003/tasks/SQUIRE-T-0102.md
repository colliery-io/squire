---
id: cloud-runtime-mode-for-squire
level: task
title: "Cloud runtime mode for squire-serve: SSM/env config, mDNS off, per-tenant SQLite on EBS, loopback bind"
short_code: "SQUIRE-T-0102"
created_at: 2026-06-20T18:45:14.736770+00:00
updated_at: 2026-06-20T18:45:14.736770+00:00
parent: SQUIRE-I-0003
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0003
---

# Cloud runtime mode for squire-serve

The deployment/config layer that makes `squire-serve` run as a cloud service (vs. the LAN appliance).
Pairs with the concurrency core [[SQUIRE-T-0024]]; keep the LAN mode intact (cloud is a mode, not a fork).

## Scope
- **Config from SSM/env**: data dir (the EBS mount), HMAC signing key, domain/advertised URL, log level.
- **mDNS OFF** in cloud mode (no LAN discovery); **bind loopback** (Caddy is the public edge, [[SQUIRE-T-0101]]).
- **Per-tenant SQLite on the EBS data path** ([[SQUIRE-A-0014]]): one file per tenant under the mounted
  volume; WAL + server-appropriate durability pragmas (vs. appliance defaults).
- Drop/adapt LAN-only bits in cloud mode: apk-sync N/A; pairing advertises the **cloud URL** not a LAN IP
  (feeds [[SQUIRE-T-0104]]).
- Structured logs to stdout (CloudWatch picks them up); `/health` + a readiness signal.

## Acceptance
- [ ] A documented cloud-mode launch (env/SSM) brings squire-serve up: loopback bind, mDNS off, data on
  the EBS path, config from SSM.
- [ ] LAN appliance mode still works unchanged (no regression).
- [ ] Per-tenant SQLite files created under the EBS mount; durability pragmas set + verified.
- [ ] Builds on / coexists with [[SQUIRE-T-0024]] (shared per-tenant store).

## Notes
The concurrency correctness (single-writer-per-tenant under load) is [[SQUIRE-T-0024]]; this task is the
config/runtime wrapper. Together they = "squire-serve is cloud-multi-tenant."
