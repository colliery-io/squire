---
id: 001-household-configuration-per-tenant
level: adr
title: "Household configuration (per-tenant settings) + timezone-aware clock"
number: 1
short_code: "SQUIRE-A-0011"
created_at: 2026-06-18T12:37:54.328564+00:00
updated_at: 2026-06-18T12:47:33.644103+00:00
decision_date: 
decision_maker: dylan.storey
parent: 
archived: false

tags:
  - "#adr"
  - "#phase/decided"


exit_criteria_met: false
initiative_id: NULL
---

# ADR-11: Household configuration (per-tenant settings) + timezone-aware clock

## Context

Daily quest availability is supposed to reset at **the family's local midnight**, but the clock (`store::SystemClock`) computes the domain `Date` (a Monday-aligned day-count) in **UTC** ("timezone config is a later refinement"). For a US household a "daily" quest therefore rolls over in the afternoon/evening, not at midnight.

The user asked that the fix be done "right" — **underpinned by a concept of household configuration**, not a one-off timezone field. There is no settings concept today: a household *is* a tenant SQLite file (ADR A-0002, schema-per-tenant, no `tenant_id`); identity lives in `users`; everything else is domain definitions/events. Household-level settings (timezone now; week-start, display name, quiet-hours, default-reward later) have no home, and `SystemClock` is a UTC unit struct threaded through `Store<C>`, `AppState`, and `KeepState`.

## Decision

Introduce a first-class **`HouseholdConfig`** — a per-tenant, typed settings record — and make the clock derive the local date from it.

1. **Type** — `HouseholdConfig { timezone: String /* IANA, e.g. "America/Los_Angeles" */ }` in `domain-core` (serde-gated like the other contract DTOs). Extensible by adding fields; the **first** field is `timezone`.
2. **Storage** — a **key/value `config` table** per tenant: `config(key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_by TEXT, updated_at BIGINT NOT NULL)`. The `key` **primary key is the lookup index** (fast point reads). It is a **long** table — config grows by adding *rows*, not columns, so no migration is needed per new setting. Access: `Store::get_setting(key) -> Option<String>` / `Store::set_setting(key, value, by)` (upsert, audit-stamped). `Store::load_config()` assembles a typed [`HouseholdConfig`] from the known keys (defaulting any absent ones); the `timezone` row is seeded on first provision. The typed struct stays the **in-memory view**; the KV table is the **persistence**.
3. **Runtime representation — a hot, lock-free forward projection.** The live config is held in `Arc<ArcSwap<ConfigView>>` (`arc-swap`), where `ConfigView { config: HouseholdConfig, tz: jiff::tz::TimeZone }` carries both the pure typed view (for handlers) and the **pre-parsed** timezone (for the clock's hot path). Readers call `.load()` — a lock-free atomic that returns an `Arc<ConfigView>` snapshot — so getters are cheap Arc clones, never a lock. The cell is a **forward projection** kept current by the single-writer path (AR-1): every `set_setting` write reads the row back under the store lock and **hot-swaps** the cell, so the in-memory view always equals the DB without any polling (there are no out-of-band writers per tenant). On startup the cell is seeded from `load_config()`. The same `Arc` is shared by the clock and the Keep/api handlers.
4. **Clock** — replace `SystemClock` with **`LocalClock`** holding the shared `Arc<ArcSwap<ConfigView>>`: `now()` stays unix-millis UTC; `today()` does a lock-free `.load()`, takes the pre-parsed `tz`, converts `now()` into that zone's local calendar day, then to the Monday-aligned day-count (DST-correct). A timezone change applies **live, without restarting** (the next `today()` sees the swapped cell). `FixedClock` (tests) is unchanged.
5. **Onboarding** — on first run `squire-serve` seeds the `timezone` key from `SQUIRE_TZ` if set, else the **auto-detected host timezone** (`iana-time-zone`), falling back to `UTC`. Zero-config for the common case (the home server sits in the home), overridable for edge cases.
6. **Changing it later** — the Keep gains a small **Settings** tab to view/change the household timezone; saving `set_setting`s the row and hot-swaps the live cell (so the clock changes instantly). Editing on the phone is a later, native concern.
7. **tz library** — `jiff` (single crate, **bundles** the IANA tz db, DST-correct, no system tzdata dependency) + `arc-swap` for the hot cell, both living in `store` where the date math already is.

## Alternatives Analysis

| Option | Pros | Cons | Risk | Cost |
|--------|------|------|------|------|
| **KV `config` table + typed `HouseholdConfig` view + LocalClock (chosen)** | Real settings concept; per-tenant; grows by rows not columns (no per-setting migration); key-indexed reads; DST-correct; live updates | Values are stringly at rest (typed view re-asserts at the boundary); cross-crate clock type change; new dep | Low | M |
| Wide typed-column `config` table | Strong typing at rest | A migration per new setting; wide table grows awkwardly | Low | M |
| One-off `SQUIRE_TZ` env only, no persistence | Tiny | Not per-household; lost on env change; no in-app edit; not "a concept" | Low | S |
| Store a fixed UTC **offset** instead of an IANA zone | Trivial math | **Breaks twice a year at DST** — wrong by an hour for months | Med | S |

## Rationale

A **key/value `config` table** gives household-level settings a real, per-tenant home that grows by adding rows as options accrue — a long table, not an ever-widening one, and no schema migration per new setting. The `key` primary key indexes lookups. A typed `HouseholdConfig` view re-asserts types at the load boundary (defaulting/validating absent or malformed values), so handlers still work with a typed struct rather than raw strings. Per-tenant by construction (A-0002); writes are audit-stamped (A-0007). An IANA zone (not an offset) is the only DST-correct choice. Auto-detecting the host zone makes the common case zero-config yet correct. A swappable in-clock timezone keeps the parent's "change timezone" instant. `jiff` bundles tzdata so the binary stays self-contained (consistent with bundled SQLite).

## Consequences

### Positive
- Daily/weekly quests reset at the household's real local midnight; DST handled automatically.
- A durable place for future household settings (week-start, display name, quiet-hours, default reward) without re-architecting.
- Per-tenant by construction — correct for the future hosted multi-tenant world (A-0002), not a process global.

### Negative
- Cross-crate change: `SystemClock` → `LocalClock` ripples through `Store<C>` instantiations, `AppState`, `KeepState`, `squire-serve`, and `Store<SystemClock>` aliases (e.g. `identity::prod`). Tests on `FixedClock` are unaffected.
- One new dependency (`jiff`) + one (`iana-time-zone`) for detection.
- `OneOff{due}` dates authored under the old UTC convention shift by up to a day at zone boundaries (cosmetic; pre-existing quests are daily/weekly).

### Neutral
- `now()` semantics unchanged (unix-millis UTC); only `today()` becomes zone-aware.
- The Keep Settings surface is minimal now; the phone gets timezone management later with the native authoring work.
- Config values are strings at rest; the typed `HouseholdConfig` view validates/defaults them on load (e.g. an unknown timezone string falls back to UTC rather than panicking).