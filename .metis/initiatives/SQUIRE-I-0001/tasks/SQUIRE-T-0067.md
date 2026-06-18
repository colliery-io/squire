---
id: a-0011-3-onboarding-host-zone
level: task
title: "A-0011 #3 — onboarding: host-zone detection + seed timezone on first run"
short_code: "SQUIRE-T-0067"
created_at: 2026-06-18T12:48:56.360694+00:00
updated_at: 2026-06-18T12:48:56.360694+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# A-0011 #3 — onboarding: host-zone detection + seed timezone on first run

## Parent Initiative

[[SQUIRE-I-0001]] · Implements **[[SQUIRE-A-0011]]** sub-task **#3 of 4** — the "set/track timezone as part of onboarding" requirement. Depends on [[SQUIRE-T-0066]] (#2). Pairs with #4 ([[SQUIRE-T-0068]]) for in-app editing.

## Objective

Capture the household timezone at onboarding so "midnight" is correct from day one with zero manual setup: on first run, seed the `timezone` setting from `SQUIRE_TZ` (if set) else the **auto-detected host zone**, persist it, and reflect it in the live cell + the server banner.

## Acceptance Criteria

- [ ] On first provision (no `timezone` row), `squire-serve` resolves the zone: `SQUIRE_TZ` env → else `iana-time-zone::get_timezone()` (host zone) → else `"UTC"`; validates it parses (`jiff`), else falls back to UTC; persists via `set_setting("timezone", zone, by=admin/system)`.
- [ ] On every start, after opening the store, the live cell is seeded from `load_config()`; an existing household with no row gets the detected/UTC default written once. The resolved zone is shown in the startup banner (e.g. `Timezone: America/Los_Angeles`).
- [ ] `SQUIRE_TZ` overrides the detected zone on first run only (doesn't clobber a parent's later choice from #4). Idempotent across restarts.
- [ ] `cargo build`/`cargo test` green; manual check: unset `SQUIRE_TZ` ⇒ banner shows the Mac's zone; daily quest `today()` matches local date. `iana-time-zone` dep added.

## Implementation Notes

### Technical Approach
In `squire-home::open_household`/`squire-serve` first-run path: after provisioning, if `get_setting("timezone")` is `None`, compute the seed (`SQUIRE_TZ` | detected | UTC), validate via the #1 resolver, `set_setting`. Always reload the cell from `load_config()` post-open. Print the active zone alongside the existing banner lines. `iana-time-zone` is a tiny cross-platform crate returning the IANA name.

### Dependencies
[[SQUIRE-T-0066]] (#2 wiring — the cell + clock are live). [[SQUIRE-A-0011]].

### Risk Considerations
`iana-time-zone` can fail on exotic hosts → UTC fallback, never panic. Don't overwrite an existing `timezone` row (respect the parent's choice). `SQUIRE_TZ` is first-run convenience, not an ongoing override (document it). Rebuild + restart `squire-serve` to observe.

## Status Updates

*To be added during implementation*
