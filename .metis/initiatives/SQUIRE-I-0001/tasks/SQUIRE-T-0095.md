---
id: real-money-rewards-on-chores-dual
level: task
title: "Real-money rewards on chores (dual currency: coins + actual money)"
short_code: "SQUIRE-T-0095"
created_at: 2026-06-20T02:05:00.755604+00:00
updated_at: 2026-06-20T02:05:00.755604+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
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
- Store integer **cents**; never floats. Show "$ owed" with a clearly non-coin visual.

## Status

Direction approved. Next: a short written design (events/commands, projections, settle flow, UI on
both surfaces) for review before building. Larger than [[SQUIRE-T-0096]]; likely sequence hazards
first, then this.
