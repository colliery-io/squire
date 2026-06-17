---
id: resilient-server-address-runtime
level: task
title: "Resilient server address: runtime mDNS re-discovery when the stored host goes stale"
short_code: "SQUIRE-T-0052"
created_at: 2026-06-17T22:30:00.000000+00:00
updated_at: 2026-06-17T22:30:00.000000+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Resilient server address: runtime mDNS re-discovery when the stored host goes stale

## Parent Initiative

[[SQUIRE-I-0001]] · Uses [[SQUIRE-T-0047]] (mDNS advert) + [[SQUIRE-T-0046]] (`NsdDiscovery`) · NFR-6 (changing LAN address)

## Objective

A paired phone pins the computer's LAN IP captured at pairing time (in the `Session`); if that IP
changes (DHCP lease, router reboot), the app can't reach the server and is stuck on its cached view
until the user Forgets + re-pairs. Make it **self-heal**: when the stored host becomes unreachable,
re-discover the server by name over mDNS (`_squire._tcp`, already advertised by `squire-serve`),
update the stored `Session` host, and reconnect — no re-pairing. The auth token is unaffected by an
IP change (stable signing key, T-0048), so only the address needs refreshing.

## Acceptance Criteria

- [ ] When the app is rendering from cache / can't reach the stored host, it attempts an mDNS
      re-discovery (`NsdDiscovery`, best-effort, at most once per offline stretch). If a responder is
      found at a **different** host/port, update the `Session` (host/port) in `SessionStore`, keeping
      the token/household/role, and reconnect (the next refresh hits the new address).
- [ ] Implemented for **both** apps with no duplication of the reconnection logic (the session-gated
      `…HomeHost` already rebuilds the transport when the `Session` state changes — drive it from
      there). Normal (reachable) operation is unchanged — no extra discovery churn when online.
- [ ] Token/session preserved across a host change (no re-pair, no re-login); a found-but-same host
      is a no-op; discovery finding nothing leaves the app gracefully offline (current behaviour).
- [ ] `:core`/`:knight-core`/`:sdk` tests green; both apps assemble. Mechanism verified to not
      regress the online flow on the emulator; **the actual "IP moved → auto-reconnect" needs a real
      LAN** (emulator NSD is unreliable, like [[SQUIRE-T-0050]]) — documented.

## Implementation Notes

### Technical Approach
The `…HomeHost` composables hold the `Session` as state and `remember(session)` the adapter, so
updating the session swaps the transport automatically. Add a relocation step: when the UI state is
`Ready(fromCache=true)`/`Error` (unreachable), launch `NsdDiscovery.discover()`; if it returns a
host/port differing from the session, `sessionStore.save(session.copy(host=…, port=…))` and update
the session state → recomposition rebuilds the adapter against the new address. Guard so it runs at
most once per offline period (a flag reset when online). No server change (the `_squire._tcp` advert
from T-0047 is the responder). Keep `:core`/`:knight-core` pure — this lives in `:app`/`:knight-app`.

### Dependencies
[[SQUIRE-T-0047]] (server advertises `_squire._tcp`), [[SQUIRE-T-0046]] (`NsdDiscovery` + `SessionStore`), [[SQUIRE-T-0048]] (stable signing key so the token survives the address change).

### Risk Considerations
Emulator mDNS is unreliable, so the auto-relocate's *effect* is only fully verifiable on real
hardware (rolled into [[SQUIRE-T-0050]]); the mechanism + graceful fallback are verifiable now.
Don't thrash discovery while online or loop on it while offline (once-per-stretch guard). A simpler
real-world mitigation remains a **DHCP reservation** for the server (documented in the runbook); this
task is the zero-config self-heal on top. Out of scope: storing a `.local` hostname instead of an IP
(an alternative approach), multi-server selection.

## Status Updates

*To be added during implementation*
