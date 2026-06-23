# Expose it over the internet

By default the home server is reachable only on your LAN, so a parent's phone has to be on home
Wi‑Fi to sync. To use Squire from anywhere — a Knight approving chores on cellular — expose the
server through a **Cloudflare Tunnel**: a free outbound tunnel that gives you a stable hostname
without opening a port on your router.

> **Status: planned.** Internet mode (cloud-endpoint pairing, mDNS off, Cloudflare Tunnel routing)
> is on the roadmap, not yet shipped. This page describes the intended setup so you know where it's
> headed; some steps may change. See [Delivery & updates](../explanation/delivery-and-updates.md)
> and the project's cloud-deployment initiative.

## Why a tunnel (and not port-forwarding)

- **No inbound ports.** `cloudflared` dials *out* to Cloudflare, so nothing on your router is
  exposed.
- **Stable hostname + TLS.** You get `https://yourname.example.com` terminating at Cloudflare and
  forwarding to the loopback `squire-serve`.
- **~$0.** It runs on Cloudflare's free tier.

## The shape of it

1. **Run the server in internet mode** — bind to loopback only, turn mDNS off, and let Cloudflare be
   the only way in.
2. **Install `cloudflared`** on the home machine and authenticate it to your Cloudflare account.
3. **Create a tunnel** and a DNS hostname that routes to the loopback `KEEP_PORT` / `API_PORT`.
4. **Run `cloudflared` as a service** so the tunnel comes up with the machine.
5. **Point the phones at the cloud endpoint.** Pairing switches from LAN/mDNS discovery to a
   cloud-endpoint QR that encodes the public hostname, so a phone can pair and sync from anywhere.

## See also

- [Back up & restore](backup-restore.md) — pair internet exposure with offsite backups.
- [Run the home server](run-server.md) — running it as a service in the first place.
