---
id: a-0011-2-wire-localclock-through
level: task
title: "A-0011 #2 — wire LocalClock through Store/AppState/KeepState/squire-serve"
short_code: "SQUIRE-T-0066"
created_at: 2026-06-18T12:48:54.962394+00:00
updated_at: 2026-06-18T12:48:54.962394+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# A-0011 #2 — wire LocalClock through Store/AppState/KeepState/squire-serve

## Parent Initiative

[[SQUIRE-I-0001]] · Implements **[[SQUIRE-A-0011]]** sub-task **#2 of 4**. Depends on [[SQUIRE-T-0060]] (#1, which adds `LocalClock` + the `ConfigView` cell additively). Unblocks #3 ([[SQUIRE-T-0067]]) and #4 ([[SQUIRE-T-0068]]).

## Objective

Make the running system actually use `LocalClock`: swap `SystemClock` → `LocalClock` at the composition roots and thread the shared `Arc<ArcSwap<ConfigView>>` cell so the clock and the Keep/api handlers read **one** live config. After this task, `today()` everywhere reflects the household timezone (still defaulting to UTC until #3 seeds a real zone).

## Acceptance Criteria

- [ ] A single `LiveConfig` (`Arc<ArcSwap<ConfigView>>`) is constructed once per server at startup (seeded from `Store::load_config()`), shared (clones of the `Arc`) into: the `Store` it opens with, `AppState`, and `KeepState`. No second source of truth.
- [ ] `store::tenant`/`Store::open` (and `provisioner.open(handle, clock)`) accept/return a `LocalClock`; `AppState { clock: LocalClock }` and `KeepState { clock: LocalClock }` replace `SystemClock`. `Store<SystemClock>` aliases (e.g. `identity::prod`, any `serve_demo`) updated to `Store<LocalClock>` or made generic.
- [ ] `squire-home`/`squire-serve` build the cell + clock before opening the store and pass them through. `engine.handle(&snap, cmd, &self.clock)` calls compile unchanged (clock is just a different `Clock` impl).
- [ ] `cargo build` (workspace) + `cargo test` green; `FixedClock`-based tests untouched; a smoke check that `today()` matches the configured zone (UTC default ⇒ same as before). No behavioural change yet beyond the clock source.

## Implementation Notes

### Technical Approach
Construct `let live = LiveConfig::seed(UTC)` early; `let clock = LocalClock::new(live.clone())`. Open the store with that clock; after open, `live.store(load_config().resolve())`. Pass `clock` + `live` into `AppState`/`KeepState`. The `provisioner.open` clock generic already exists (`Store<C: Clock>`), so the main churn is the concrete type names at the roots + the `Store<SystemClock>` aliases. Keep `SystemClock` in the tree (other tools/tests may use it) but the servers use `LocalClock`.

### Dependencies
[[SQUIRE-T-0060]] (LocalClock + ConfigView + LiveConfig exist). [[SQUIRE-A-0011]].

### Risk Considerations
The cross-crate type swap is the bulk of A-0011's "Negative" consequence — do it mechanically, lean on the compiler. Watch for `Copy` assumptions on the old unit-struct `SystemClock` (`LocalClock` is `Clone`, not `Copy`). Ensure exactly one cell is shared (don't accidentally build two). Rebuild + restart any linked binary before live testing (recurring gotcha).

## Status Updates

*To be added during implementation*
