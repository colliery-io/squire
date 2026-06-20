---
id: hosted-multi-tenant-cloud
level: initiative
title: "Hosted multi-tenant deployment (home server via Cloudflare Tunnel, ~$0)"
short_code: "SQUIRE-I-0003"
created_at: 2026-06-20T18:10:44.430485+00:00
updated_at: 2026-06-20T18:49:12.052393+00:00
parent: SQUIRE-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/decompose"


exit_criteria_met: false
estimated_complexity: M
initiative_id: hosted-multi-tenant-cloud
---

# Hosted multi-tenant deployment (home server via Cloudflare Tunnel, ~$0)

> **Hosting decided ([[SQUIRE-A-0016]]):** run on the **existing home Mac** (the always-up launchd
> service), exposed via **Cloudflare Tunnel** (outbound-only; free TLS/DNS/DDoS) — **~$0 infra**, no
> public IP or inbound ports. This superseded the earlier AWS-EC2/Caddy framing after a cost pass
> (~$13/mo → $0). The stack is provider-neutral, so a later move to Hetzner/Lightsail is a deployment
> swap, not a rewrite.

## Context

Today Squire ships as a **self-hosted LAN appliance**: one `squire-serve` per household, phones paired
by QR + mDNS over the local network, the Keep on loopback. This initiative hosts **today's product**
(no feature changes) **multi-tenant over the internet** so multiple known families run on one shared
`squire-serve` — keeping the LAN path intact as a variant.

**Explicitly NOT in scope:** Play Store / mass-market launch, open public signup at scale, COPPA-as-a-
launch-gate, Cognito-at-50k-MAU. This is "run what we have, for known families, in the cloud."

The multi-tenant *design* is largely already decided — this initiative is mostly **provisioning,
internet-reachability, and deploy/ops**, not new domain work:
- [[SQUIRE-A-0002]] tenancy: schema-per-tenant isolation, dual SQLite/Postgres — **decided**.
- [[SQUIRE-A-0004]] identity/roles/registration, tenant-scoped tokens — **decided**.
- [[SQUIRE-A-0010]] device pairing + per-user token provisioning — **decided** (LAN today).
- `SQUIRE-S-0007` Identity/Tenancy/Registration spec; `store` already has the PG `Provisioner` +
  SQLite/PG conformance parity; `api` already routes by `X-Household`.
- **`SQUIRE-T-0024`** (backlog) "Concurrent hosted multi-tenant API: per-tenant shared store (single
  writer per tenant)" — the core server-side multi-tenant task already scoped; pull into this work.

## Goals & Non-Goals

**Goals**
- One cloud-hosted `squire-serve` serving **many tenants** (households), isolated per [[SQUIRE-A-0002]].
- Reachable over the **internet with TLS** via Cloudflare Tunnel; phones pair to a **fixed cloud
  hostname** (no LAN/mDNS).
- **Accounts**: parent = account, owns a tenant; provisioned/invite onboarding (not open signup).
- **~$0 infra** (home Mac + Cloudflare Tunnel); no public IP/inbound ports.
- **LAN mode preserved** — internet hosting is a runtime mode, not a replacement.

**Non-Goals**
- Play Store / app-store distribution (sideload via the existing dist repo stays).
- Mass-market signup, marketing site, billing/subscriptions.
- HA / multi-region (single-box MVP; scale path noted).
- New product features (this is lift-and-host of today's behavior).

## Architecture ($0 home-hosted topology — [[SQUIRE-A-0016]])

```
Phone (sideloaded APK) ──HTTPS──┐
Parent browser (the Keep) ──────┴─> Cloudflare edge (TLS/DNS/DDoS)
                                         │  Tunnel (outbound-only from the Mac — no inbound ports/IP)
                                         ▼
                              squire-serve (multi-tenant, X-Household) on the HOME MAC (launchd)
                                         │
                                         ▼
                              per-tenant SQLite on local disk ([[SQUIRE-A-0014]])
   offsite backup: per-tenant export ([[SQUIRE-T-0012]]) → Cloudflare R2 (free, S3-compatible)
```
`cloudflared` on the Mac dials out to Cloudflare; CF terminates TLS and routes the hostname → the tunnel
→ `squire-serve` on loopback. No EC2/EBS/ALB/Caddy/Route53/SSM. The api's `X-Household` routing makes one
process serve all tenants; per-tenant single-writer is [[SQUIRE-T-0024]].

## Detailed Design — the two work lists

### A. To PROVISION (AWS resources)
1. **Account hygiene**: root MFA; admin role; least-priv deploy role; **billing + free-tier usage
   alarms** (hard requirement — cost guardrail).
2. **Network**: VPC (default ok), security groups — **443 in only**, DB private to the app SG, shell
   via **SSM Session Manager** (no open 22).
3. **Compute**: **EC2 t4g.micro** (Graviton, free 12mo) running `squire-serve` + Caddy; instance IAM
   role for SSM/S3/CloudWatch.
4. **Data**: per the data decision — RDS Postgres db.t4g.micro (free 12mo) **or** SQLite-on-EBS (free
   indefinitely); encryption at rest either way.
5. **DNS/TLS**: Route 53 hosted zone + domain; TLS via Caddy/Let's Encrypt.
6. **Secrets**: SSM Parameter Store (free) — token-signing key (HMAC), DB creds.
7. **Storage**: S3 — dist/APK mirror, event-log/DB backups, static assets.
8. **Email**: SES — account verification + password reset.
9. **Observability**: CloudWatch log group + alarms (CPU, mem, DB conns, 5xx, disk, free-tier).
10. *(Deferred/optional)* SNS→FCM push — the phone's existing poll-based notifications still work over
    the internet, so push is not MVP-blocking.

### B. To IMPLEMENT (code)
1. **Cloud runtime mode** for `squire-serve`: chosen data backend + **connection pooling** (PG needs
   r2d2/deadpool; SQLite was per-process), **mDNS off**, public bind behind Caddy, config from SSM/env.
   (Realizes `SQUIRE-T-0024`.)
2. **Accounts (parent = account)**: login/session, **email verification + password reset**, token
   issuance — extend `ProdIdentity` ([[SQUIRE-A-0004]]). Onboarding is **provisioned/invite**, not open.
3. **Tenant provisioning on account creation**: call the existing `Provisioner`; map account→handle.
4. **Internet pairing**: QR encodes **cloud URL + household handle + pairing code** (drop LAN IP/mDNS);
   phone stores the cloud endpoint. Pairing-code mechanics already exist ([[SQUIRE-A-0010]]) — review
   **TTL/entropy** for a public endpoint.
5. **Phone app cloud config**: a build flavor / runtime config targeting the cloud endpoint; keep
   sideload distribution.
6. **Tenant-isolation hardening + tests**: prove no cross-tenant leakage under the shared process
   (the strongest *security* requirement here).
7. **Data export + hard-delete per tenant** (good practice even without COPPA-gate).
8. **PG migrations on deploy** (already PG-clean) + **deploy path** — reuse the self-update mechanism
   ([[SQUIRE-A-0012]]) or SSM run-command; `/health` already exists.
9. **Keep account UI**: login/session/account-management (today it assumes one bootstrapped admin).

## Decisions (all RESOLVED)

| Decision | Resolution | ADR |
|---|---|---|
| **Data store** ✅ | SQLite-per-tenant on local disk; Postgres = scale path | [[SQUIRE-A-0014]] |
| **Compute** ✅ | **Home Mac** (existing launchd service) — *was EC2* | [[SQUIRE-A-0016]] (supersedes A-0015 #1) |
| **Edge / TLS** ✅ | **Cloudflare Tunnel** (outbound-only, free TLS/DNS) — *was Caddy* | [[SQUIRE-A-0016]] (supersedes A-0015 #2) |
| **Auth** ✅ | Extend `ProdIdentity` (add email verify + password reset) | [[SQUIRE-A-0015]] |
| **Onboarding** ✅ | Provisioned / invite (not open signup) | [[SQUIRE-A-0015]] |

All load-bearing decisions are locked → ready to **decompose into tasks** (Phase 1 MVP).

## Risks / gotchas
- **Tenant-isolation correctness** is now a *security* boundary on shared infra — must be tested hard.
- **RDS free-tier cliff** at 12 months (data decision).
- **Single EC2 = SPOF**; no HA on free-tier (acceptable MVP; note scale-out).
- **Pairing trust over the internet** — code TTL/entropy + endpoint auth matter more than on LAN.
- **Availability = home power + ISP + the Mac** ([[SQUIRE-A-0016]]) — no HA; an outage takes all tenants
  down. Acceptable for known families; the trigger to move to a rented box.
- **Hosting other families' data on a personal machine** — we're the data controller; fine for known
  families, a consideration before any wider audience.
- **Backup/restore drills** — must be exercised, not assumed (offsite to R2, since the box is at home).
- **Cloudflare + ISP dependency/ToS** — confirm low-traffic personal hosting is permitted.

## Implementation Plan

### Phase 1 — Cloud MVP (decomposed)
| Task | What | Deps |
|---|---|---|
| [[SQUIRE-T-0100]] | Expose the home server: Cloudflare Tunnel (`cloudflared`) + DNS hostname + offsite backup target | — |
| [[SQUIRE-T-0101]] | Cloudflare Tunnel config: hostname → loopback `squire-serve`, run as a service *(rescoped from Caddy)* | T-0100 |
| [[SQUIRE-T-0024]] | Hosted multi-tenant concurrency: per-tenant shared store, single-writer-per-tenant (pulled from backlog) | — |
| [[SQUIRE-T-0102]] | Cloud runtime mode: config, mDNS off, per-tenant SQLite on local disk, loopback bind | — |
| [[SQUIRE-T-0103]] | Accounts + provisioned onboarding: ProdIdentity verify/reset, token revocation, invite tenant | T-0102 |
| [[SQUIRE-T-0104]] | Internet pairing + phone cloud config: cloud-endpoint QR (drop mDNS), code hardening | T-0103 |
| [[SQUIRE-T-0105]] | **Tenant-isolation hardening + cross-tenant leakage tests** (security gate) | T-0024 |
| [[SQUIRE-T-0106]] | Backups + restore drill: per-tenant export → Cloudflare R2, rehearsed restore | T-0102 |

**Critical path:** the server/auth track dominates — T-0024→T-0102→T-0103→T-0104; the tunnel
(T-0100→T-0101) is a quick parallel setup. T-0105 gates launch; T-0106 before any real data.
*Much smaller infra surface than the AWS framing* — `cloudflared` + R2, not a provisioned box.

### Phase 2 — Hardening (not yet decomposed)
Rate limiting, deeper observability/alarms, deploy automation, per-tenant export/**delete**.

### Phase 3 — Scale path (not yet decomposed)
RDS/Postgres migration (per [[SQUIRE-A-0014]] triggers), second box / multi-instance.

## Status

**Decompose.** All decisions locked ([[SQUIRE-A-0014]], [[SQUIRE-A-0015]]); Phase 1 decomposed into
T-0024 + T-0100..T-0106. Ready to start — recommended first moves: **T-0100** (stand up the box) and
**T-0024** (the concurrency core) in parallel. **T-0105** is the highest-stakes item (security) — gate
the MVP on it.