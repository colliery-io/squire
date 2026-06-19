---
id: 001-delivery-via-public-dist-repo
level: adr
title: "Delivery via public dist repo: server pulls GitHub releases, distributes on LAN, and self-updates"
number: 1
short_code: "SQUIRE-A-0012"
created_at: 2026-06-19T17:40:55.212847+00:00
updated_at: 2026-06-19T17:47:32.429384+00:00
decision_date: 
decision_maker: Dylan Storey
parent: 
archived: false

tags:
  - "#adr"
  - "#phase/decided"


exit_criteria_met: false
initiative_id: NULL
---

# ADR-12: Delivery via public dist repo: server pulls GitHub releases, distributes on LAN, and self-updates

## Context

Squire is moving from a hand-operated home deploy to a product distributed to other households. The
current flow is manual and brittle (build → cp APK → hand-edit manifest → verify signer/hash →
redeploy; see the OTA work in [[SQUIRE-T-0085]]) and assumes a single operator on one machine.

Two forces shape delivery:
- **At-home topology**: phones live on the home LAN; the home computer runs the server
  (`squire-serve`). We already serve APKs over LAN with content-hash OTA detection (A-0012 builds on
  [[SQUIRE-T-0085]]).
- **Distribute to others**: the binaries (phone APK *and* the self-updating server) must be fetchable
  by people we don't control — so they cannot require a token to a private repo. The source repo
  (`colliery-io/squire`) is and should remain **private**.

The intent (operator's words): the computer app should self-update *and* keep the phone APKs current
for LAN distribution; a new phone installs via a QR "click to install"; the running app does in-place
upgrades and runs in the background.

## Decision

Adopt a **GitHub-releases delivery model split across two repos**, with the home server as the LAN
distribution hub:

1. **Source private, artifacts public.** Keep `colliery-io/squire` private. Create a separate
   **public** repo `colliery-io/squire-dist` that holds *only* signed release artifacts (the phone
   APK and the cross-platform server binaries). No source, no history.
2. **CI builds + signs on a version tag** in the private repo and publishes the artifacts to a
   **Release in `squire-dist`** (cross-repo via a scoped PAT). Release signing keys live as encrypted
   Actions secrets, never in the repo.
3. **The home server is the LAN distributor.** On startup (and on a schedule) `squire-serve` pulls the
   latest phone APK from `squire-dist` Releases into its updates dir, derives the manifest, and serves
   it over LAN exactly as today (content-hash OTA + the `/app/*` endpoints). Offline-tolerant: if the
   pull fails it serves whatever is already present.
4. **Phones never touch GitHub.** Initial install is a **QR** the Keep renders, encoding the LAN APK
   URL (`/app/squire-<code>.apk`); in-place upgrades flow from the LAN server (existing PackageInstaller
   path). A backgrounded foreground service keeps sync + update polling alive.
5. **The server self-updates too.** `squire-serve` checks `squire-dist` for a newer server binary for
   its platform, downloads, and swaps itself in — so a household that installed once stays current
   without manual intervention, and keeps its phone APKs current as a side effect of (3).

Unauthenticated reads from `squire-dist` are what make "distribute to others" work: any household's
server and any QR scan can fetch without credentials.

## Alternatives Analysis

| Option | Pros | Cons | Risk | Cost |
|--------|------|------|------|------|
| **Separate public dist repo (chosen)** | Source stays private; artifacts public + tokenless for others; reuses our LAN OTA | Two repos; cross-repo CI token; public binaries | Low | Medium |
| Make source repo public | Simplest wiring (one repo) | Exposes all source + history; hard to reverse | High | Low |
| Keep private + per-user token | Nothing public | Can't safely hand private-repo tokens to strangers; doesn't scale to "others" | High | Medium |
| Self-hosted CDN/hosting | Full control of download surface | Infra to run + pay for; auth/DNS/TLS ownership | Medium | High |

## Rationale

You cannot ship private-repo access to people you don't control, so distribution artifacts must be
publicly fetchable. Splitting into a public **artifacts-only** repo keeps the source and history
private while making downloads tokenless for arbitrary households. It reuses the LAN content-hash OTA
already built ([[SQUIRE-T-0085]]) rather than inventing a new channel, and keeps phones off the public
internet (they only ever talk to their home server).

## Consequences

### Positive
- Distribution to arbitrary households works with zero credentials handed out.
- Source + git history stay private.
- Reuses the existing LAN OTA serving + content-hash detection; phones never touch GitHub.
- Fully automated release: tag → CI → public artifacts → servers self-pull → phones update.

### Negative
- Release **binaries are public** (the APK and server binaries are downloadable by anyone). Acceptable
  for a signed client app, but worth a conscious eye on what the server binary embeds.
- Two repos to keep in lockstep; CI needs a cross-repo PAT (a managed secret) to publish.
- Server self-update is platform-specific (macOS/Windows/Linux build matrix + swap-in mechanics).

### Neutral
- Versioning becomes tag-driven; the version-bump policy (currently manual, at versionCode 6 / 0.7.2)
  should be written down.
- The content-hash OTA Phase 2 (drop versionCode) from [[SQUIRE-T-0085]] still applies on top of this.

## Review Triggers
- The dist repo / public binaries become a problem (abuse, leakage of anything sensitive in the server binary).
- A move to app-store distribution (Play Store / TestFlight-equivalent) supersedes LAN+GitHub delivery.