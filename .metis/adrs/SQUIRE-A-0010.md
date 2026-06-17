---
id: 001-device-pairing-per-user-token
level: adr
title: "Device pairing & per-user token provisioning over the LAN"
number: 1
short_code: "SQUIRE-A-0010"
created_at: 2026-06-17T19:33:51.839900+00:00
updated_at: 2026-06-17T19:33:51.839900+00:00
decision_date: 
decision_maker: Dylan Storey
parent: 
archived: false

tags:
  - "#adr"
  - "#phase/draft"


exit_criteria_met: false
initiative_id: NULL
---

# ADR SQUIRE-A-0010: Device pairing & per-user token provisioning over the LAN

> **Status: DRAFT — awaiting decision.** Proposed to unblock [[SQUIRE-T-0042]]. Resolves the three "ADR: TBD" decision areas in [[SQUIRE-S-0006]] (Knight) and the equivalent in [[SQUIRE-S-0005]] (Squire). Extends, does not replace, [[SQUIRE-A-0004]] (per-user accounts + tenant-scoped tokens).

## Context

Both phone apps currently authenticate with **baked demo credentials** hard-coded in `MainActivity` (`household="demo"`, a fixed `user`, `secret="demo"`, `baseUrl="http://10.0.2.2:8080"`). That was fine for the emulator bring-up but violates the product's trust model:

- **NFR-5** requires each device to hold a **per-user, tenant-scoped bearer token** (not a shared secret), so neither another LAN device nor the wrong-role app can act. A-0004 already issues such tokens via Identity (`/login`); what's missing is how a *device* obtains one without typing a password into a child's phone.
- **NFR-6** says the Keep's LAN address is **intermittent and may change** — a hard-coded `baseUrl` is wrong on a real network; the app must discover the host.
- The token must be **stored securely** on-device (not plaintext prefs/logs) and presented on every call (the okhttp interceptor already attaches `Authorization: Bearer <token>` — only the *source* of host+token needs to change).

We need a one-time pairing flow that binds a phone to a household member's account and yields a stored per-user token, plus host discovery, all **LAN-only** (no cloud relay). This spans the Keep (admin UI on the computer), the control-plane API, Identity, and both phone apps.

## Decision

A **QR-based, Keep-mediated pairing** flow with **mDNS/NSD discovery** and **Keystore-backed encrypted storage**:

1. **Mint (on the computer, the Keep).** The parent, already authenticated to the loopback Keep, opens a "Pair a device" screen, picks a household **member** (a specific Knight or Squire), and the Keep calls a new control-plane endpoint to mint a **one-time pairing code**: a high-entropy, single-use, short-TTL (≈5 min) token scoped to `{household, user_id, role}`. The Keep renders a **QR** encoding `{host, port, household_handle, pairing_code}` (and shows the code as text for manual fallback).
2. **Discover + scan (on the phone).** On first run the app browses mDNS/NSD for the service type `_squire._tcp` (advertised by `squire-home`) to learn `{host, port}` — with a manual host-entry fallback if discovery fails. The app scans the QR (or accepts a typed code).
3. **Exchange (phone → control plane).** The app POSTs the pairing code to a new `POST /pair` endpoint. Identity validates it (exists, unexpired, unused), marks it consumed, and returns the member's **per-user tenant-scoped token** (same kind A-0004/`/login` issues) plus the resolved `{household_handle, user_id, role}`.
4. **Store (on the phone).** The app persists `{host, port, household_handle, token, role}` in **Android Keystore-backed `EncryptedSharedPreferences`** (Jetpack Security). Subsequent launches read this; the demo `MainActivity` constants are deleted. A "forget device" action clears it.

The pairing code is consumed server-side from the **append-only log** (mint emits an event; consume checks no prior consume event for that code) — consistent with the project's "dedup/state derived from the log" stance (A-0001/AR-3), no new mutable side table.

## Alternatives Analysis

### Pairing channel
| Option | Pros | Cons | Risk | Cost |
|--------|------|------|------|------|
| **QR from the Keep (chosen)** | Keep is already the trusted admin surface + authenticated parent; no typing on a child's phone; encodes host+port+code in one scan | Needs a QR lib on the phone + a render on the Keep | Low | M |
| Short numeric code typed in | No camera/QR lib | Error-prone typing; still needs host discovery; weaker entropy unless long | Low | S |
| Username/password login on each phone | Familiar | A child typing a secret; shared-secret smell; no device binding | Med | S |

### Host discovery
| Option | Pros | Cons | Risk | Cost |
|--------|------|------|------|------|
| **mDNS/NSD `_squire._tcp` (chosen) + manual fallback** | Survives address changes (NFR-6); zero-config | NSD flaky on some networks/emulator → needs fallback | Med | M |
| Host baked in QR only | Simple | Wrong after the computer's IP changes; re-pair needed | Med | S |
| Manual IP entry only | Trivial | Hostile UX; breaks on DHCP changes | Low | S |

### Token storage
| Option | Pros | Cons | Risk | Cost |
|--------|------|------|------|------|
| **Keystore EncryptedSharedPreferences (chosen)** | Hardware-backed key; standard Jetpack Security | Slightly more setup | Low | S |
| Plain SharedPreferences | Trivial | Token readable on a rooted/backed-up device | High | S |

## Rationale

The Keep is already the authenticated, on-computer admin surface and the single writer — making it the **minting authority** keeps the trust root where it belongs and avoids any unauthenticated "claim a token" path. QR carries host+port+code in one scan so the child's phone needs no typing and no prior network config. mDNS directly answers NFR-6 (changing address), with a manual fallback for flaky networks. Keystore storage is the standard, low-cost way to honour "secure storage." Deriving code-consumption from the event log keeps us consistent with A-0001/AR-3 (no processed-commands side table). This **extends** A-0004 (same token shape, same role gate) rather than changing the token model.

## Consequences

### Positive
- NFR-5 satisfied: real per-user, role-scoped, device-bound tokens; the baked `demo`/`demo` creds are removed.
- NFR-6 satisfied: the app finds and follows the Keep without a typed IP.
- Trust root stays on the authenticated Keep; no new unauthenticated surface beyond the one-time, short-TTL, single-use `/pair` exchange.
- Reuses the existing token + interceptor plumbing; only the *source* of host+token changes.

### Negative
- New dependencies on the phone: a QR scanner (ML Kit or ZXing), `NsdManager`, Jetpack Security. Larger than the auto-refresh change.
- mDNS/NSD is environment-sensitive (notably on emulators) — the manual-host fallback is mandatory, not optional.
- New server surface (`/pair` + the Keep "pair a device" screen) and Identity changes (mint/consume one-time codes) — a cross-component effort; should decompose into sub-tasks.

### Neutral
- Demo/dev can keep a "skip pairing, use demo creds" debug path behind a build flag so `squire-home` + emulator flows stay one-tap.
- iOS Keychain equivalent is deferred until an iOS client exists.

## Decomposition (once decided)
1. **Server**: Identity mint/consume one-time pairing codes (log-derived) + control-plane `POST /pair`; tests. Re-freeze `openapi.json`, regen SDK.
2. **Keep**: "Pair a device" screen (pick member → mint → render QR + text code).
3. **Phone (shared)**: NSD discovery + manual fallback, QR scan, `/pair` exchange, Keystore `SessionStore`; replace the baked `MainActivity` constants in both apps; "forget device".
4. **Demo path**: a debug-only bypass so emulator/`squire-home` flows stay frictionless.

## Open questions for the decision
- QR library preference: **ML Kit** (bundled, heavier) vs **ZXing** (lighter, manual)?
- Keep a **debug demo-creds bypass**, or pair even in dev?
- Pairing-code TTL/length (proposed: 5 min, single-use, ≥128-bit)?
