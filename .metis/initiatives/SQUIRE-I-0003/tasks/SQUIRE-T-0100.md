---
id: provision-the-cloud-box-ec2-t4g
level: task
title: "Expose the home server via Cloudflare Tunnel: cloudflared + DNS hostname + offsite backup target"
short_code: "SQUIRE-T-0100"
created_at: 2026-06-20T18:45:03.724063+00:00
updated_at: 2026-06-20T18:45:03.724063+00:00
parent: SQUIRE-I-0003
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0003
---

# Expose the home server via Cloudflare Tunnel

Per [[SQUIRE-A-0016]] — make the existing home `squire-serve` reachable over the internet at ~$0, with
no public IP or inbound ports. *(Rescoped from "provision an AWS box" after the cost pass.)*

## Scope
- **Cloudflare account + zone/domain** (a domain on Cloudflare, ~$10/yr, or a CF-provided hostname).
- **`cloudflared`** installed on the home Mac, run as a managed service (launchd) alongside `squire-serve`
  — outbound-only connection to Cloudflare.
- **Tunnel route**: `https://<hostname>` → the tunnel → `squire-serve` on **loopback** (the api port).
  (Server loopback-bind is [[SQUIRE-T-0102]].)
- **DNS** record (CF) for the hostname → the tunnel.
- **Offsite backup target**: a **Cloudflare R2** bucket (free tier, S3-compatible) for per-tenant exports
  (consumed by [[SQUIRE-T-0106]]) — the box is at home, so backups MUST be offsite.
- **Secrets/config** stay local (file / macOS keychain) — no SSM.
- Confirm **Cloudflare + ISP ToS** allow this low-traffic personal hosting.

## Acceptance
- [ ] `https://<hostname>/health` returns 200 via the tunnel (valid CF TLS), with **no inbound ports
  opened** on the home network / no public IP exposed.
- [ ] `cloudflared` runs as a service that survives reboot + restarts on failure (like the squire-serve
  launchd unit).
- [ ] An R2 bucket exists + credentials are available to the backup job.
- [ ] Tunnel only reaches `squire-serve` loopback (no other local services exposed).

## Notes
Tiny infra surface vs the old AWS plan — `cloudflared` + R2. Blocks [[SQUIRE-T-0101]] (tunnel routing
config) and feeds [[SQUIRE-T-0106]] (R2 backups). Provider-neutral: a later move to Hetzner/Lightsail
re-points DNS/tunnel + copies the files ([[SQUIRE-A-0016]]).
