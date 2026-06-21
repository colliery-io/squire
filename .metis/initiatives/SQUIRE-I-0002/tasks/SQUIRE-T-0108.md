---
id: stable-code-signing-for-squire
level: task
title: "Stable code-signing for squire-serve so macOS Local Network permission survives updates"
short_code: "SQUIRE-T-0108"
created_at: 2026-06-21T20:38:03.291315+00:00
updated_at: 2026-06-21T20:38:03.291315+00:00
parent: SQUIRE-I-0002
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0002
---

# Stable code-signing for squire-serve (macOS Local Network permission)

## Root cause (diagnosed 2026-06-21 — the "nothing submits" saga)

macOS Sequoia's **Local Network privacy** gates any app that uses the local network / Bonjour-mDNS.
`squire-serve` advertises `_squire._tcp` via mDNS **and** serves the LAN, so it needs the **"Local
Network"** TCC grant. That grant is keyed to the binary's **code-signing identity (cdhash / designated
requirement)**.

We **ad-hoc re-sign** the binary on every build (`codesign --force --sign -`), which mints a *new*
identity each time. So every rebuild + reinstall today (TTL fix, gold-path, version-align, the temp
request-logger) made macOS treat `squire-serve` as a **brand-new app** → it **reset and re-prompted**
the Local Network permission. Until re-granted, the server's LAN networking was impaired → **phone
writes silently failed** (the "nothing submits"; phones served cached reads). Operator caught it: the
submit started working the instant macOS prompted to "allow the application to see devices on the
network" and they allowed it. Almost certainly the cause of the earlier "all phones offline" episodes
too — each coincided with a rebuild.

## Fix

Sign `squire-serve` with a **stable identity** instead of ad-hoc, so the signature is constant across
updates and macOS keeps the Local Network grant (no re-prompt on self-update / reinstall):
- A reused **self-signed cert** in the login keychain (cheapest), or a **Developer ID** cert (also
  fixes Gatekeeper). Sign in the build (`angreal apk`/server build) + the dist release artifacts.
- Verify the **designated requirement / cdhash is stable** across two builds (`codesign -dvvv`,
  `csreq`), so TCC sees the same app.
- Confirm the self-update path (`updater.rs`) preserves the signature (the downloaded dist binary must
  carry the same stable signature → no re-prompt after a self-update re-exec).

## Acceptance
- [ ] Two successive builds of `squire-serve` share the same designated requirement (stable cdhash/cert).
- [ ] After a rebuild+reinstall **or** a self-update, the macOS Local Network permission is NOT
  re-prompted and LAN serving keeps working (no submit breakage).
- [ ] Documented for the installer/runbook.

## Notes / context
- **Interim (done):** the running 0.7.9 binary has the permission granted; submissions work. Just stop
  churning the binary and it stays. Source is clean (the temp request-logger was reverted, not committed).
- **The cloud direction sidesteps this entirely:** [[SQUIRE-I-0003]] (home server via Cloudflare Tunnel)
  drops LAN/mDNS, so there's no Local Network permission to manage — another point for it.
- Related to the self-update mechanism ([[SQUIRE-A-0012]]) and the launchd service work.
