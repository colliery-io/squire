---
id: parity-cash-payout-settle-action
level: task
title: "Parity: cash payout / settle action in the Android Knight app"
short_code: "SQUIRE-T-0111"
created_at: 2026-06-21T23:05:27.496972+00:00
updated_at: 2026-06-21T23:05:27.496972+00:00
parent: SQUIRE-I-0004
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: true
initiative_id: SQUIRE-I-0004
---

# Parity: cash payout / settle on Android Knight

Close the biggest parity gap ([[SQUIRE-I-0004]]): the Keep can settle real-money cash owed ("Pay") but
the Android Knight app only *shows* the cash-owed figure and points to the Keep.

## Backend
**Already exists** — `POST /admin/adjust` accepts `currency: Cash` (and the null-currency fix
[[SQUIRE-T-0109]] makes it robust). A payout = `Adjusted{Cash, -amount, reason}` that draws down the
owed balance (floored at 0). No new endpoint.

## Scope (Android Knight)
- On each Squire card showing `cashBalance > 0`, add a **"Pay"** action (mirrors "Add coins"): amount +
  required reason → `KnightCommand` → adjust with `currency = Cash`, negative amount.
- Reuse the existing `AddFundsDialog` pattern (amount + reason, non-blank validation); label it for cash
  ($) and cap the amount at the owed balance.
- Optimistic update + outbox, same as the coin adjust.

## Acceptance
- [ ] Knight can settle cash from the phone; owed balance drops; an `Adjusted{Cash,-}` event lands.
- [ ] Mirrors the Keep's payout semantics (floored at 0, reason required).
- [ ] Gherkin scenario added ([[SQUIRE-T-0115]]): Given a squire owed $X, When the knight pays $Y,
  Then owed = X−Y.
