---
id: domain-core-claim-review-subject
level: task
title: "Domain Core: claim & review (subject, Race, snapshot, actor, idempotency)"
short_code: "SQUIRE-T-0003"
created_at: 2026-06-17T03:01:56.031141+00:00
updated_at: 2026-06-17T03:30:52.888950+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


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

- [ ] `SubmitClaim` requires an active quest due on `on` and an **assignee** squire; emits `CompletionClaimed{squire}` (zero balance); idempotent on `claim_id` (replay = no-op); unknown/non-Squire → `UserNotFound`/`NotASquire`, non-assignee → `NotAssigned`.
- [ ] `repeatable_within_day=false` ⇒ a second claim for the same `(squire,quest,on)` → `AlreadyClaimedToday` (per-Squire; two different Squires may each claim same quest/day); `true` ⇒ independent repeat payouts.
- [ ] `auto_approve` ⇒ `CompletionClaimed` then immediate `CompletionApproved` with the quest's **current reward snapshotted**, `actor=None`.
- [ ] `ReviewClaim(Approve)` ⇒ `CompletionApproved` snapshotting current reward + `actor=Some(knight)`; `(Reject)` ⇒ `CompletionRejected{reason, actor}`; second review → `AlreadyReviewed`; unknown → `ClaimNotFound`.
- [ ] `Race`: the `(quest,on)` occurrence stays OPEN until the first **approved** completion (which closes it & is the sole payout); multiple assignees may hold pending claims while open (a false claim must not lock out the real doer); approving/reviewing against a closed occurrence → `OccurrenceTaken`; a **rejected** first claim reopens it. `EachAssignee` ⇒ each assignee an independent occurrence.
- [ ] Streak/achievement-relevant dating uses the claim's `on`, never approval timestamp.

## Implementation Notes

### Technical Approach
Derive occurrence open/closed from the log; stamp `squire`+`actor` onto emitted events; snapshot reward at approval time.

### Requirements covered
REQ-1.1.3, REQ-1.1.4, REQ-1.1.5, REQ-1.2.1, REQ-1.2.1a, REQ-1.2.1b, REQ-1.2.2, REQ-1.2.3, REQ-1.2.4, REQ-1.2.5, REQ-1.2.6, REQ-1.5.3 (approve/reject actor).

### Dependencies
T-0001 (uses fixture snapshots; realistic flows pair with T-0002). Achievement-unlock-on-approval is implemented in T-0006 and hooks into this approval path — coordinate.

## Status Updates

*To be added during implementation*