---
id: domain-core-claim-review-subject
level: task
title: "Domain Core: claim & review (subject, Race, snapshot, actor, idempotency)"
short_code: "SQUIRE-T-0003"
created_at: 2026-06-17T03:01:56.031141+00:00
updated_at: 2026-06-17T03:34:16.079765+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Domain Core: claim & review (subject, Race, snapshot, actor, idempotency)

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0001]] (Domain Core)

## Objective

Implement claim submission + review end-to-end: per-Squire subject validation, assignment gating, auto-approve, snapshot-at-approval, single-review, actor stamping, claim idempotency, and the `Race` occurrence-resolution rule. This is the most complex task in the initiative.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] `SubmitClaim` requires an active quest + an **assignee** Squire; emits `CompletionClaimed{squire}` (zero balance); idempotent on `claim_id` (replay = no-op); unknown/non-Squire → `UserNotFound`/`NotASquire`, non-assignee → `NotAssigned`. *(Design note: scheduling/due-ness is NOT enforced at submit — claims are zero-value until approved, so a wrong-day claim is a safe, reviewable pending item; due-ness gates `quests_due` (T-0004). Recorded in spec rationale.)*
- [x] `repeatable_within_day=false` ⇒ a second claim for the same `(squire,quest,on)` → `AlreadyClaimedToday` (per-Squire; two different Squires may each claim same quest/day); `true` ⇒ independent repeats.
- [x] `auto_approve` ⇒ `CompletionClaimed` then immediate `CompletionApproved` with the quest's **current reward snapshotted**, `actor=None`.
- [x] `ReviewClaim(Approve)` ⇒ `CompletionApproved` snapshotting current reward + `actor=Some(knight)`; `(Reject)` ⇒ `CompletionRejected{reason, actor}`; second review → `AlreadyReviewed`; unknown → `ClaimNotFound`.
- [x] `Race`: occurrence stays OPEN until the first **approved** completion (closes it; sole payout); multiple assignees may hold pending claims while open; approving/submitting against a closed occurrence → `OccurrenceTaken`; a **rejected** first claim reopens it; `EachAssignee` ⇒ independent occurrences.
- [x] Snapshot-at-approval is immutable under later reward edits (AC-7, tested). *(Streak/achievement use-of-`on`: `CompletionApproved` carries `claim_id`+`squire`; `on` is joined via the claim — consumed in T-0006.)*

## Implementation Notes

### Technical Approach
Derive occurrence open/closed from the log; stamp `squire`+`actor` onto emitted events; snapshot reward at approval time.

### Requirements covered
REQ-1.1.3, REQ-1.1.4, REQ-1.1.5, REQ-1.2.1, REQ-1.2.1a, REQ-1.2.1b, REQ-1.2.2, REQ-1.2.3, REQ-1.2.4, REQ-1.2.5, REQ-1.2.6, REQ-1.5.3 (approve/reject actor).

### Dependencies
T-0001 (uses fixture snapshots; realistic flows pair with T-0002). Achievement-unlock-on-approval is implemented in T-0006 and hooks into this approval path — coordinate.

## Status Updates

**2026-06-16 — Completed.** Implemented `crate::claims::handle` (`src/claims.rs`): `submit` (idempotent on `claim_id`; validates active-Squire + assignee + active quest; dup/Race gating; emits `CompletionClaimed{squire}`, plus an auto-approve `CompletionApproved{actor:None}` snapshotting the reward) and `review` (`claim_meta` lookup → `ClaimNotFound`; `AlreadyReviewed` guard; Approve snapshots current reward + `actor:Some(knight)`, with Race `OccurrenceTaken` if already closed; Reject emits `CompletionRejected{actor}`). Added log-query + cadence helpers to `src/common.rs` (`claim_meta`, `claim_resolution`, `occurrence_closed`, `squire_satisfied`, `squire_has_live_claim`, `weekday_of`/`schedule_matches`/`cadence_matches` — the last four are for T-0004, currently dead-code-warned). `approval_events` is the seam T-0006 extends to append `AchievementUnlocked`.

**Design decision (documented):** scheduling/due-ness is not enforced at `SubmitClaim` — claims are zero-value until approved, so the trust model holds; due-ness gates `quests_due` (T-0004). Avoids duplicating cadence logic and a `NotDue` error.

Tests: `tests/claims.rs` (12) — claim/idempotency, subject validation, per-Squire AlreadyClaimedToday, repeatable, auto-approve, approve/reject/AlreadyReviewed/ClaimNotFound, snapshot-at-approval immutability (AC-7), Race first-approved-wins + reject-reopens. Full suite **29 passed**. Committed.