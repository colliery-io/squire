---
id: a-0011-2-wire-localclock-through
level: task
title: "A-0011 #2 — wire LocalClock through Store/AppState/KeepState/squire-serve"
short_code: "SQUIRE-T-0066"
created_at: 2026-06-18T12:48:54.962394+00:00
updated_at: 2026-06-18T13:08:21.361184+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# A-0011 #2 — wire LocalClock through Store/AppState/KeepState/squire-serve

## Parent Initiative

[[SQUIRE-I-0001]] · Implements **[[SQUIRE-A-0011]]** sub-task **#2 of 4**. Depends on [[SQUIRE-T-0060]] (#1, which adds `LocalClock` + the `ConfigView` cell additively). Unblocks #3 ([[SQUIRE-T-0067]]) and #4 ([[SQUIRE-T-0068]]).

## Objective

Make the running system actually use `LocalClock`: swap `SystemClock` → `LocalClock` at the composition roots and thread the shared `Arc<ArcSwap<ConfigView>>` cell so the clock and the Keep/api handlers read **one** live config. After this task, `today()` everywhere reflects the household timezone (still defaulting to UTC until #3 seeds a real zone).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] **Refinement (less ripple than the ADR's worst case):** the store's clock `today()` is **unused** (verified — all scheduling `today()`/`now()` comes from `state.clock`), so the store **stays `Store<SystemClock>`** (audit `now()` is UTC millis, correct) and `SharedStore`/`identity`/`tenant`/`prod` are **untouched**. Only the two **handler** clocks change.
- [x] `AppState { clock: LocalClock }` and `KeepState { clock: LocalClock }`. `new`/`from_parts`/`local`/`local_prod` seed a `LocalClock` from `store.load_config()` (helper `clock_from_store`); added `AppState::with_clock` + `KeepState::from_parts_with_clock` for an explicit **shared** clock.
- [x] `squire-home::serve` builds **one** shared `LocalClock` (seeded from the store's config) and passes clones to both `AppState::with_clock` + `KeepState::from_parts_with_clock`, so the api and Keep share one hot cell (a Keep tz change will apply to both without restart — wired for #4). `engine.handle(&snap, cmd, &self.clock)` compiles unchanged.
- [x] `cargo build --workspace` + `cargo test --workspace` green (all suites: api/keep/identity/store/domain-core). `FixedClock` tests untouched. Behaviour-neutral until #3 seeds a real zone (default config ⇒ UTC ⇒ same as before).

## Implementation Notes

### Technical Approach
Construct `let live = LiveConfig::seed(UTC)` early; `let clock = LocalClock::new(live.clone())`. Open the store with that clock; after open, `live.store(load_config().resolve())`. Pass `clock` + `live` into `AppState`/`KeepState`. The `provisioner.open` clock generic already exists (`Store<C: Clock>`), so the main churn is the concrete type names at the roots + the `Store<SystemClock>` aliases. Keep `SystemClock` in the tree (other tools/tests may use it) but the servers use `LocalClock`.

### Dependencies
[[SQUIRE-T-0060]] (LocalClock + ConfigView + LiveConfig exist). [[SQUIRE-A-0011]].

### Risk Considerations
The cross-crate type swap is the bulk of A-0011's "Negative" consequence — do it mechanically, lean on the compiler. Watch for `Copy` assumptions on the old unit-struct `SystemClock` (`LocalClock` is `Clone`, not `Copy`). Ensure exactly one cell is shared (don't accidentally build two). Rebuild + restart any linked binary before live testing (recurring gotcha).

## Status Updates

**2026-06-18 — Done (cleaner than planned).** Discovered the store's clock `today()` is dead — every scheduling `today()`/`now()` comes from `state.clock` — so I avoided the whole `SharedStore`/`identity`/`tenant`/`prod` ripple the ADR flagged. Only `AppState.clock` and `KeepState.clock` became `LocalClock` (the store keeps `SystemClock` for UTC audit timestamps). Constructors seed the clock from `store.load_config()` (`clock_from_store`); added `with_clock`/`from_parts_with_clock` so `squire-home::serve` shares ONE cell across the api + Keep (live tz change propagates to both, ready for #4). `cargo build --workspace` + `cargo test --workspace` fully green; no test changes needed. Behaviour-neutral until [[SQUIRE-T-0067]] (#3) seeds the host zone.