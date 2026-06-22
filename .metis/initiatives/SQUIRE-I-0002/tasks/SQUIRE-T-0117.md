---
id: authenticate-server-self-update
level: task
title: "Authenticate server self-update + apk-sync GitHub API calls (fix the 403 rate-limit)"
short_code: "SQUIRE-T-0117"
created_at: 2026-06-22T02:11:00+00:00
updated_at: 2026-06-22T02:11:00+00:00
parent: SQUIRE-I-0002
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0002
---

# Authenticate the self-update + apk-sync GitHub API calls

## Problem (observed 2026-06-21, v0.7.10 deploy)
The server's startup self-update (`crates/squire-home/src/updater.rs`, `self_update` crate) and the
apk-sync both hit the **unauthenticated** GitHub API (`api.github.com/repos/colliery-io/squire/releases`),
which is rate-limited to ~60 req/hr/IP. Under repeated restarts it returns **403** and the server skips the
update ("continuing on the current build") — so a release reaching prod is a **coin-flip on the rate-limit
window**. The v0.7.10 deploy only landed because the limit happened to reset; a manual `gh release
download` (authenticated) was the fallback.

## Fix
- Pass a token to the `self_update` GitHub backend (it supports `.auth_token(...)`) and to the apk-sync
  release query — raises the limit to 5000/hr. Source the token from env/keychain (e.g.
  `SQUIRE_UPDATE_TOKEN`); a read-only PAT or even a fine-grained public-repo token suffices.
- Degrade gracefully if no token (current behavior), but log clearly that updates are rate-limited.
- Optional: back off + retry on 403 with the reset header, rather than skipping the whole update.

## Acceptance
- [ ] Restarts reliably pick up a newer dist release (no 403 skips) when a token is configured.
- [ ] apk-sync reliably pulls the latest APK.
- [ ] No token → still boots, with a clear "updates rate-limited (unauthenticated)" log line.

## Note
Moot if the deployment moves to a **container** (image pull) or the **cloud** ([[SQUIRE-I-0003]]) — but
worth fixing for the current gold-path home appliance regardless.
