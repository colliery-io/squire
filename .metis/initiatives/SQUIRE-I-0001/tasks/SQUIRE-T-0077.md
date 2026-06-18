---
id: harden-the-reward-request-redeem
level: task
title: "Harden the reward request/redeem loop (reject + blocked paths, reasons-to-child, edge-case tests)"
short_code: "SQUIRE-T-0077"
created_at: 2026-06-18T22:32:10.505382+00:00
updated_at: 2026-06-18T22:43:54.959295+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Harden the reward request/redeem loop (reject + blocked paths, reasons-to-child, edge-case tests)

## Parent Initiative

[[SQUIRE-I-0001]]

## Objective

Lock in the behavioral contract of the reward **request → review → debit** loop (and the Knight direct-redeem) with tests at the layers that lack them, and prove the child sees *why* a request was rejected. Like T-0076, this is **mostly tests + visual**, not new behavior.

## Define-First: the contract this loop must honor

A child requests a reward → a pending **redemption request** → a Knight approves or rejects:
1. **Request** reserves nothing (no points held while pending); always allowed for an existing item; idempotent on `request_id`.
2. **Approve** debits the cost **re-checked at approval** — it can fail at commit with `Redeem(Blocked)`: `InsufficientPoints`, `AchievementLocked`, `OutOfStock` (Once already redeemed / inactive). On success → `ItemRedeemed` (cost snapshotted), balance drops.
3. **Reject** debits nothing, optionally carries a **reason**.
4. **Idempotency**: `RedeemItem`/`AdjustPoints` idempotent on `command_id`; a second review of a resolved request is `AlreadyReviewed` (409). `Redeem(Blocked)` → 409.
5. **The child sees the outcome**: `GET /state` → `my_requests` carries `Rejected {reason}` / resolved, and `PlayerHomeScreen` "Recent" renders it (incl. reason). Reward cards already show `OutOfStock` / `NeedsAchievement` / not-affordable locks.

## Current Coverage (from the map)

- **Domain** (`tests/redemption.rs`): thorough — per-squire balance, request→approve→ItemRedeemed + double-approve=AlreadyReviewed, **reject spends nothing** + RequestNotFound, direct-redeem idempotent, adjust idempotent+reason, insufficient-points block, Once out-of-stock, gated locked-until-unlock, affordability-rechecked-at-approval. **Gaps**: double-**reject** → AlreadyReviewed; a gate that unlocks *between* request and approval then approves successfully.
- **API** (`tests/knight.rs`,`squire.rs`): request=Pending+shows-in-state, direct-redeem idempotent, adjust-reason. **Gap**: the **review-redemption** path entirely — approve (debits, reflected in balance) and **reject with reason → child `GET /state` `my_requests` shows `Rejected {reason}`**; and redeem **blocked → 409** (insufficient / out-of-stock / gated) at the HTTP layer.
- **Keep** (`tests/review.rs`): queue, direct-redeem, adjust, affordability-at-approval. **Gap**: the redemption **reject** path over the HTTP surface.
- **Phone**: `PlayerHomeScreen` already renders `my_requests` rejection reasons + reward lock chips — **no snapshot exercises a rejected request**.

## Acceptance Criteria

- [x] Domain (`tests/redemption.rs`, +2): double-reject of a request → `AlreadyReviewed`; a request for a gated item that unlocks before approval then approves and debits (a blocked approval debits nothing). 12 pass.
- [x] API (`tests/knight.rs`, +5): request → approve debits balance and `my_requests` shows Approved; request → reject with reason → child `GET /state` `my_requests` shows `Rejected {reason}` (balance unchanged); direct-redeem blocked by insufficient funds → 409, by Once-out-of-stock → 409, by an unmet gate → 409 (the last two authored via `/admin/items` + `/admin/achievements`). 25 pass.
- [x] Keep (`tests/review.rs`, +1): rejecting a redemption request over `/api/review/redemption` spends nothing and resolves the request. 8 pass.
- [x] Paparazzi: extended `squirePlayerHomeHistory` to include a rejected request — the child "Recent" shows "Movie night · 25★ · Rejected: After homework".
- [x] All touched suites green (`cargo test -p domain-core`, `-p api`, `-p keep`; `:app:verifyPaparazziDebug`).

## Implementation Notes

- Reuse `redemption.rs` helpers (`run/request/redeem/adjust/bal/seed_points`) and the `knight.rs`/`review.rs` harnesses. The reason path is already wired domain→api→UI; only add code if a test exposes a real gap.

## Dependencies
- Sibling of T-0076; shares the child "Recent" snapshot.

## Status Updates

**2026-06-18 — Done.**
- Domain (`tests/redemption.rs`, +2): `double_reject_is_already_reviewed`; `gated_request_unlocks_before_approval_then_debits` (request allowed while locked, approval blocked → `AchievementLocked` debits nothing, then unlock → approve debits). 12 pass.
- API (`tests/knight.rs`, +5): `redemption_request_approve_debits_and_shows_approved`; `redemption_reject_with_reason_surfaces_to_child` (child `GET /state` `my_requests` → `Rejected{reason="Maybe next week"}`, balance unchanged); `direct_redeem_insufficient_funds_is_409`; `direct_redeem_once_item_out_of_stock_is_409`; `direct_redeem_gated_item_is_409`. The Once/gated cases author their fixtures via the T-0074/0072 `/admin/items` + `/admin/achievements` endpoints (nice cross-feature integration). 25 pass.
- Keep (`tests/review.rs`, +1): `reject_redemption_with_reason_does_not_credit_and_resolves`. 8 pass.
- Paparazzi: extended `squirePlayerHomeHistory` with a rejected `myRequests` entry → "Movie night · 25★ · Rejected: After homework" renders in the child "Recent". `verifyPaparazziDebug` green.
- **No production code changed** — like T-0076, the reject/blocked paths were already wired domain→api→UI; this locks them against regression. Confirmed `redemption_state`/`my_requests` keep resolved requests with their computed `Rejected{reason}`/`Approved` state.