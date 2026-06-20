---
id: 015-cloud-platform-choices-ec2-caddy
level: adr
title: "Cloud platform choices: EC2+Caddy compute/edge, ProdIdentity auth, provisioned onboarding"
number: 15
short_code: "SQUIRE-A-0015"
created_at: 2026-06-20T18:41:15.772116+00:00
updated_at: 2026-06-20T18:41:56.792023+00:00
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

# ADR-15: Cloud platform choices — compute/edge, auth, onboarding

> **⚠ Partially superseded by [[SQUIRE-A-0016]] (2026-06-20):** decisions **#1 Compute (EC2)** and
> **#2 Edge/TLS (Caddy)** are replaced by *home server + Cloudflare Tunnel ($0)* after a cost pass.
> Decisions **#3 Auth (extend ProdIdentity)** and **#4 Onboarding (provisioned/invite)** REMAIN IN FORCE.

## Context

[[SQUIRE-I-0003]] (hosted multi-tenant, AWS free-tier, known families) had four secondary decisions
left open after the cascading data-store call ([[SQUIRE-A-0014]]: SQLite-per-tenant on EBS). This ADR
closes them together — none cascades the way data-store did, and the operator selected the
recommended lean on each. Recorded here so the platform shape is durable, not buried in the initiative.

## Decision

1. **Compute — EC2 t4g.micro (free-tier).** One Graviton t4g.micro runs `squire-serve` + Caddy. Free
   for 12 months; cheap thereafter. (Not Lightsail.)
2. **Edge / TLS — Caddy on the box.** Caddy terminates TLS with automatic Let's Encrypt certs and
   reverse-proxies to `squire-serve` on loopback. No ALB/CloudFront (cost + moving parts) for the MVP.
3. **Auth — extend `ProdIdentity`.** Add email **verification + password reset** and internet-grade
   session/token handling to the existing Argon2id/HMAC identity ([[SQUIRE-A-0004]]). (Not Cognito —
   no mass-scale/MFA-offload need for known families; keeps the auth surface in our control + tested.)
4. **Onboarding — provisioned / invite.** We create a household (tenant) for a known family and issue
   an invite; the parent sets their password and pairs kids by QR. (Not open self-service signup —
   smaller abuse surface, matches the "known families" scope.)

## Alternatives Analysis

| Decision | Chosen | Rejected | Why |
|---|---|---|---|
| Compute | **EC2 t4g.micro free-tier** | Lightsail (~$5/mo, bundled IP/transfer, simpler) | Free 12mo + we already drive EC2/SSM; revisit if EC2 ops toil outweighs the $5 |
| Edge/TLS | **Caddy auto-TLS on the box** | ALB+ACM / CloudFront | Free, one config file, auto-renew; ALB is ~$16/mo, CloudFront adds caching we don't need |
| Auth | **Extend ProdIdentity** | AWS Cognito (50k MAU free) | No mass scale; keep auth in-repo + conformance-tested; avoid a vendor dependency for a handful of families |
| Onboarding | **Provisioned/invite** | Open self-service signup | Known families only → smaller surface, no email-bomb/abuse vector, simpler to build |

## Rationale

Every choice optimizes for the same brief as A-0014: **smallest secure surface, stays-on-free-tier,
reuses what we have, one box.** EC2+Caddy is the cheapest internet-facing TLS path with the fewest
moving parts. Extending `ProdIdentity` keeps the security-sensitive auth code in one tested place
rather than splitting trust between our HMAC tokens and a managed pool — and we only need verify/reset,
not Cognito's scale features. Provisioned onboarding removes an entire class of public-endpoint abuse
(signup spam, email bombing, throwaway tenants) that buys us nothing for known families. None of these
is a one-way door: Lightsail, a managed edge, Cognito, or open signup can each be adopted later without
touching the domain.

## Consequences

### Positive
- Cheapest viable internet-facing posture; nearly all free-tier.
- Auth stays one tested module; pairing/token revocation reuse ([[SQUIRE-A-0010]], `SQUIRE-T-0075`).
- No public signup endpoint to harden/abuse-protect at MVP.

### Negative
- **We own edge + host ops** (Caddy config, cert renewal monitoring, EC2 patching via SSM) — no managed
  ALB/Cognito to lean on.
- Single box: edge + app + data co-located (SPOF) — acceptable per [[SQUIRE-A-0014]], noted for scale.
- Provisioned onboarding = a manual/admin step per family until/unless we build self-serve.

### Neutral
- The internet credential is now real (vs. a LAN bearer): enforce token revocation + rotation (extends
  `SQUIRE-T-0075` unpair/deactivate) — a requirement for the auth/pairing tasks, not a blocker here.

## Review Schedule

Revisit a specific choice if its trigger fires: EC2 ops toil > Lightsail's ~$5 (→ Lightsail); need
managed WAF/edge caching (→ CloudFront/ALB); need MFA or >~thousands of accounts (→ reconsider Cognito);
demand for self-serve signup (→ build open onboarding + abuse controls). Until then these stand.