---
id: parity-event-log-history-on
level: task
title: "Parity: event-log/history on Android Knight (new Knight API endpoint + view)"
short_code: "SQUIRE-T-0112"
created_at: 2026-06-21T23:05:28.991309+00:00
updated_at: 2026-06-21T23:05:28.991309+00:00
parent: SQUIRE-I-0004
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: true
initiative_id: SQUIRE-I-0004
---

# Parity: event-log / history on Android Knight

The Keep has an event-log inspector (`/api/log/quest/{id}`, `/api/log/item/{id}`) for audit/history; the
Android Knight app has no history view. Close the gap ([[SQUIRE-I-0004]]).

## Backend (new — LAN API does not expose this yet)
- Add Knight-gated history endpoint(s) to `crates/api` mirroring the Keep inspector. Likely
  `GET /admin/log/quest/{id}` + `GET /admin/log/item/{id}` (or a unified `GET /admin/history?…`).
  Reuse the projection the Keep uses (`crates/keep/src/inspector.rs`) — pull it to a shared place if
  needed so api + keep don't drift.
- Decide the default view: per-quest/item drill-down (matches Keep) vs. a recent-household-activity
  feed. **Recommend** a recent-activity feed for the phone (more useful than id drill-down) + keep the
  per-entity log behind a tap.
- SDK regen (batch with [[SQUIRE-T-0113]]).

## Scope (Android Knight)
- A read-only **History** screen (reachable from the home dropdown): recent approvals/rejections/adjusts/
  redemptions with actor + timestamp + reason.

## Acceptance
- [ ] New Knight history endpoint(s) in `crates/api`, tested (Gherkin scenario).
- [ ] Android History screen renders the feed; offline-cached like the rest.
- [ ] Shared projection (no api/keep drift).
