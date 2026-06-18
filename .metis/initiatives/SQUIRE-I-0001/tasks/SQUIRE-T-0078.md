---
id: refine-the-knight-review
level: task
title: "Refine the Knight review experience on the phone (reject-reason capture + triage polish)"
short_code: "SQUIRE-T-0078"
created_at: 2026-06-18T22:52:51.352484+00:00
updated_at: 2026-06-18T22:56:07.619068+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Refine the Knight review experience on the phone (reject-reason capture + triage polish)

## Parent Initiative

[[SQUIRE-I-0001]]

## Objective

Close the loop opened by T-0076/0077: let a parent **explain a rejection from the phone**. Today the phone's Reject buttons fire immediately with a `null` reason (`KnightHomeHost.kt:192,194`), so a reason can only be set from the Keep — even though the whole reason→child chain is wired and tested, and "Add ★" already prompts for a reason. This adds a reject-reason dialog for both claims and reward requests, matching the existing `AddFundsDialog`.

## Define-First: the flows we expect

### Flow K1 — Reject a quest claim with an optional reason
- Tapping **Reject** on a pending claim opens a dialog naming the quest + child, with an optional "Reason (the child will see this)" field and **Reject** / **Cancel**.
- Confirm → `viewModel.rejectClaim(claimId, reason?.ifBlank{null})`. The reason flows to `Decision::Reject{reason}` → the child's "Recent" shows `Rejected: <reason>` (proven in T-0076).
- A blank reason is allowed (rejects with no note) — parity with the engine, which treats reason as optional.

### Flow K2 — Reject a reward request with an optional reason
- Same dialog for a pending redemption request (names the item + child) → `viewModel.rejectRequest(requestId, reason?)`.

### Flow K3 — No regression to the rest of triage
- Approve, Add ★ (reason-required), Redeem, Mark-done, Open/assume unchanged.

## Current State

- `KnightViewModel.rejectClaim(id, reason)` / `rejectRequest(id, reason)` and `KnightStore` **already accept a reason** — only the UI drops it (passes `null`). So this is a **UI-only** change.
- `AddFundsDialog` is the dialog pattern to mirror (text field + confirm/cancel).

- [x] `KnightHomeScreen` `onRejectClaim` / `onRejectRequest` carry an optional reason (`(Long, String?) -> Unit`); the Reject buttons open a reason dialog instead of firing immediately.
- [x] A `RejectReasonDialog` (optional reason, Reject/Cancel) is shown for both claims and requests; confirm threads `reason.trim().ifBlank{null}` through to the ViewModel.
- [x] `KnightHomeHost` passes the captured reason (`{ id, reason -> viewModel.rejectClaim(id, reason) }`), no longer hard-coded `null`.
- [x] Paparazzi `knightRejectReasonDialog` snapshot — image-validated.
- [x] `:app:assembleDebug` + `:app:verifyPaparazziDebug` green.

## Implementation Notes

- Add `rejectClaimFor`/`rejectRequestFor` state in `ReadyContent` (like `fundsFor`); render a shared `RejectReasonDialog`. Make it `internal` so the screenshot harness can snapshot it.
- No backend / SDK change.

## Dependencies
- Directly closes T-0076 / T-0077 (the reason→child path those hardened).

## Status Updates

**2026-06-18 — Done.**
- Confirmed UI-only: `KnightViewModel`/`KnightStore` already accept a reason; the screen passed `null`.
- `KnightHomeScreen`: `onRejectClaim`/`onRejectRequest` now `(Long, String?)`; Reject buttons set `rejectClaimFor`/`rejectRequestFor` state; added an `internal RejectReasonDialog` (subject line + optional "Reason (the child will see this)" field + red Reject / Cancel). Fixed the two existing call sites (screenshot + `@Preview`) to the new arity.
- `KnightHomeHost`: threads the captured reason to `viewModel.rejectClaim/rejectRequest`.
- Paparazzi `knightRejectReasonDialog` recorded + image-validated (title "Reject quest", subject "Walk the dog · Gawain", reason field, Cancel/Reject). `verifyPaparazziDebug` green — the review home itself is visually unchanged.
- This makes the reason→child chain hardened in T-0076/0077 reachable from the phone, not just the Keep.