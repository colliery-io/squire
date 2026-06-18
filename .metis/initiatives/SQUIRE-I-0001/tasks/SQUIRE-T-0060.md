---
id: a-0011-1-config-kv-store
level: task
title: "A-0011 #1 — config KV store + HouseholdConfig + ArcSwap cell + LocalClock"
short_code: "SQUIRE-T-0060"
created_at: 2026-06-18T11:35:15.846742+00:00
updated_at: 2026-06-18T12:51:05.559291+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# A-0011 #1 — config KV store + HouseholdConfig + ArcSwap cell + LocalClock

## Parent Initiative

[[SQUIRE-I-0001]] · Implements **[[SQUIRE-A-0011]]** (household configuration + tz-aware clock), sub-task **#1 of 4**. Foundation for #2 wiring ([[SQUIRE-T-0066]]), #3 onboarding ([[SQUIRE-T-0067]]), #4 Keep Settings ([[SQUIRE-T-0068]]).

## Objective

Build the storage + runtime + clock layer for household configuration, **additively** (keep `SystemClock` so the tree still compiles until #2 swaps usages): a key/value `config` table, a typed `HouseholdConfig` view, the `Arc<ArcSwap<ConfigView>>` hot cell, and `LocalClock` that derives the local `today()` from the live timezone.

## Acceptance Criteria

## Acceptance Criteria

- [x] **Migration** `config(key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_by TEXT, updated_at BIGINT NOT NULL)` (`2026-06-18-000000_household_config`) + `down.sql`. `schema.rs` table def added; workspace builds (existing-DB open runs the migration on open).
- [x] **`domain_core::HouseholdConfig { timezone: String }`** (serde+openapi-gated, `Default` → `"UTC"`) + `config_keys::TIMEZONE`. **`store`**: `get_setting`, `set_setting(key, value, by)` (upsert via `run_upsert!`, audit-stamped from `clock.now()`), `load_config() -> HouseholdConfig`.
- [x] **Hot cell**: `ConfigView { config, tz: jiff::tz::TimeZone }` + `LiveConfig = Arc<ArcSwap<ConfigView>>` + `live_config()`; resolver with **UTC fallback** on a bad zone (tested). Pure `date_in_zone(millis, &tz)` (Hinnant `days_from_civil` + Monday offset) — tested: 07:30 UTC resolves to the LA day one behind, and UTC agrees exactly with the legacy `date_from_unix_millis`.
- [x] **`LocalClock { live }`** impl `Clock`: `now()` = shared `now_millis()`; `today()` = `date_in_zone(now, &live.load().tz)` (lock-free). `SystemClock`/`FixedClock` untouched. `cargo test -p store -p domain-core` green (3 new settings tests pass); `jiff` + `arc-swap` added. (Module named `settings` to avoid colliding with the `config` schema table.)

## Implementation Notes

### Technical Approach
New migration under `crates/store/migrations/`. `domain-core/src/contract/` gets `HouseholdConfig` (serde feature). `store`: KV accessors on the repository; `ConfigView`/`LiveConfig`/resolver in a small `config` module; `date_in_zone` using `jiff::Timestamp::from_millisecond(...).to_zoned(tz).date()` → days-since-`1970-01-01` `+ UNIX_TO_MONDAY_EPOCH_OFFSET`. Keep `SystemClock` in place; add `LocalClock` alongside. Do NOT rewire callers yet (that's #2).

### Dependencies
[[SQUIRE-A-0011]]. No contract break (additive). #2/#3/#4 depend on this.

### Risk Considerations
`jiff` date-math API: verify the days-since-epoch conversion against the existing `date_from_unix_millis` for UTC (must agree when tz==UTC). UTC fallback on bad zone strings (never panic). `arc-swap` `.load()` returns a `Guard`; use `.load_full()` where an owned `Arc` is needed. Keep the additive change compiling on its own.

## Status Updates

**2026-06-18 — Done.** Built the household-config foundation per [[SQUIRE-A-0011]], additively (no caller rewire — that's #2). `domain-core`: `HouseholdConfig` + `config_keys`. `store`: `config` KV migration + schema; `get_setting`/`set_setting`/`load_config`; a `settings` module with `ConfigView`/`LiveConfig`/`live_config`/`date_in_zone` (pure Hinnant day-count) and `LocalClock` over `Arc<ArcSwap<ConfigView>>`; refactored `now_millis()` shared by `SystemClock` + `LocalClock`. Deps `jiff` (tz/civil-date) + `arc-swap`. **Gotcha:** the runtime module had to be named `settings` because `config` collided with the Diesel `config` table module. 3 new tests green (LA-vs-UTC midnight; UTC≡legacy; bad-zone→UTC fallback); `cargo build --workspace` + `cargo test -p store -p domain-core` green. Next: [[SQUIRE-T-0066]] (#2) wires `LocalClock` + the shared cell through `Store`/`AppState`/`KeepState`/`squire-serve`.