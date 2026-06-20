---
id: cloud-runtime-mode-for-squire
level: task
title: "Internet runtime mode for squire-serve: env/local config, mDNS off, per-tenant SQLite on local disk, loopback bind"
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
- **Config from env/local file** (macOS keychain or a config file — no SSM): data dir, HMAC signing key,
  advertised hostname, log level.
- **mDNS OFF** in internet mode (no LAN discovery); **bind loopback** (Cloudflare Tunnel is the public
  edge, [[SQUIRE-T-0101]]).
- **Per-tenant SQLite on the local data dir** ([[SQUIRE-A-0014]]): one file per tenant on the home Mac's
  disk; WAL + server-appropriate durability pragmas (vs. appliance defaults).
- Drop/adapt LAN-only bits: apk-sync N/A; pairing advertises the **cloud hostname** not a LAN IP
  (feeds [[SQUIRE-T-0104]]).
- Structured logs to stdout/file (the launchd log we already capture); `/health` + a readiness signal.

## Acceptance
- [ ] A documented internet-mode launch brings squire-serve up: loopback bind, mDNS off, config from
  env/local file.
- [ ] LAN appliance mode still works unchanged (no regression).
- [ ] Per-tenant SQLite files created under the local data dir; durability pragmas set + verified.
- [ ] Builds on / coexists with [[SQUIRE-T-0024]] (shared per-tenant store).

## Notes
The concurrency correctness (single-writer-per-tenant under load) is [[SQUIRE-T-0024]]; this task is the
config/runtime wrapper. Together they = "squire-serve is cloud-multi-tenant."
