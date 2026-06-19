---
id: first-install-onramp-one-line
level: task
title: "First-install onramp: one-line installer + clickable Squire.app + browser-based first-run admin"
short_code: "SQUIRE-T-0091"
created_at: 2026-06-19T19:56:53.849098+00:00
updated_at: 2026-06-19T19:56:53.849098+00:00
parent: SQUIRE-I-0002
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: true
initiative_id: SQUIRE-I-0002
---

# First-install onramp: one-line installer + clickable Squire.app

## Parent Initiative

[[SQUIRE-I-0002]] — the "operator onboarding for others" open question from [[SQUIRE-A-0012]].

## Objective

Make a new household's *first* server install trivial: one line to install, a double-clickable app to
run, and admin setup in the browser — no terminal, no env vars, no code-signing.

## What shipped

- **First-run UX** (`squire-serve`): no longer errors without `SQUIRE_ADMIN_SECRET`. With the secret
  it still bootstraps headlessly; **without** it, it starts with no admin and the operator creates one
  via the Keep's existing "First run? Create the admin Knight" form. (Verified: starts, `/register`
  creates the admin, login works.)
- **One-line installer** (`dist/install.sh`, published to the public dist repo):
  `curl -fsSL https://raw.githubusercontent.com/colliery-io/squire/main/install.sh | sh`. Detects
  OS/arch, downloads the latest signed `squire-serve` for the platform → `~/.local/bin`, and on macOS
  builds `~/Applications/Squire.app` (launcher starts the server + opens the Keep) with a crest icon.
  Honors `SQUIRE_BIN_DIR`/`SQUIRE_APP_DIR` for testing.
- **Crest icon** (`dist/icon-source.svg` → `dist/squire.icns`, also on the dist repo).
- **No code-signing needed**: curl-fetched files aren't quarantined, so this sidesteps macOS
  Gatekeeper / Windows SmartScreen.

## Acceptance Criteria

- [x] One-line `curl | sh` installs the right binary for the platform from the public release.
- [x] macOS gets a clickable `Squire.app` (crest icon) that starts the server + opens the Keep.
- [x] First run needs no terminal/env — admin created in the browser; `/register` + login verified.

## Follow-ups

- Windows `install.ps1` + Start-Menu shortcut; Linux `.desktop` entry.
- Optional run-on-login (launchd/systemd) and a stop/status affordance.
- Build the icon/installer into CI publishing (today the icns + install.sh are committed to the dist
  repo by hand).

## Status Updates

- Done + verified end-to-end via the real `curl | sh` one-liner (binary + icon'd .app) and the
  browser first-run admin flow.
