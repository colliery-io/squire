---
id: 001-generalized-multi-currency-model
level: adr
title: "Generalized multi-currency model (coins, dollars, and beyond)"
number: 13
short_code: "SQUIRE-A-0013"
created_at: 2026-06-20T03:20:14.491216+00:00
updated_at: 2026-06-20T03:20:14.491216+00:00
decision_date: 2026-06-19
decision_maker: Operator (Dylan)
parent: 
archived: false

tags:
  - "#adr"
  - "#phase/draft"


exit_criteria_met: false
initiative_id: NULL
---

# ADR-13: Generalized multi-currency model (coins, dollars, and beyond)

## Context **[REQUIRED]**

Squire today has exactly one currency — **coins** — hard-coded as `Points` (`u32`) throughout the
domain:

- `Quest.reward: Points` — what a chore awards.
- `Event::CompletionApproved { points }` — coins snapshotted at approval.
- `Event::AchievementUnlocked { bonus }` — coins from a badge.
- `Event::PointsAdjusted { amount, reason }` — a parent grant / correction (can go ±).
- `Event::ItemRedeemed { cost }` — coins spent in the reward shop.
- `Proj::balance(snap, squire)` — the per-Squire sum over the log, floored at zero.

The operator wants **real-money "dollars" on chores** (SQUIRE-T-0095) **and has confirmed more
currency types are coming** (e.g. screen-time tokens, charity points). Two structural facts shape this:

1. **Adding each currency as a hard-coded parallel (`reward`+`cash`, `points`+`cash`,
   `PointsAdjusted`+`CashAdjusted`, `balance`+`cash_balance`) is O(N) duplication** — it does not
   scale to "and beyond."
2. **The currencies are NOT symmetric.** Coins are *spent in the shop* and *price rewards*; dollars
   are a real-world **IOU** that is *paid out / settled* and is **never spendable in-shop**. A naive
   "same mechanics, different key" map would immediately sprout `if currency == Coins` branches.

So the question is not just "add a second number" — it is whether to **generalize the currency model**
now, with first-class per-currency *policy*, before there are three hard-coded copies to untangle.

## Decision **[REQUIRED]**

Introduce a **generalized, currency-keyed model** with explicit **per-currency policy**. Coins are
folded in as the first currency; dollars are the second; a third is a new `Currency` entry + a policy
row + UI — no engine changes.

### The model

- **`Currency`** — a stable identity with a serde-stable tag: `Coins`, `Cash` (dollars), … Each
  currency carries **policy as data, not branches**:
  | policy | Coins | Cash (dollars) |
  |---|---|---|
  | display (name / symbol / icon) | "coins" / 🪙 | "dollars" / $ (banknote) |
  | `spendable_in_shop` (prices + buys rewards) | **yes** | no |
  | `supports_payout` (settle to zero) | no | **yes** |
  | `earns_achievement_bonus` | yes | no (start) |
  | `floors_at_zero` | yes | yes |
  | unit | whole | **whole dollars** (no cents) |

- **Awards keyed by currency.** `Quest.reward: Points` → `Quest.rewards: Map<Currency, u32>` (a chore
  can pay coins, dollars, both, or — degenerate — nothing). `CompletionApproved` snapshots the
  **awarded map** at approval time (same snapshot guarantee coins have today).

- **One generic adjust event.** `Event::PointsAdjusted` → `Event::Adjusted { currency, amount: i64,
  reason, … }`. A parent grant is `Adjusted{Coins,+}`; a dollar grant `Adjusted{Cash,+}`; a **payout
  is just `Adjusted{Cash, −}`** (allowed only where `supports_payout`).

- **Spend stays shop-scoped.** `ItemRedeemed.cost` is priced in the **shop currency** (Coins) per
  policy; redemption only ever touches `spendable_in_shop` currencies. Dollars never enter the shop.

- **`balance(snap, squire, currency)`** — the per-currency sum over the log, floored per policy. The
  child home shows one balance pill per currency the household uses; "$ owed" reads as money, not a
  coin.

### Acceptance invariant (the part we do not rush)

> **No existing balance may move.** For every Squire, replaying the full event log under the new model
> yields `balance(squire, Coins)` **exactly equal** to today's `balance(squire)` — byte-for-byte. This
> is the migration's pass/fail test, run against a copy of the live data before anything ships.

## Alternatives Analysis **[CONDITIONAL: Complex Decision]**

| Option | Pros | Cons | Risk | Cost |
|--------|------|------|------|------|
| **A. Parallel per-currency** (`cash` mirrors `coins`) | Additive (one new column); never touches the working coin code; simplest to ship dollars | O(N) duplication; a 3rd currency repeats it all; two near-identical code paths drift | Low | Low (but repeated) |
| **B. Generic seam, coins left legacy** | No live-coin refactor; new currencies are generic | Hybrid: coins special-cased forever; the asymmetry leaks anyway | Medium | Medium |
| **C. Generalized model, coins folded in** (CHOSEN) | Adding currency N = data + policy; one engine/projection; no drift | Refactors working money code; migrates the kids' live balances; richer types | **High** | High (one-time) |

## Rationale **[REQUIRED]**

The operator confirmed **more currencies are coming**, which is exactly the signal that flips the
"rule of three": the duplication of Option A is a known future tax, and a half-measure (Option B)
keeps coins special-cased forever while still having to model the asymmetry. Paying the refactor once,
**now, with two concrete currencies to extract the abstraction from** (coins = spendable/priced;
dollars = IOU/payout), yields the right policy seam and makes every future currency cheap. The cost —
rewriting money code on live data — is real, which is why this ADR exists and why the migration carries
a hard, testable "balances must not move" invariant.

## Consequences **[REQUIRED]**

### Positive
- Adding a currency becomes: one `Currency` entry + a policy row + UI strings. No engine/projection
  change.
- A single audited adjust path (`Adjusted{currency}`) covers grants, corrections, and payouts.
- The coin/dollar asymmetry is captured once, as policy data, instead of scattered conditionals.

### Negative
- A refactor of **working** money code (`Event`, projections, every coin reference) plus a **migration
  of the kids' live balances** — the highest-risk class of change. Mitigated by the acceptance
  invariant + a dry-run against a copy of prod.
- Richer types (per-currency maps) ripple into api/SDK/Keep/phone.
- Strictly more upfront work than shipping dollars parallel.

### Neutral
- Storage shape choice (an events `currency` column + a `quest_rewards` table, vs. reusing today's
  typed columns as the implicit `Coins` currency) is an implementation detail decided in the build
  task, constrained by the invariant.
- Achievement bonuses stay coins-only initially (`earns_achievement_bonus` policy) — trivially
  flipped later.

## Review Schedule **[CONDITIONAL: Temporary Decision]**

Permanent (a foundational model). Revisit only if a future currency cannot be expressed by the policy
table (i.e. needs behavior no flag captures) — that's the signal the policy set is missing a dimension,
not that the model is wrong.
