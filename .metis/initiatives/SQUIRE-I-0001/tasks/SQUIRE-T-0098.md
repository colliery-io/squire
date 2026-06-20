---
id: currency-in-api-sdk-per-currency
level: task
title: "Currency in api/SDK: per-currency balances, adjust-by-currency, quest rewards map"
short_code: "SQUIRE-T-0098"
created_at: 2026-06-20T03:26:26.411633+00:00
updated_at: 2026-06-20T03:26:26.411633+00:00
parent: SQUIRE-I-0001
blocked_by: ["SQUIRE-T-0097"]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
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

- [ ] api tests green; openapi regenerated; SDK builds.
- [ ] Coins-only payloads/behaviour unchanged for existing clients (back-compat verified).

Blocks [[SQUIRE-T-0099]].
