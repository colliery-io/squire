---
id: 014-cloud-data-store-sqlite-per-tenant
level: adr
title: "Cloud data store: SQLite-per-tenant on EBS for the free-tier MVP, Postgres as the scale path"
number: 14
short_code: "SQUIRE-A-0014"
created_at: 2026-06-20T18:36:01.557458+00:00
updated_at: 2026-06-20T18:37:22.297651+00:00
decision_date: 
decision_maker: Operator (Dylan)
parent: 
archived: false

tags:
  - "#adr"
  - "#phase/decided"


exit_criteria_met: false
initiative_id: NULL
---

# ADR-14: Cloud data store — SQLite-per-tenant on EBS (MVP), Postgres as the scale path

## Context

[[SQUIRE-I-0003]] hosts today's product multi-tenant in the cloud for **known families**, on **AWS
free-tier**, single box, no HA required for the MVP. The store already supports **two backends**
([[SQUIRE-A-0003]]) with **per-tenant isolation** ([[SQUIRE-A-0002]] — file-per-tenant on SQLite,
schema-per-tenant on Postgres), and a **conformance suite proves SQLite/Postgres parity**. Crucially,
[[SQUIRE-A-0002]] decided the *isolation model* and that *both* backends are first-class — it did **not**
mandate Postgres for hosting. So the open question this ADR closes is narrow: **which backend backs the
cloud MVP**, because it cascades into backups, HA, scaling, ops, and cost.

Relevant facts already in the codebase:
- `store` has both backends; `SQUIRE-T-0012` shipped **per-tenant single-file export/import**.
- The api routes by `X-Household`; the hosted multi-tenant server task is `SQUIRE-T-0024`
  (single-writer-per-tenant).
- RDS db.t4g.micro is free for **12 months only** (~$15/mo after); EC2/EBS t4g.micro free 12mo, EBS
  cheap thereafter (~$0.08/GB-mo).

## Decision

For the free-tier MVP, the hosted server uses **SQLite, one database file per tenant, on an attached
EBS volume** — the same per-tenant-file model the LAN appliance already uses, lifted to the cloud box.
**Postgres (RDS) is the documented scale path**, adopted when a trigger below fires. No code fork:
both backends already exist and pass conformance, so this is a *deployment* choice, swappable later
without touching the domain.

Backup/durability for the SQLite-on-EBS MVP:
- **Per-tenant export to S3** on a schedule (reuses `SQUIRE-T-0012`) — versioned, per-file, making
  per-tenant restore/export/delete trivial.
- **EBS volume snapshots** on a schedule (point-in-time of the whole box).
- Encryption at rest via an encrypted EBS volume.

## Alternatives Analysis

| Option | Pros | Cons | Risk | Cost |
|--------|------|------|------|------|
| **A. SQLite-per-tenant on EBS** (CHOSEN, MVP) | Free indefinitely; in-process (no network hop, fast); **file-per-tenant = strong isolation + trivial export/backup/delete** (T-0012); **single-writer-per-tenant falls out for free** (one write lock per file, WAL concurrent reads) — matches T-0024; no pooling, minimal ops | Local to one box → no horizontal app scale; durability = EBS (single-AZ) + our backups, not managed PITR/HA; restore is a manual drill | Low–Med | Free (12mo), then trivial |
| **B. RDS Postgres, schema-per-tenant** | Managed backups + PITR + optional multi-AZ; decoupled from compute → multi-instance app later; schema-per-tenant already supported | **12-mo free cliff (~$15/mo)**; network latency + **connection pooling required**; more ops surface; isolation is logical (schema) not physical (file) | Med | Free 12mo, then ~$15/mo+ |
| **C. Postgres on the EC2 box (self-managed)** | Free; PG semantics without RDS cost | We run/patch/back up PG ourselves on a 1GB box; worst of both (PG ops + no managed durability) | Med–High | Free, high toil |

## Rationale

The MVP is explicitly **single-box, known-families, no-HA, stay-on-free-tier**. Against that brief,
SQLite-per-tenant wins on every axis that matters now: it's **free past 12 months** (no RDS cliff), it
**reuses the exact model + export/import tooling we already ship**, and **file-per-tenant gives the
strongest isolation and the simplest per-tenant backup/restore/delete** — the riskiest part of
multi-tenancy. It also makes `SQUIRE-T-0024`'s single-writer-per-tenant requirement *fall out of the
filesystem* (one SQLite write lock per tenant file) rather than needing app-level coordination. RDS's
advantages — managed PITR/HA and compute/storage decoupling — only pay off once we need multi-instance
or can't tolerate a restore drill, neither of which is true at MVP. Because both backends already pass
conformance, choosing SQLite now costs us nothing later: the move to Postgres is a deployment swap, not
a rewrite.

## Consequences

### Positive
- Free indefinitely; no 12-month cost cliff.
- Per-tenant file = physical isolation + one-file export/backup/restore/delete (T-0012 reuse).
- Single-writer-per-tenant for free (per-file lock); no connection pool to operate.
- Zero new persistence code — a deployment/config choice over existing backends.

### Negative
- **Durability is ours to own**: EBS is single-AZ; real protection comes from our scheduled S3 exports
  + EBS snapshots. **Backup *and a tested restore drill* are mandatory, not optional.**
- **No horizontal app scale**: data is local to the box → one `squire-serve` instance. Fine until
  tenant load outgrows one t4g.micro.
- Restore/migration is a manual procedure we must document and rehearse.

### Neutral
- WAL mode + per-tenant files should be set explicitly; confirm fsync/durability pragmas for a server
  (vs. an appliance) context.
- The Postgres path stays warm via the conformance suite — keep it green so the swap remains a
  non-event.

## Review Schedule

Revisit (→ migrate to RDS/Postgres per Option B) when **any** trigger fires:
- Need a **second app instance** (HA or load) — local files block this.
- Tenant count / write volume saturates one t4g.micro.
- We require **managed PITR / multi-AZ** durability we're unwilling to hand-roll.
- A restore drill proves the SQLite+S3 backup story too slow/risky for the data we hold.

Until a trigger fires, this is the cloud data store. Migration is a deployment swap (both backends are
already supported + conformance-tested), not a re-architecture.