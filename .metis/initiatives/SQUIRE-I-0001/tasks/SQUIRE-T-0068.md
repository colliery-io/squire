---
id: a-0011-4-keep-settings-tab-view
level: task
title: "A-0011 #4 — Keep Settings tab: view/change household timezone (live)"
short_code: "SQUIRE-T-0068"
created_at: 2026-06-18T12:48:57.324527+00:00
updated_at: 2026-06-18T13:18:12.102856+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# A-0011 #4 — Keep Settings tab: view/change household timezone (live)

## Parent Initiative

[[SQUIRE-I-0001]] · Implements **[[SQUIRE-A-0011]]** sub-task **#4 of 4** — the in-app surface for the household timezone. Depends on [[SQUIRE-T-0067]] (#3). Completes the timezone story (phone-side editing is deferred to the native work).

## Objective

Let a parent view and change the household timezone from the Keep, applied **live** (no restart): a Settings tab backed by config read/write endpoints that update the row and hot-swap the live cell so the clock changes on the next `today()`.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] Keep endpoints (`Operator`/Knight-gated): `GET /api/config` → the current `HouseholdConfig`; `PUT /api/config` → validates the zone (`store::valid_timezone`, 400 on bad), `set_setting`s it (audit-stamped with the acting Knight), and **hot-swaps the live cell** (`state.clock.live().store(ConfigView::resolve(..))`). Verified: GET→Detroit, PUT Chicago→200+persisted, invalid→400 (unchanged), unauth→401.
- [x] A new **Settings** tab in the Keep tabbed shell with a timezone picker (curated `<select>` of common zones + free-text custom field) showing the current value; save → success line; invalid → inline error.
- [x] Changing the zone takes effect with **no restart** — `PUT` hot-swaps the shared cell the api + Keep clocks read, so the next `today()` uses the new zone. (Cell is shared via `squire-home::serve` from #2.)
- [x] `cargo build -p keep` + `node --check` green; `cargo test -p keep` green; Settings tab rendered (screenshot) + endpoints exercised on a live throwaway server.

## Implementation Notes

### Technical Approach
`keep`: add `config` handlers (read assembles `load_config()`; write validates + `set_setting` + `live.store(resolve(load_config()))`). Route `GET/PUT /api/config`. `keep.js`: a `settings` tab + `loadSettings()`/save; `index.html`: a Settings panel + nav tab; reuse the themed form styles. Curated zone list can be a small static array (US zones first) with an "other" text input.

### Dependencies
[[SQUIRE-T-0067]] (#3 — seeded zone + live cell), [[SQUIRE-T-0061]] (tabbed shell). [[SQUIRE-A-0011]].

### Risk Considerations
Validate the zone before persisting (reject unknown → keep the old value). The hot-swap must use the SAME `Arc` the clock holds (don't rebuild a parallel cell). Keep the zone list short but escape-hatched (free text) so any IANA name works. Rebuild + restart the live Keep to deploy.

## Status Updates

**2026-06-18 — Done + verified live.** Added a Keep `config` handler module (`GET`/`PUT /api/config`, Operator-gated) that reads `load_config()` and, on write, validates the zone, `set_setting`s it (audit-stamped), and hot-swaps the shared live cell. Added a **Settings** tab to the Keep shell (curated zone `<select>` + custom IANA field, save/status/error). Live test on a throwaway server: GET→America/Detroit; PUT America/Chicago→200 and persisted; PUT invalid→400 (value unchanged); unauth→401. Settings tab rendered. `cargo test -p keep` green. **Completes A-0011** — daily quests reset at the household's local midnight, the zone is captured at onboarding and changeable live from the Keep. Recurring-gotcha note: had to rebuild the `squire-home` binary (not just `-p keep`) before the new route appeared.