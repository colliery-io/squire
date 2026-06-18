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

- [x] `squire_home::ensure_timezone(&store)`: if no/blank `timezone` row, resolve `SQUIRE_TZ` → else `iana_time_zone::get_timezone()` (host) → else `"UTC"`; validate via `store::valid_timezone` (UTC fallback); persist `set_setting("timezone", zone, None)`. Returns the active zone.
- [x] Called by **both** `squire-serve` and the demo `squire-home` before `serve` (which builds the clock from the now-seeded `load_config()`). Active zone shown in the startup banner (`Timezone: …`).
- [x] **Idempotent / respects later choice**: an existing non-empty `timezone` is returned untouched (never clobbers a #4 change). `SQUIRE_TZ` is first-run-only convenience.
- [x] `cargo build` green; verified live: no `SQUIRE_TZ` ⇒ banner showed the host zone **America/Detroit**; `SQUIRE_TZ=America/New_York` ⇒ used it; `SQUIRE_TZ=Not/AZone` ⇒ fell back to **UTC**. `iana-time-zone` dep added; `valid_timezone` exposed from `store` (so squire-home needs no direct `jiff`).

## Implementation Notes

### Technical Approach
In `squire-home::open_household`/`squire-serve` first-run path: after provisioning, if `get_setting("timezone")` is `None`, compute the seed (`SQUIRE_TZ` | detected | UTC), validate via the #1 resolver, `set_setting`. Always reload the cell from `load_config()` post-open. Print the active zone alongside the existing banner lines. `iana-time-zone` is a tiny cross-platform crate returning the IANA name.

### Dependencies
[[SQUIRE-T-0066]] (#2 wiring — the cell + clock are live). [[SQUIRE-A-0011]].

### Risk Considerations
`iana-time-zone` can fail on exotic hosts → UTC fallback, never panic. Don't overwrite an existing `timezone` row (respect the parent's choice). `SQUIRE_TZ` is first-run convenience, not an ongoing override (document it). Rebuild + restart `squire-serve` to observe.

## Status Updates

**2026-06-18 — Done + verified live.** Added `ensure_timezone(&store)` to squire-home (idempotent seed: `SQUIRE_TZ` → detected host zone → UTC, validated before persist) + a `store::valid_timezone` helper so squire-home needs no direct `jiff`. Wired into both binaries before `serve`, with a `Timezone:` banner line. Live check: bare run detected **America/Detroit** (so this household is Eastern); `SQUIRE_TZ=America/New_York` honoured; invalid zone → UTC. `iana-time-zone` dep added. Daily quests now reset at the household's local midnight on the real server once deployed. Next: [[SQUIRE-T-0068]] (#4) the Keep Settings tab to change it live.
