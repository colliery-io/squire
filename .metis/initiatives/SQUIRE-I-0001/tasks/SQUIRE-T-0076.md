---
id: harden-the-quest-complete-accept
level: task
title: "Harden the quest complete/accept loop (reject paths, reasons-to-child, edge-case tests)"
short_code: "SQUIRE-T-0076"
created_at: 2026-06-18T22:32:09.126886+00:00
updated_at: 2026-06-18T22:38:35.760243+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Harden the quest complete/accept loop (reject paths, reasons-to-child, edge-case tests)

## Parent Initiative

[[SQUIRE-I-0001]]

## Objective

Lock in the behavioral contract of the quest **complete → review → credit** loop with tests at the layers that currently lack them, and prove visually that a child sees *why* a claim was rejected. The loop works today; this task hardens it against regression — it is **mostly tests + one visual snapshot**, not new behavior.

## Define-First: the contract this loop must honor

A child marks a quest done → a pending **claim** → a Knight approves or rejects:
1. **Approve** credits the quest's reward (snapshotted at approval, immutable under later edits) and may unlock achievements (+bonus to balance). Actor = the Knight. Auto-approve quests skip review (actor = none).
2. **Reject** credits nothing, optionally carries a **reason**, and re-opens the slot so the child can claim again.
3. **Idempotency**: replaying a claim id is a no-op; a second review of a resolved claim is `AlreadyReviewed` (409). The Knight outbox treats 409/403 as "already done".
4. **Race** quests: first approval wins and closes the occurrence (others → `OccurrenceTaken`); a reject re-opens it for another assignee.
5. **The child sees the outcome**: `GET /state` → `my_claims` carries `Approved {points}` / `Rejected {reason}` / `Pending`, and `PlayerHomeScreen`'s "Recent" renders it (incl. the reason).

## Current Coverage (from the map)

- **Domain** (`tests/claims.rs`): thorough — idempotency, AlreadyClaimedToday, auto-approve, snapshot-at-approval, double-review→AlreadyReviewed, reject-lets-reclaim, Race first-wins, Race reject-reopens-then-approve-B. **Gaps**: approval that *completes an achievement criterion* emitting the unlock+bonus (only tested in `achievements.rs`, not via the claim→approve path); the full reject→reclaim→**approve credits** cycle.
- **API** (`tests/squire.rs`,`knight.rs`): submit=Pending+idempotent, approve credits balance + double-approve=409, mark-done. **Gap**: the end-to-end **reject with reason → child `GET /state` shows `Rejected {reason}`**, and approve → `Approved {points}` surfaces in the child's state.
- **Keep** (`tests/review.rs`): approve credits, reject-with-reason no-credit — covered.
- **Phone**: `PlayerHomeScreen` already renders rejected/approved/pending claims (incl. reason) — but **no snapshot exercises it** (the existing golden seeds empty history).

## Acceptance Criteria

- [x] Domain (`tests/claims.rs`): a claim whose approval completes an achievement criterion emits `AchievementUnlocked` and the bonus lands on balance (claim→approve path); and a reject→reclaim→approve cycle credits exactly once on the second claim.
- [x] API (`tests/squire.rs`): a Knight reject with a reason makes the child's `GET /state` `my_claims` show `Rejected` carrying that reason (balance 0); a re-claim then approve shows `Approved {points}` and the credited balance.
- [x] Paparazzi `squirePlayerHomeHistory` snapshot: the child "Recent" section shows a rejected claim (with reason "Bowl wasn't refilled"), an approved claim (+10★), and a pending one — visual proof the rejection reason reaches the child.
- [x] All touched suites green (`cargo test -p domain-core` claims 14, `-p api` squire 6; `:app:verifyPaparazziDebug`).

## Implementation Notes

- No production code change expected unless a test surfaces a real gap (the reason path is already wired domain→api→UI). Reuse the `claims.rs` `run/submit/approve/reject/approved` helpers and the `knight.rs`/`squire.rs` oneshot harness.
- Snapshot: add a `StateView` seed with `myClaims` covering all three `ClaimStateKind`s to `ScreenshotTests.kt`.

## Dependencies
- Sibling of T-0077 (reward request/redeem loop), which shares the child "Recent" rendering.

## Status Updates

**2026-06-18 — Done.**
- Verified the map: domain claims coverage was already strong (incl. Race reject-A→approve-B at `claims.rs:218`); the real gaps were the achievement-unlock-via-claim path, the full reject→reclaim→approve cycle, the HTTP-boundary reject-reason-to-child, and zero visual proof.
- Domain (`tests/claims.rs`, +2): `approving_a_claim_that_completes_a_criterion_unlocks_and_credits_bonus` (reward 10 + bonus 50 = 60, `AchievementUnlocked` emitted); `reject_then_reclaim_then_approve_credits_once` (rejected credits 0, re-claim approved credits exactly once). 14 pass.
- API (`tests/squire.rs`, +1): `knight_reject_with_reason_surfaces_to_child_then_reclaim_approves` — child `GET /state` shows `Rejected{reason="Make the bed first"}`, balance 0; re-claim+approve → `Approved{points:5}`, balance 5. (Note: review-claim returns 200, not 204.) 6 pass.
- Paparazzi `squirePlayerHomeHistory` recorded + image-validated: "Recent" shows ✓ Approved (+10★) / ✗ Rejected: Bowl wasn't refilled / ⏳ Pending. The child sees *why* a claim was rejected.
- **No production code changed** — the reject-reason path was already wired domain→api→UI; this task locks it against regression. `verifyPaparazziDebug` green.