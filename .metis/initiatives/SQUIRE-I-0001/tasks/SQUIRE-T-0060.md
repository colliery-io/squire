---
id: household-timezone-onboarding
level: task
title: "A-0011 #1 — config KV store + HouseholdConfig + ArcSwap cell + LocalClock"
short_code: "SQUIRE-T-0060"
created_at: 2026-06-18T11:35:15.846742+00:00
updated_at: 2026-06-18T12:14:00.000000+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# A-0011 #1 — config KV store + HouseholdConfig + ArcSwap cell + LocalClock

## Parent Initiative

[[SQUIRE-I-0001]] · Implements **[[SQUIRE-A-0011]]** (household configuration + tz-aware clock), sub-task **#1 of 4**. Foundation for #2 wiring ([[SQUIRE-T-0066]]), #3 onboarding ([[SQUIRE-T-0067]]), #4 Keep Settings ([[SQUIRE-T-0068]]).

## Objective

Build the storage + runtime + clock layer for household configuration, **additively** (keep `SystemClock` so the tree still compiles until #2 swaps usages): a key/value `config` table, a typed `HouseholdConfig` view, the `Arc<ArcSwap<ConfigView>>` hot cell, and `LocalClock` that derives the local `today()` from the live timezone.

## Acceptance Criteria

- [ ] **Migration** `config(key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_by TEXT, updated_at BIGINT NOT NULL)` (new dir; PK is the lookup index) + `down.sql`. `schema.rs` table def added; existing-DB open still runs clean.
- [ ] **`domain_core::HouseholdConfig { timezone: String }`** (serde-gated, `Default` → `"UTC"`). **`store`**: `get_setting(key)`, `set_setting(key, value, by)` (upsert, audit-stamped via `now()`), `load_config() -> HouseholdConfig` (assembles from known keys, defaults absent).
- [ ] **Hot cell**: `ConfigView { config: HouseholdConfig, tz: jiff::tz::TimeZone }` + `LiveConfig = Arc<ArcSwap<ConfigView>>`; a resolver parsing the zone with **UTC fallback** on an unknown string (no panic). Pure `date_in_zone(millis, &tz) -> Date` (Monday-aligned day-count) factored out + unit-tested (a 23:00 PST instant resolves to the PST calendar day, not the UTC next-day).
- [ ] **`LocalClock { live: LiveConfig }`** impl `Clock`: `now()` = system millis (unchanged); `today()` = `date_in_zone(now, &live.load().tz)` (lock-free). `SystemClock`/`FixedClock` untouched; `cargo test -p store` + `-p domain-core` green; new deps `jiff` + `arc-swap` added to `store`.

## Implementation Notes

### Technical Approach
New migration under `crates/store/migrations/`. `domain-core/src/contract/` gets `HouseholdConfig` (serde feature). `store`: KV accessors on the repository; `ConfigView`/`LiveConfig`/resolver in a small `config` module; `date_in_zone` using `jiff::Timestamp::from_millisecond(...).to_zoned(tz).date()` → days-since-`1970-01-01` `+ UNIX_TO_MONDAY_EPOCH_OFFSET`. Keep `SystemClock` in place; add `LocalClock` alongside. Do NOT rewire callers yet (that's #2).

### Dependencies
[[SQUIRE-A-0011]]. No contract break (additive). #2/#3/#4 depend on this.

### Risk Considerations
`jiff` date-math API: verify the days-since-epoch conversion against the existing `date_from_unix_millis` for UTC (must agree when tz==UTC). UTC fallback on bad zone strings (never panic). `arc-swap` `.load()` returns a `Guard`; use `.load_full()` where an owned `Arc` is needed. Keep the additive change compiling on its own.

## Status Updates

*To be added during implementation*
