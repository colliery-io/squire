---
id: currency-model-foundation-generic
level: task
title: "Currency model foundation: generic Currency + policy, fold coins in, zero-balance migration"
short_code: "SQUIRE-T-0097"
created_at: 2026-06-20T03:26:20.263647+00:00
updated_at: 2026-06-20T03:26:20.263647+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
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

- [ ] **No balance moves:** for every Squire, `balance(squire, Coins)` under the new model ==
  today's `balance(squire)` exactly, replayed over the full log. A test asserts this over the live
  data dump (a copy of prod), and over the property/golden suites.
- [ ] All existing domain + api tests green (coin behavior unchanged).
- [ ] Migration is reversible (down.sql) and idempotent.

## Notes

The hardest seam is **policy-as-data** (no `if currency == Coins`). Get `Currency` + policy right
here; everything above (api/UI) is mechanical. Blocks [[SQUIRE-T-0098]].
