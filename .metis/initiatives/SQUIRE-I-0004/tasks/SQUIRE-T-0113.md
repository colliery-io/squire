---
id: parity-household-timezone-setting
level: task
title: "Parity: household timezone setting on Android Knight (new Knight config API + UI)"
short_code: "SQUIRE-T-0113"
created_at: 2026-06-21T23:05:29.967768+00:00
updated_at: 2026-06-21T23:05:29.967768+00:00
parent: SQUIRE-I-0004
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0004
---

# Parity: household timezone setting on Android Knight

The Keep can set the household IANA timezone (`GET/PUT /api/config`, hot-swaps the live day-boundary);
Android Knight cannot. Close the gap ([[SQUIRE-I-0004]]).

## Backend (new — LAN API does not expose config yet)
- Add Knight-gated `GET /admin/config` + `PUT /admin/config` to `crates/api`, delegating to the same
  shared config cell the Keep uses (`crates/keep/src/config.rs` logic) so the timezone hot-swap applies
  to both servers immediately (no restart), exactly like the Keep path.
- Validate IANA zone (400 on unknown), audit-stamp the change.
- SDK regen (batch with [[SQUIRE-T-0112]]).

## Scope (Android Knight)
- A small **Settings** surface (home dropdown → Settings): show current timezone, pick a new IANA zone,
  save → `PUT /admin/config`.

## Acceptance
- [ ] `GET/PUT /admin/config` (Knight-gated) in `crates/api`, tested (Gherkin scenario, incl. bad-zone
  400 + live hot-swap).
- [ ] Android Settings can read + change the timezone; change reflected without a restart.
- [ ] No config drift between api + keep (shared cell).
