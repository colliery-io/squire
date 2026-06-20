---
id: 016-host-the-multi-tenant-deployment
level: adr
title: "Host the multi-tenant deployment on the existing home server, exposed via Cloudflare Tunnel ($0 infra)"
number: 16
short_code: "SQUIRE-A-0016"
created_at: 2026-06-20T23:44:48.475148+00:00
updated_at: 2026-06-20T23:45:30.936689+00:00
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

# ADR-16: Host on the existing home server, exposed via Cloudflare Tunnel ($0 infra)

**Supersedes the _compute_ and _edge/TLS_ decisions of [[SQUIRE-A-0015]].** A-0015's _auth_ (extend
ProdIdentity) and _onboarding_ (provisioned/invite) decisions, and [[SQUIRE-A-0014]] (SQLite-per-tenant
on local disk), remain in force.

## Context

A cost pass on [[SQUIRE-T-0100]] showed AWS EC2-DIY is **~$13/mo steady-state** (EC2 + the IPv4 charge +
EBS + Route 53), and the "free 12 months" framing had eroded (t4g.micro free-tier promo expired Dec
2025; IPv4 is billed even in free tier). The key realization: by [[SQUIRE-A-0014]] (SQLite on local
disk) + [[SQUIRE-A-0015]] (Caddy on the box, **no managed AWS services**), the deployment is just *"one
Linux box with a disk + public reachability"* — **provider-neutral**, no AWS lock-in. And we already run
an **always-up, self-updating `squire-serve` on the home Mac** (launchd service, this session). The
cheapest correct option is therefore to expose *that* box to the internet rather than rent a second one.

## Decision

**Compute = the existing home server.** The multi-tenant `squire-serve` runs on the home Mac (the launchd
service already in place), serving both the LAN household and remote tenants from one process.

**Edge = Cloudflare Tunnel.** `cloudflared` runs on the Mac and makes an **outbound-only** connection to
Cloudflare; Cloudflare terminates TLS at its edge and routes `https://<host>` → the tunnel →
`squire-serve` on loopback. Consequently:
- **No public IP, no inbound ports, no port-forwarding/NAT config** — the tunnel dials out.
- **Caddy is dropped** — Cloudflare provides TLS + the public hostname (free DNS + cert). (Caddy stays
  available for a non-Cloudflare/self-hosted variant.)
- DNS via Cloudflare (free); a custom domain is optional (CF can provide a hostname).

**Cost ≈ $0** (Cloudflare Tunnel free; offsite backups to **Cloudflare R2** free tier, S3-compatible —
reuses the per-tenant export from [[SQUIRE-T-0012]]; only a domain ~$10/yr if wanted).

## Alternatives Analysis

| Option | $/mo | Pros | Cons |
|--------|------|------|------|
| **Home server + Cloudflare Tunnel** (CHOSEN) | **~$0** | Reuses the box we have; **no inbound exposure** (outbound tunnel); free TLS/DNS/DDoS; zero new infra | Availability = home power/internet; we host others' data on a home machine; tunnel is a Cloudflare dependency |
| Hetzner CAX11 (ARM) | ~$4.65 | Cheap dedicated box off home net; 4 GB RAM; ARM = our build | A box to run/patch; still needs edge (Caddy/Tunnel) |
| AWS Lightsail | ~$5 | Cheapest in-AWS; bundle incl. IP | Pricier than Hetzner; in AWS |
| AWS EC2 (A-0015) | ~$13 | — | Most expensive; IPv4 + EBS + R53 surprises |

## Rationale

It's free, it reuses infrastructure we just hardened, and the **outbound-only tunnel is strictly more
secure** than any public-IP box (nothing to port-scan; no inbound firewall surface; Cloudflare absorbs
DDoS/TLS). For **known families** the availability trade-off (home uptime) is acceptable, and because the
stack is provider-neutral we can later `rsync` the per-tenant SQLite files to Hetzner/Lightsail and swap
the tunnel for Caddy with no domain/code changes — this is explicitly **not a one-way door**.

## Consequences

### Positive
- **$0 infra**; no AWS account/billing-alarm surface to manage for the MVP.
- **No public attack surface** — outbound tunnel, no open ports/IP. Best security posture of the options.
- **Tolerates a dynamic home IP and CGNAT** — the tunnel is outbound-only, so the public hostname is a
  CNAME to the tunnel, NOT an A-record at our IP. An ISP IP change just triggers a `cloudflared`
  reconnect (no DNS/DDNS, no propagation gap); works even behind carrier-grade NAT where inbound
  port-forwarding is impossible. (This is a decisive advantage over a port-forward + DDNS setup.)
- Reuses the home launchd service + self-update; only adds `cloudflared`.
- Free managed TLS, DNS, and DDoS from Cloudflare.

### Negative
- **Availability = home power + ISP + the Mac.** No HA; an outage takes all tenants down (acceptable for
  known families; revisit at scale — same trigger family as [[SQUIRE-A-0014]]).
- **Hosting other families' data on a personal machine** — we are the data controller; fine for known
  families, a consideration before any wider audience.
- **Cloudflare dependency** for reachability (free tier; ToS-compatible for this use).
- ISP terms: confirm no prohibition on this kind of low-traffic personal service.

### Neutral
- Caddy work (was [[SQUIRE-T-0101]]) is replaced by `cloudflared` setup; the loopback-bind requirement on
  `squire-serve` ([[SQUIRE-T-0102]]) is unchanged (the tunnel connects to loopback).
- Offsite backup target shifts S3 → **R2** (or keep S3); the per-tenant export logic is identical.

## Review Schedule

Move compute to a rented box (Hetzner first, per cost) when: home availability becomes unacceptable;
tenant count/trust outgrows hosting on a personal machine; or an ISP/Cloudflare constraint bites.
Migration = copy the per-tenant files + repoint the tunnel/DNS (or swap to Caddy) — a deployment move,
not a re-architecture.