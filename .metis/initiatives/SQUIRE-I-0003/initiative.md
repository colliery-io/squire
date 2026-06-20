---
id: hosted-multi-tenant-cloud
level: initiative
title: "Hosted multi-tenant cloud deployment (AWS free-tier)"
short_code: "SQUIRE-I-0003"
created_at: 2026-06-20T18:10:44.430485+00:00
updated_at: 2026-06-20T18:10:44.430485+00:00
parent: SQUIRE-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: M
initiative_id: hosted-multi-tenant-cloud
---

# Hosted multi-tenant cloud deployment (AWS free-tier)

## Context

Today Squire ships as a **self-hosted LAN appliance**: one `squire-serve` per household, phones paired
by QR + mDNS over the local network, the Keep on loopback. This initiative hosts **today's product**
(no feature changes) **multi-tenant in the cloud** so multiple known families run on shared AWS infra,
reachable over the internet — keeping the self-hosted path intact as a deployment variant.

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
- Reachable over the **internet with TLS**; phones pair to a **cloud endpoint** (no LAN/mDNS).
- **Accounts**: parent = account, owns a tenant; provisioned/invite onboarding (not open signup).
- Stays **on AWS free-tier** for the MVP; cost-guardrailed.
- **Self-hosted LAN mode preserved** — cloud is a runtime mode, not a replacement.

**Non-Goals**
- Play Store / app-store distribution (sideload via the existing dist repo stays).
- Mass-market signup, marketing site, billing/subscriptions.
- HA / multi-region (single-box MVP; scale path noted).
- New product features (this is lift-and-host of today's behavior).

## Architecture (free-tier topology)

```
Phone (sideloaded APK) ──HTTPS──┐
                                ├─> [Caddy auto-TLS]  ── squire-serve (multi-tenant, X-Household)
Parent browser (the Keep) ──────┘   on EC2 t4g.micro          │
                                                               ▼
                                          data store: SQLite-on-EBS  *or*  RDS Postgres  (DECISION)
  S3 (dist mirror + backups) · SSM (secrets) · CloudWatch (logs/alarms) · SES (account email)
```
One always-on EC2 + (optional) one RDS both fit in 750h/mo. Caddy on the box = free Let's Encrypt TLS,
avoiding ALB/CloudFront cost. The api's existing `X-Household` routing makes one process serve all
tenants; per-tenant single-writer is `SQUIRE-T-0024`.

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

## Alternatives Considered (the load-bearing decisions — OPEN)

| Decision | Option A | Option B | Lean |
|---|---|---|---|
| **Data store** ✅ | SQLite-on-EBS — free forever, code-supported | RDS Postgres — managed, 12-mo free cliff | **DECIDED [[SQUIRE-A-0014]]: SQLite-per-tenant on EBS (MVP), Postgres scale path** |
| **Onboarding** | Provisioned/invite (we create households for known families) | Open self-service signup | Provisioned (matches "known families", smaller surface) |
| **Auth** | Extend `ProdIdentity` (Argon2id/HMAC, add verify/reset) | Managed (Cognito) | Extend ProdIdentity (no mass-scale need) |
| **Compute** | EC2 t4g.micro free-tier | Lightsail (~$5/mo, simpler) | EC2 free-tier to start |
| **Edge/TLS** | Caddy on the box (free LE) | ALB/CloudFront (cost/complexity) | Caddy |

## Risks / gotchas
- **Tenant-isolation correctness** is now a *security* boundary on shared infra — must be tested hard.
- **RDS free-tier cliff** at 12 months (data decision).
- **Single EC2 = SPOF**; no HA on free-tier (acceptable MVP; note scale-out).
- **Pairing trust over the internet** — code TTL/entropy + endpoint auth matter more than on LAN.
- **t4g.micro = 1GB RAM** — watch under many tenants + PG pool.
- **Backup/restore drills** — must be exercised, not assumed.
- Cost creep past free-tier — billing alarms are mandatory, not optional.

## Implementation Plan (phasing — NOT yet decomposed into tasks)
1. **Cloud MVP**: 1 EC2 + Caddy + data + accounts + internet pairing → one real household live in cloud.
2. **Hardening**: isolation tests, backups/restore drill, rate limiting, observability/alarms, deploy
   automation, export/delete.
3. **Multi-household onboarding + scale path** (RDS/PG, second box when needed).

## Status

**Discovery / design.** Plan captured. **Human-in-the-loop**: resolve the open decisions (data store
first — it cascades) before decomposing into tasks. Pull `SQUIRE-T-0024` under this initiative when
decomposing.
