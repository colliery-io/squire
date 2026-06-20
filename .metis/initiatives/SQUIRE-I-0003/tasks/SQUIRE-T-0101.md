---
id: edge-caddy-auto-tls-reverse-proxy
level: task
title: "Cloudflare Tunnel routing config: hostname → loopback squire-serve, run as a service"
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

# Cloudflare Tunnel routing config

Per [[SQUIRE-A-0016]] — the tunnel *is* the edge; Cloudflare terminates TLS. *(Rescoped from "Caddy
auto-TLS"; Caddy is dropped — kept only for a non-Cloudflare/self-hosted variant.)* On top of
[[SQUIRE-T-0100]].

## Scope
- `cloudflared` **ingress config**: `<hostname>` → `http://127.0.0.1:<api_port>` (loopback); a catch-all
  404 for everything else (only the api is exposed).
- Pass through `X-Household` + auth headers untouched; sensible timeouts; websockets N/A.
- Run `cloudflared` under launchd (start-on-boot, restart-on-fail) — mirror the squire-serve service.
- Verify Cloudflare TLS/proxy settings (Full/strict not needed since origin is loopback-local to the
  tunnel; confirm no double-encoding of headers).
- Health-monitor the tunnel (alert if it disconnects).

## Acceptance
- [ ] A real `/state` request flows phone → Cloudflare → tunnel → `squire-serve` loopback with auth +
  `X-Household` intact.
- [ ] Only the api hostname is routed; no other local port is reachable through the tunnel.
- [ ] `cloudflared` auto-starts on boot + restarts on crash; a disconnect raises an alert.

## Notes
Blocked by [[SQUIRE-T-0100]]. Pairs with [[SQUIRE-T-0102]] (squire-serve binds loopback).
