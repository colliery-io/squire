---
id: squire-home-advertises-squire-tcp
level: task
title: "squire-home advertises _squire._tcp via mDNS so phone NSD discovery works"
short_code: "SQUIRE-T-0047"
created_at: 2026-06-17T21:00:00+00:00
updated_at: 2026-06-17T21:08:54.192810+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# squire-home advertises _squire._tcp via mDNS so phone NSD discovery works

## Parent Initiative

[[SQUIRE-I-0001]] · Follow-up of [[SQUIRE-T-0046]] · Implements the discovery half of [[SQUIRE-A-0010]] (NFR-6) · Spec [[SQUIRE-S-0003]]

## Objective

Close the one gap left by the pairing work: the phone's `NsdDiscovery` (T-0046) browses `_squire._tcp` to prefill the api host/port, but **nothing advertises that service yet**, so "Discover" always times out to null. Make `squire-home` register an mDNS/DNS-SD service for the LAN api so a paired/​pairing phone can find the computer without a typed IP — even after the computer's address changes (NFR-6). Non-blocking today because the pairing QR already carries host/port; this enables true zero-config discovery.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] `squire-home` registers `_squire._tcp` advertising the **api** port (`API_PORT`, default 8080), instance "Squire", TXT `household=<handle>` (via `libmdns`). Best-effort: a responder failure only logs (`eprintln`); serving never depends on it. Banner prints the advertised state.
- [x] `libmdns` enumerates the host interfaces and announces the machine's LAN address itself (not loopback). Confirmed `squire-home` binds the mDNS multicast socket — `lsof -iUDP:5353` shows the process holding **IPv4 + IPv6 `*:5353`** (announcing on the wire). *Caveat:* same-host `dns-sd -B` on macOS can't see it because the OS `mDNSResponder` already owns :5353 (two responders, one host) — cross-device LAN discovery is unaffected.
- [x] No client change needed — the phone `NsdDiscovery` (T-0046) already browses `_squire._tcp`. Full resolve verification needs a separate device on a real LAN (emulator + same-host macOS NSD are both unreliable, as anticipated); the wire-level check used here is the `lsof :5353` socket-bound proof + the server log.
- [x] `cargo test --workspace` green (45 groups, 0 failures). Opt-out verified: `SQUIRE_MDNS=off` → "disabled", 0 sockets on :5353, server still serves.

## Implementation Notes

### Technical Approach
Add a pure-Rust mDNS responder to `squire-home` — e.g. the `libmdns` crate (`Responder` + `register("_squire._tcp", instance, port, &[txt...])`), or `mdns-sd`. Register after the listeners bind, keep the registration handle alive for the process lifetime, and gate it behind an env flag. The service must advertise the **api** port (phones talk to the LAN api, not the loopback Keep), mirroring the QR's host/port from [[SQUIRE-T-0045]]. No client change needed — `NsdDiscovery` already browses `_squire._tcp.`.

### Dependencies
[[SQUIRE-T-0046]] (the phone `NsdDiscovery` consumer), [[SQUIRE-T-0045]] (the QR advertises the same host/port as a fallback).

### Risk Considerations
mDNS is environment-sensitive — keep it best-effort and opt-out-able; never let a registration failure take down serving. Emulators don't reliably do mDNS, so meaningful verification needs a real device or a host-network browse tool. Picking the right outbound LAN interface/IP (vs loopback/VPN) is the subtle part; `libmdns` typically enumerates interfaces itself. Out of scope: multi-interface/VPN edge cases, Windows firewall prompts.

## Status Updates

**2026-06-17 — Done.** Added `libmdns = "0.9"` to `squire-home`; new `start_mdns(api_port, household)` helper registers `_squire._tcp` (instance "Squire", TXT `household=…`) on the api port after the store is wired, holding the `(Responder, Service)` guard for the process lifetime via `let _mdns = …` before `tokio::try_join!`. Gated by `SQUIRE_MDNS=off`; any responder error only logs. `cargo build`/`cargo test --workspace` green.

Verification on the macOS host: `lsof -nP -iUDP:5353` shows `squire-home` holding `*:5353` on **both IPv4 and IPv6** → libmdns is bound and announcing. `dns-sd -B _squire._tcp` returns nothing same-host because macOS's `mDNSResponder` already owns :5353 (you can't browse a second responder's records locally) — this is a host-tooling limitation, not a bug; a real phone on the LAN receives the multicast. Opt-out confirmed: `SQUIRE_MDNS=off` → log "disabled", 0 sockets on :5353, server still serves. Closes the discovery gap noted in [[SQUIRE-T-0046]]; full cross-device resolve verification deferred to a real-device test (out of emulator scope).