---
id: usage-feedback-polish-v0711
level: initiative
title: "Usage-feedback polish (v0.7.11)"
short_code: "SQUIRE-I-0005"
created_at: 2026-06-22T03:00:00+00:00
updated_at: 2026-06-22T03:00:00+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: true
estimated_complexity: M
initiative_id: usage-feedback-polish-v0711
---

# Usage-feedback polish (v0.7.11)

Five fixes from real family usage (operator, 2026-06-22). All implemented + tested; shipping as v0.7.11.

## Items
1. ✅ **Self-update 403 rate-limit** ([[SQUIRE-T-0117]]) — authenticate the self-update + apk-sync
   GitHub API calls (`SQUIRE_UPDATE_TOKEN`/`GITHUB_TOKEN`); graceful + logged when absent. `54445c1`.
   *Deploy step: set `SQUIRE_UPDATE_TOKEN` in the prod launchd plist for it to take effect.*
2. ✅ **Completed quest drops from the active log** — `quests_today` now omits
   CompletedToday/TakenByOther; the active list shows only actionable quests + shrinks as they're done.
   Completed ones live in recent activity. `c0bd73c`.
3. ✅ **One-off quest retires once completed** — a OneOff with any approved completion is excluded
   from the list for good (no reappearing next day); `total_completions(Scope::Quest)` check. `c0bd73c`.
4. ✅ **Recent feeds capped + ordered** — `my_claims`/`my_requests` were uncapped + oldest-first (the
   son's spamming buried the recent ones); now newest-first, last 10 (`RECENT_LIMIT`), like adjustments. `c0bd73c`.
5. ✅ **Immediate turn-in feedback** — the child home shows a "Sent!" snackbar on every Done/Redeem
   tap (repeatable items stay Available, so without it the kid re-taps/spams). `55fa426`.

## Tests
- New Gherkin scenario (claims.feature): "an approved quest drops off the active list" — 11 api
  cucumber scenarios green. `:app` compiles; Paparazzi snapshots green; full Rust suite green.

## Ship
v0.7.11 via the GitHub pipeline (ADR [[SQUIRE-A-0012]]). Note #1's token must be set in the prod env
for reliable self-update; otherwise prod is pulled manually (as for v0.7.10) until then. Phones OTA to
the new APK for #5 + (carried from v0.7.10) the Pay/History/Settings parity screens.
