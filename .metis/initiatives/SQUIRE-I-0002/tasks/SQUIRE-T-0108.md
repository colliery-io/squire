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
  - "#phase/completed"


exit_criteria_met: true
initiative_id: SQUIRE-I-0002
---

# Stable code-signing for squire-serve (macOS Local Network permission)

## ⚠️ Causation UNCERTAIN — re-evaluated 2026-06-21 (operator skeptical)
The "ad-hoc re-signing churns the grant" mechanism below is an **inference, not confirmed**. Counter-
evidence: the binary was rebuilt/restarted several times during the saga but the operator was prompted
**once**, and the grant **held across subsequent restarts** — consistent with a **one-time first grant
macOS surfaced late**, not a per-update churn. Also, for a **bare CLI binary** (no bundle id) macOS may
key the Local-Network grant by **path**, not cdhash → re-signing wouldn't re-prompt at all.

**Empirical test in flight:** the **v0.7.10** release (cut via GitHub 2026-06-21) self-updates prod to a
**CI-built binary with a different signature** than the running local one. If the kid's submit then
works with **no new prompt** → no churn → **this task is unnecessary** (close as won't-fix). If it
re-prompts → churn confirmed → proceed with a stable signing identity (operator chose **self-signed
local cert**, $0).

**Alternative that moots this entirely:** run squire-serve in a **container** (Docker Desktop now / the
cloud box per [[SQUIRE-A-0016]] + [[SQUIRE-I-0003]]) — no TCC-gated native binary at all. Parked as a
cloud-direction option, not an urgent fix, pending the v0.7.10 churn-test result.

## Original root-cause hypothesis (diagnosed 2026-06-21 — the "nothing submits" saga)

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

## RESOLVED 2026-06-21 — won't-fix (churn disproven)
The v0.7.10 release self-updated prod to a CI-built binary and a **phone write landed with NO macOS
prompt** (operator confirmed: no prompt; coin grant saved, event seq 68). So re-signing/updating does
**not** churn the Local Network grant — the earlier prompt was a one-time first grant macOS surfaced late.
Stable signing is **not needed** for the home appliance. The container/cloud path ([[SQUIRE-A-0016]] /
[[SQUIRE-I-0003]]) remains a roadmap option for other reasons, not as a fix for this.
