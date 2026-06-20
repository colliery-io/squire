---
id: dollars-end-to-end-on-the-currency
level: task
title: "Dollars end-to-end on the currency model: per-chore $, payout, Keep + phone UI, child $ owed"
short_code: "SQUIRE-T-0099"
created_at: 2026-06-20T03:26:30.190+00:00
updated_at: 2026-06-20T13:11:33.487895+00:00
parent: SQUIRE-I-0001
blocked_by: [SQUIRE-T-0098]
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Dollars end-to-end (the first new currency on the model)

Wire **Cash (whole dollars)** as currency #2 on top of [[SQUIRE-T-0098]] — realizing the original ask
in [[SQUIRE-T-0095]]. Mostly UI once the model exists (the engine is currency-agnostic).

## Scope

- **Register `Cash`** in the currency catalog with its policy (not spendable in-shop; supports payout;
  whole dollars; banknote glyph).
- **Quest authoring** (Keep + phone): a "$ (dollars)" amount alongside the coin reward.
- **Earn:** a chore with `Cash` set accrues dollars to the kid on approval (automatic — falls out of
  the awarded-map snapshot). Manual dollar grants via the adjust path.
- **Payout:** a parent "Paid out $X" action = `Adjusted{Cash, −}` (audited); guarded by
  `supports_payout`.
- **Display:** parent sees each kid's "$ owed"; child home shows a distinct **$** balance (banknote,
  not the coin) + dollar grants/payouts in the activity feed.
- Config editable on **both** phone and Keep (not phone-only).

## Acceptance

- [ ] Set $ on a chore → approve → kid's $ balance rises by that amount.
- [ ] "Paid out $X" reduces the owed balance; shows in activity; can't go below zero.
- [ ] Dollars never appear as spendable in the reward shop.
- [ ] Adding a *third* currency later needs no engine/projection change (the model's promise).

Realizes [[SQUIRE-T-0095]] (which can be closed once this ships).

## Progress

- **Domain + store DONE (committed `99298af`):** `Quest.cash` + `quests.cash` migration; approval
  emits a coupled `Adjusted{Cash, +cash}` (reason = quest title) so `balance_in(Cash)` accrues it;
  `CreateQuestReq.cash` → authoring; openapi regen; cash-accrual tests green (49 groups).
- **Payout:** no new endpoint — it's `/admin/adjust` (api) / `/api/adjust` (Keep) with `currency=Cash,
  amount<0` (both already take `currency` from T-0098). Floor-at-zero handled by the balance projection.
- **Remaining (UI):** Keep quest form `$` field + show `$` balances + payout; phone quest form `$` +
  child `$` balance display + dollar activity + parent payout. Then APK + server redeploy.