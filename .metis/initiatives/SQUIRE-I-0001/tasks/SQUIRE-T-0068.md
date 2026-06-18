---
id: a-0011-4-keep-settings-tab-view
level: task
title: "A-0011 #4 — Keep Settings tab: view/change household timezone (live)"
short_code: "SQUIRE-T-0068"
created_at: 2026-06-18T12:48:57.324527+00:00
updated_at: 2026-06-18T12:48:57.324527+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# A-0011 #4 — Keep Settings tab: view/change household timezone (live)

## Parent Initiative

[[SQUIRE-I-0001]] · Implements **[[SQUIRE-A-0011]]** sub-task **#4 of 4** — the in-app surface for the household timezone. Depends on [[SQUIRE-T-0067]] (#3). Completes the timezone story (phone-side editing is deferred to the native work).

## Objective

Let a parent view and change the household timezone from the Keep, applied **live** (no restart): a Settings tab backed by config read/write endpoints that update the row and hot-swap the live cell so the clock changes on the next `today()`.

## Acceptance Criteria

- [ ] Keep endpoints (Operator/Knight-gated, same as the other `/api/*`): `GET /api/config` → the current `HouseholdConfig` (+ available zones if practical); `PUT /api/config` (or `/api/config/timezone`) → validates the zone (`jiff`), `set_setting`s it (audit-stamped), and **hot-swaps the live cell**.
- [ ] A new **Settings** tab in the Keep tabbed shell ([[SQUIRE-T-0061]]) with a timezone picker (a curated `<select>` of common IANA zones + free-text fallback) showing the current value; save → success toast; invalid zone → inline error.
- [ ] Changing the zone takes effect with **no restart**: a subsequent `today()` (and the next phone refresh) reflects the new local date. Verified live on a throwaway server (set zone → `GET /api/config` echoes it → a boundary timestamp's `today()` shifts).
- [ ] `cargo build -p keep` + `node --check` green; Settings tab rendered/screenshot.

## Implementation Notes

### Technical Approach
`keep`: add `config` handlers (read assembles `load_config()`; write validates + `set_setting` + `live.store(resolve(load_config()))`). Route `GET/PUT /api/config`. `keep.js`: a `settings` tab + `loadSettings()`/save; `index.html`: a Settings panel + nav tab; reuse the themed form styles. Curated zone list can be a small static array (US zones first) with an "other" text input.

### Dependencies
[[SQUIRE-T-0067]] (#3 — seeded zone + live cell), [[SQUIRE-T-0061]] (tabbed shell). [[SQUIRE-A-0011]].

### Risk Considerations
Validate the zone before persisting (reject unknown → keep the old value). The hot-swap must use the SAME `Arc` the clock holds (don't rebuild a parallel cell). Keep the zone list short but escape-hatched (free text) so any IANA name works. Rebuild + restart the live Keep to deploy.

## Status Updates

*To be added during implementation*
