---
id: currency-in-api-sdk-per-currency
level: task
title: "Currency in api/SDK: per-currency balances, adjust-by-currency, quest rewards map"
short_code: "SQUIRE-T-0098"
created_at: 2026-06-20T03:26:26.411633+00:00
updated_at: 2026-06-20T13:03:31.877751+00:00
parent: SQUIRE-I-0001
blocked_by: [SQUIRE-T-0097]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: true
initiative_id: SQUIRE-I-0001
---

# Currency in api/SDK

Expose the [[SQUIRE-A-0013]] model across the trust boundary, on top of [[SQUIRE-T-0097]].

## Scope

- **`StateView`:** `balance: Points` → `balances: Map<Currency, i64>` (or a `CurrencyBalance` list);
  keep a back-compat shim if needed. Adjustments in activity already carry the amount/reason — add the
  `currency`.
- **Quest DTOs:** `CreateQuestReq.reward` → per-currency `rewards`; the card surfaces what a chore pays
  in each currency.
- **Adjust endpoint:** `POST /admin/adjust` gains a `currency` (default `Coins` for back-compat);
  payout is `currency=Cash, amount<0` (server enforces `supports_payout`).
- **Currency catalog:** a `GET /currencies` (or fold into config) so clients render the right
  symbol/policy per currency the household uses.
- Regenerate `openapi.json` + the Kotlin SDK; keep the frozen-spec guard green.

## Acceptance

- [x] api tests green (40 groups); openapi.json regenerated (Currency, CurrencyBalance + currency
  field); Kotlin SDK regenerates + the app + screenshot tests compile.
- [x] Coins-only payloads/behaviour unchanged: `balance` stays the coin balance; `balances` +
  `AdjustReq.currency` are `#[serde(default)]` so older clients are unaffected (verified by the app +
  test compile and all balance assertions passing).

## Done

- `Command::AdjustPoints { currency, .. }` threaded through engine + api `/admin/adjust` + Keep
  `/api/adjust`.
- `StateView.balances: Vec<CurrencyBalance>` (currency, floored balance, name, symbol); Coins always
  present, dollars listed too.
- openapi + SDK regenerated; committed `4595a1f`.

**T-0098 complete.** Unblocks [[SQUIRE-T-0099]] (dollars end-to-end UI + Quest cash + payout).