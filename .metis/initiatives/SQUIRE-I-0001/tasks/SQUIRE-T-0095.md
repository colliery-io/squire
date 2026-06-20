---
id: real-money-rewards-on-chores-dual
level: task
title: "Real-money rewards on chores (dual currency: coins + actual money)"
short_code: "SQUIRE-T-0095"
created_at: 2026-06-20T02:05:00.755604+00:00
updated_at: 2026-06-20T03:09:10.124901+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: true
initiative_id: SQUIRE-I-0001
---

# Real-money rewards on chores (dual currency)

## Request (operator, 2026-06-19)

"Mom would like to add additional currency (actual money) to chores." Some chores should also accrue
**real money** the parent owes the child — alongside the existing in-app **coins**.

## Why this is a design task, not a quick edit

Coins today are a single `Points` (u32) currency, earned from quests and spent on rewards — a closed
loop. Real money is a *second, open* currency: it's a ledger of what the parent owes, it gets **paid
out** (settled) in cash, and it must never be spendable on in-app rewards. So it needs its own
balance, its own earn events, and a **settle/payout** flow — it is not just "another number on a
quest."

## Open design questions (decide before building)

- **Per-quest money:** does a quest carry an optional `money` amount (cents) in addition to `reward`
  (coins)? Both on completion, or pick one per quest?
- **Money ledger:** a separate balance + history (earned / paid-out), shown to parent and child.
- **Settle/payout:** a parent action "Paid $X" that zeroes/decrements the owed balance (audited).
- **Child view:** show "$ owed" distinct from coins (different visual — we just unified coins, so
  money must read as clearly different, e.g. a banknote vs the coin).
- **Rounding/units:** store integer cents; never floats.
- **Rewards:** money is NOT spendable in the in-app shop (keep the loops separate), unless explicitly
  wanted.

## Likely scope

Domain (`Money` type, quest field, `MoneyEarned` / `MoneyPaidOut` events + commands), projections
(owed balance + history), api/SDK (state view + admin), Keep + phone authoring (money on a quest),
parent payout UI, child display. A real feature — warrants its own initiative if it grows.

## Decisions (operator-approved, 2026-06-19)

- **Per-chore cash** in addition to coins (a quest can carry an optional money amount).
- A **"paid out" settle action** for the parent that draws down the owed balance (audited).
- **Kept separate from coins** — money is its own ledger, NOT spendable in the in-app shop.
- Store **whole dollars** (operator preference, 2026-06-19) — integer dollars, no cents, never floats.
  Show "$ owed" with a clearly non-coin visual.

## Build plan (operator: "EXACT same flow as coins — 'dollars'", auto-accrue on chores + manual grants)

Mirror the coin system as a parallel **cash** currency (whole dollars, u32). Decisions locked:
auto-accrue on chore approval (when a quest has cash set) **and** manual grants; payout = a negative
cash adjust.

- **Migration** (additive, safe): `quests.cash` + `events.cash` columns (default 0).
- **Domain:** `Quest.cash`; `CompletionApproved` gains `cash` (snapshotted at approval like `points`);
  new `CashAdjusted` event (reuses the generic `amount`/`reason` event columns — only `kind` differs)
  + `AdjustCash` command (parent grant / payout); `Proj::cash_balance` = Σ approved `cash` + Σ cash
  adjusts (floored at 0).
- **Store:** `QuestRow.cash`; event row mapping for `CompletionApproved.cash` + `CashAdjusted`.
- **API/SDK:** `StateView.cash_balance` + cash adjustments in activity; `CreateQuestReq.cash`;
  `POST /admin/adjust-cash`; regenerate.
- **Keep + phone:** quest authoring "$ (dollars)" field; cash balance display (banknote, not coin);
  grant/payout cash; child sees "$ owed".

Building deliberately layer-by-layer; the migration touches the LIVE prod DB (the kids' balances), so
it ships only once the whole thing is solid + tested — not piecemeal.

## DIRECTION CHANGE (operator, 2026-06-19): generalize, don't parallel

Operator foresees more currencies → build a generalized **Currency** model now, not a one-off `cash`
parallel. Coins get folded in; dollars is the second entry; a third is just config.

This turns the work from an *additive feature* into a **currency-system refactor** of the working coin
code + a **data migration of the kids' live balances** — the riskiest class of change. So it needs a
real design pass (likely an **ADR**) before any code:

- **`Currency`** identity + per-currency **policy** (coins: spendable-in-shop, prices rewards;
  dollars: not spendable, has *payout*; floor-at-zero for both) — the policy is the crux, since the
  currencies are NOT symmetric.
- Generic shapes: `Quest.rewards: {currency → amount}`; `CompletionApproved` snapshots an awarded map;
  one `Adjusted {currency, amount, reason}` (today's `PointsAdjusted` becomes `Adjusted{Coins}`);
  `ItemRedeemed.cost` stays Coins (rewards priced in coins); `balance(squire, currency)`.
- **Migration of existing data:** every current coin event/field must map to `Coins` without changing
  any balance. Design the store shape (currency column / rewards table) + a tested migration.
- api/SDK/Keep/phone updated to the currency-keyed model.

## Status

**ADR done + decided:** [[SQUIRE-A-0013]] (generalized multi-currency model, operator-approved).
Decomposed into the build chain:
- [[SQUIRE-T-0097]] — Currency model foundation (domain + store + zero-balance migration). ← start here
- [[SQUIRE-T-0098]] — Currency in api/SDK.
- [[SQUIRE-T-0099]] — Dollars end-to-end on the model (realizes this task).

This task (real-money) is realized by T-0099; close it when that ships. Build is a focused, well-tested
pass — it rewrites working money code on the kids' live balances.
## Realized

Delivered by the currency model build [[SQUIRE-A-0013]] → [[SQUIRE-T-0097]]/[[SQUIRE-T-0098]]/[[SQUIRE-T-0099]]. Dollars ship as currency #2 (whole dollars, per-chore accrual + payout). **Closed.**
