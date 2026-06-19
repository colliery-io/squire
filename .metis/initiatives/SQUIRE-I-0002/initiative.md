---
id: github-based-delivery-public-dist
level: initiative
title: "GitHub-based delivery: public dist repo, server pull + self-update, QR install"
short_code: "SQUIRE-I-0002"
created_at: 2026-06-19T17:48:08.072768+00:00
updated_at: 2026-06-19T17:48:08.072768+00:00
parent: SQUIRE-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/decompose"


exit_criteria_met: false
estimated_complexity: L
initiative_id: github-based-delivery-public-dist
---

# GitHub-based delivery: public dist repo, server pull + self-update, QR install

Implements [[SQUIRE-A-0012]]: source private, artifacts public (`colliery-io/squire`); the home
server pulls releases + is the sole LAN distributor; both the phone APK and the server binary
self-update; new phones install via a QR.

## Goal

Turn the manual, single-operator deploy into a hands-off product-distribution pipeline:
**tag → CI builds+signs → public dist Release → every household's server self-pulls → phones update
over LAN**, with a QR onramp for first install.

## Workstreams (tasks)

1. **CI release pipeline (APK)** — ✅ done (commit `db378d5`, no separate ticket):
   `.github/workflows/release-apk.yml` builds+signs on a `v*` tag and publishes `squire-<code>.apk`
   to `squire`.
2. **Server startup-pull** — [[SQUIRE-T-0087]] (active): `squire-serve` fetches the latest APK from
   `squire` into its updates dir on startup (+ schedule), derives the manifest, offline-tolerant.
   Removes the last manual publish step.
3. **QR click-to-install** — ✅ [[SQUIRE-T-0088]]: the Keep's Pair tab renders a QR of the LAN APK URL.
4. **Cross-platform server binaries + server self-update** — ✅ [[SQUIRE-T-0089]]: CI builds the server
   for macOS/Windows/Linux into `squire`; the running server self-replaces from there.
5. **Background update checks** — ✅ [[SQUIRE-T-0090]]: a WorkManager job checks for upgrades while
   backgrounded and notifies (chosen over a foreground service; background state-sync is a follow-up).

**All five workstreams implemented** (v0.7.3 ships the full APK + cross-platform server binary set).
Remaining = runtime validation that needs devices/two releases (server self-replace, the Android
background firing) + relaunching the operator's running prod onto the new binary.

## Open questions

- **Operator onboarding for others**: how does a *new* household get the server binary the first time
  (installer / `brew` / a download page)? (Feeds workstream 4.)
- **Version policy**: tag-driven; write down the bump rules (currently manual, at versionCode 6 / 0.7.2).
- **Server GH auth for the pull**: `squire` is public, so the APK pull needs no token — confirm we
  never need auth for reads (only CI's publish needs the cross-repo PAT).

## Exit criteria

A fresh `v*` tag flows end-to-end with no manual steps: CI publishes to `squire`, a server picks
up the new APK on its next pull and serves it, and a phone updates over LAN — plus the server can
self-update its own binary and a new phone can install from the QR.
