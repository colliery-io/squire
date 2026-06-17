---
id: squire-home-advertises-squire-tcp
level: task
title: "squire-home advertises _squire._tcp via mDNS so phone NSD discovery works"
short_code: "SQUIRE-T-0047"
created_at: 2026-06-17T21:00:00.000000+00:00
updated_at: 2026-06-17T21:00:00.000000+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# squire-home advertises _squire._tcp via mDNS so phone NSD discovery works

## Parent Initiative

[[SQUIRE-I-0001]] · Follow-up of [[SQUIRE-T-0046]] · Implements the discovery half of [[SQUIRE-A-0010]] (NFR-6) · Spec [[SQUIRE-S-0003]]

## Objective

Close the one gap left by the pairing work: the phone's `NsdDiscovery` (T-0046) browses `_squire._tcp` to prefill the api host/port, but **nothing advertises that service yet**, so "Discover" always times out to null. Make `squire-home` register an mDNS/DNS-SD service for the LAN api so a paired/​pairing phone can find the computer without a typed IP — even after the computer's address changes (NFR-6). Non-blocking today because the pairing QR already carries host/port; this enables true zero-config discovery.

## Acceptance Criteria

- [ ] `squire-home` registers an mDNS service `_squire._tcp` on the LAN advertising the **api** port (the one phones hit, default 8080 / `API_PORT`), with an instance name (e.g. the household or "Squire") and optionally a TXT record (`household`, `version`). Registration is best-effort: a failure logs a warning and the server still serves (no hard dependency).
- [ ] The advertised host/port resolve to the machine's LAN address (not loopback), so a real device on the same network can reach the api. Verified with a discovery tool (e.g. `dns-sd -B _squire._tcp` / `avahi-browse`) on the host network.
- [ ] The phone `NsdDiscovery.discover()` returns a host:port when the service is up (verified on a real device or a network where NSD works — emulator NSD remains unreliable, so document the check used).
- [ ] `cargo build`/`cargo test --workspace` green; the advertisement is opt-out-able via env (e.g. `SQUIRE_MDNS=off`) for environments where mDNS is unwanted.

## Implementation Notes

### Technical Approach
Add a pure-Rust mDNS responder to `squire-home` — e.g. the `libmdns` crate (`Responder` + `register("_squire._tcp", instance, port, &[txt...])`), or `mdns-sd`. Register after the listeners bind, keep the registration handle alive for the process lifetime, and gate it behind an env flag. The service must advertise the **api** port (phones talk to the LAN api, not the loopback Keep), mirroring the QR's host/port from [[SQUIRE-T-0045]]. No client change needed — `NsdDiscovery` already browses `_squire._tcp.`.

### Dependencies
[[SQUIRE-T-0046]] (the phone `NsdDiscovery` consumer), [[SQUIRE-T-0045]] (the QR advertises the same host/port as a fallback).

### Risk Considerations
mDNS is environment-sensitive — keep it best-effort and opt-out-able; never let a registration failure take down serving. Emulators don't reliably do mDNS, so meaningful verification needs a real device or a host-network browse tool. Picking the right outbound LAN interface/IP (vs loopback/VPN) is the subtle part; `libmdns` typically enumerates interfaces itself. Out of scope: multi-interface/VPN edge cases, Windows firewall prompts.

## Status Updates

*To be added during implementation*
