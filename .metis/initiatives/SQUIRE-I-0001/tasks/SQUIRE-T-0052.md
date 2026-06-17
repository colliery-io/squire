---
id: resilient-server-address-runtime
level: task
title: "Resilient server address: runtime mDNS re-discovery when the stored host goes stale"
short_code: "SQUIRE-T-0052"
created_at: 2026-06-17T22:30:00+00:00
updated_at: 2026-06-17T22:26:13.014591+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

## Acceptance Criteria

## Acceptance Criteria

- [x] While the app is rendering from cache / errored (can't reach the stored host), a loop in
      `…HomeHost` attempts `NsdDiscovery.discover()` every 15s; if a responder is found at a
      **different** host/port, it updates the `Session` (host/port) via `onSessionChanged` →
      `SessionStore.save`, keeping token/household/role, and the adapter reconnects.
- [x] Both apps, no duplicated reconnection logic — the `…HomeHost` `remember(session)` already
      rebuilds the transport when the `Session` state changes; the loop just swaps host/port. **Online
      is unchanged** — the loop only calls discovery when offline (`Ready(fromCache)`/`Error`),
      verified: online → home reached, loop idle.
- [x] Token/session preserved (only host/port change; no re-pair/re-login); a found-but-same address
      is a no-op; discovery finding nothing leaves the app gracefully offline (unchanged behaviour).
- [x] Both apps assemble; `:core`/`:knight-core`/`:sdk` unaffected (app-module-only change). Verified
      on the emulator: online flow intact; airplane-mode → "Offline — showing last saved view", the
      relocate loop runs discovery (null on emulator) **without crashing/thrashing**. The actual "IP
      moved → auto-reconnect" effect needs a real LAN (emulator NSD unreliable, folded into
      [[SQUIRE-T-0050]]).

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

**2026-06-17 — Done (mechanism; live relocate needs hardware).** Both `…HomeHost`s gained a
`discovery: NsdDiscovery` + `onSessionChanged: (Session) -> Unit` param and a relocate loop: while
the UI state is `Ready(fromCache)`/`Error`, call `discovery.discover()` every `RELOCATE_INTERVAL_MS`
(15s); on a different host/port, `onSessionChanged(session.copy(host, port))` → `SessionStore.save`
+ session state update → `remember(session)` rebuilds the adapter at the new address. The token is
untouched (stable signing key, T-0048). MainActivity wires `onSessionChanged = { save; session = it }`.

Verified on the emulator: **online** → player home, loop idle (no discovery churn); **airplane mode**
→ "Offline — showing last saved view", process stays alive, the loop runs discovery (returns null —
no mDNS on the emulator) without crashing or thrashing. The real "server IP moved → app auto-
reconnects" can only be confirmed on a real LAN — rolled into [[SQUIRE-T-0050]]; the DHCP-reservation
note in the runbook remains the simpler guaranteed mitigation. Both apps assemble; cores unaffected.