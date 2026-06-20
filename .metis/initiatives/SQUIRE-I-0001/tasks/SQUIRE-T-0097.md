---
id: currency-model-foundation-generic
level: task
title: "Currency model foundation: generic Currency + policy, fold coins in, zero-balance migration"
short_code: "SQUIRE-T-0097"
created_at: 2026-06-20T03:26:20.263647+00:00
updated_at: 2026-06-20T12:26:33.267771+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: true
initiative_id: SQUIRE-I-0001
---

# Currency model foundation (domain + store + migration)

Implements [[SQUIRE-A-0013]] in the domain + store. The risky, foundational layer — it rewrites
working money code and migrates the kids' live balances. **Do this slow and verified.**

## Scope

- **`Currency`** type (serde-stable tag: `Coins`, `Cash`, …) + a **policy** table (`spendable_in_shop`,
  `supports_payout`, `earns_achievement_bonus`, `floors_at_zero`, unit, display).
- **Generic events:** `PointsAdjusted` → `Adjusted { currency, amount, reason }`; `CompletionApproved`
  snapshots an **awarded map** (`{currency → amount}`); `AchievementUnlocked.bonus` → Coins (policy);
  `ItemRedeemed.cost` stays Coins (shop currency).
- **`Quest.rewards: Map<Currency,u32>`** (was `reward: Points`).
- **`balance(snap, squire, currency)`** projection, floored per policy.
- **Store:** currency-aware event rows + quest rewards; **additive/safe migration** (e.g. events
  `currency` column default `Coins`; a `quest_rewards` table or reuse `quests.reward` as the Coins
  amount + add rows). Backup/restore updated.

## Acceptance (hard gate — ADR-13)

- [x] **No balance moves:** `balance(snap, squire)` is now `balance_in(.., Coins)`, and the renamed
  `Adjusted{Coins}` path computes identically — so equivalence holds **by construction**. Every
  existing balance assertion across the property/golden/store suites passes unchanged (40 green
  test-result groups, 0 failures).
- [x] All existing domain + store + api + keep tests green (coin behavior unchanged).
- [x] Migration is reversible (down.sql) — additive `currency` column + a deterministic
  `PointsAdjusted`→`Adjusted{Coins}` rename; the store tests run it on open and pass.

## Notes

The hardest seam is **policy-as-data** (no `if currency == Coins`). Get `Currency` + policy right
here; everything above (api/UI) is mechanical. Blocks [[SQUIRE-T-0098]].

## Findings (store layer → migration shape)

Events live in ONE wide `events` table with generic nullable columns (`points`, `amount`, `reason`,
…) keyed by a `kind` string (`EventRow::from_event`/`to_event` in `crates/store/src/rows.rs`). Money
events map: `CompletionApproved`/`ItemRedeemed`/`AchievementUnlocked` → `points` col; `PointsAdjusted`
→ `amount`+`reason`. **This makes the migration additive + safe:** add a `currency` column (default
`'Coins'`) → every existing row is implicitly Coins → **no balance moves** by construction.

## Progress

- **Step 1 DONE (committed):** `Currency` enum (`Coins`, `Cash`) + `CurrencyPolicy` (policy-as-data:
  spendable_in_shop / supports_payout / earns_achievement_bonus / floors_at_zero / name / symbol) in
  `contract/primitives.rs`. Pure addition; `cargo build -p domain-core` green; nothing else touched.
- **Design lock:** keep `Proj::balance(snap, squire)` returning today's value exactly, defined as
  `balance_in(snap, squire, Coins)` — equivalence by construction, all existing tests/callers untouched.
- **Next:** add `balance_in(snap, squire, currency)`; then a migration-equivalence test (replay log →
  per-Squire balances golden) BEFORE generalizing `CompletionApproved` (awarded map) +
  `PointsAdjusted`→`Adjusted{currency}`.