---
id: edge-caddy-auto-tls-reverse-proxy
level: task
title: "Edge: Caddy auto-TLS reverse proxy in front of squire-serve"
short_code: "SQUIRE-T-0101"
created_at: 2026-06-20T18:45:10.351729+00:00
updated_at: 2026-06-20T18:45:10.351729+00:00
parent: SQUIRE-I-0003
blocked_by: ["SQUIRE-T-0100"]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0003
---

# Edge: Caddy auto-TLS reverse proxy

Per [[SQUIRE-A-0015]] — Caddy terminates TLS and reverse-proxies to `squire-serve` on loopback. On top
of [[SQUIRE-T-0100]].

## Scope
- Install Caddy on the box; systemd-managed.
- Caddyfile: auto-HTTPS (Let's Encrypt) for the domain → `reverse_proxy 127.0.0.1:<api_port>`.
- HTTP→HTTPS redirect; sensible timeouts; pass through `X-Household` + auth headers untouched.
- Cert auto-renewal (Caddy handles it) + a renewal-failure alarm.
- `squire-serve` binds **loopback only** (the public bind is Caddy) — coordinate with [[SQUIRE-T-0102]].

## Acceptance
- [ ] `https://<domain>/health` returns 200 with a valid LE cert; HTTP redirects to HTTPS.
- [ ] A real `/state` request flows phone → Caddy(443) → squire-serve(loopback) with auth + `X-Household`
  intact.
- [ ] Cert renews automatically; a renewal failure raises an alarm.
- [ ] squire-serve is NOT reachable except via Caddy (no public bind on the api port).

## Notes
Blocked by [[SQUIRE-T-0100]]. Pairs with [[SQUIRE-T-0102]] (loopback bind).
