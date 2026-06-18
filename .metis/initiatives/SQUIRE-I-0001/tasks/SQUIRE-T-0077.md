---
id: harden-the-reward-request-redeem
level: task
title: "Harden the reward request/redeem loop (reject + blocked paths, reasons-to-child, edge-case tests)"
short_code: "SQUIRE-T-0077"
created_at: 2026-06-18T22:32:10.505382+00:00
updated_at: 2026-06-18T22:32:10.505382+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


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

- [ ] Domain (`tests/redemption.rs`): double-reject of a request → `AlreadyReviewed`; a request for a gated item that unlocks before approval then approves and debits.
- [ ] API (`tests/knight.rs`): request → approve debits balance (and the request leaves `my_requests` pending); request → reject with reason → child `GET /state` `my_requests` shows `Rejected {reason}`; direct-redeem blocked by insufficient funds → 409, by Once-out-of-stock → 409, by an unmet gate → 409.
- [ ] Keep (`tests/review.rs`): rejecting a redemption request over `/api/review/redemption` spends nothing and resolves the request.
- [ ] Paparazzi: the child "Recent" shows a rejected request (with reason) — extend the T-0076 `squirePlayerHomeHistory` seed (or a dedicated snapshot) to include `myRequests`.
- [ ] All touched suites green (`cargo test -p domain-core`, `-p api`, `-p keep`; `:app:verifyPaparazziDebug`).

## Implementation Notes

- Reuse `redemption.rs` helpers (`run/request/redeem/adjust/bal/seed_points`) and the `knight.rs`/`review.rs` harnesses. The reason path is already wired domain→api→UI; only add code if a test exposes a real gap.

## Dependencies
- Sibling of T-0076; shares the child "Recent" snapshot.

## Status Updates

*To be added during implementation.*
