//! Claim & review (T-0003): `SubmitClaim` / `ReviewClaim`.
//!
//! Per-Squire subject validation, assignment gating, auto-approve, snapshot-at-approval,
//! single-review, actor stamping, claim idempotency, and the `Race` occurrence-resolution
//! rule. Claims are zero-value until approved (FR-C2), so this path is deliberately lenient
//! about *scheduling*: cadence/due-ness gates what `quests_due` (T-0004) shows, not what may
//! be submitted — a wrong-day claim just becomes a reviewable pending item a Knight can
//! reject (it can never move the balance on its own). What we DO enforce here: active quest,
//! active assignee Squire, no duplicate live claim, and the Race occurrence rule.

use crate::common::*;
use crate::contract::*;

pub(crate) fn handle(
    snap: &Snapshot,
    cmd: Command,
    clock: &dyn Clock,
) -> Result<Vec<Change>, DomainError> {
    match cmd {
        Command::SubmitClaim { claim_id, squire, quest_id, on } => {
            submit(snap, claim_id, squire, quest_id, on, clock)
        }
        Command::ReviewClaim { actor, claim_id, decision } => {
            review(snap, actor, claim_id, decision, clock)
        }
        _ => unreachable!("claims::handle only receives claim/review commands"),
    }
}

fn submit(
    snap: &Snapshot,
    claim_id: ClaimId,
    squire: UserId,
    quest_id: QuestId,
    on: Date,
    clock: &dyn Clock,
) -> Result<Vec<Change>, DomainError> {
    // Idempotent on the phone-minted claim_id (FR-SY3 / NFR-3): a replay is a no-op.
    if claim_meta(snap, claim_id).is_some() {
        return Ok(Vec::new());
    }
    require_active_squire(snap, squire)?;
    let quest = find_quest(snap, quest_id).ok_or(DomainError::QuestNotFound)?;
    // Active + assignee + dup/Race gating, shared verbatim with `quests_due` (T-0004) so the
    // two can never disagree about what is claimable.
    if let Some(reason) = submit_rejection(snap, squire, quest, on) {
        return Err(reason);
    }

    let at = clock.now();
    let mut changes = vec![Change::Append(Event::CompletionClaimed {
        claim_id,
        squire,
        quest_id,
        on,
        at,
    })];
    if quest.auto_approve {
        // Auto-approval is a system commit (actor = None), snapshotting the current reward.
        changes.extend(approval_events(snap, claim_id, squire, quest.reward, None, at));
    }
    Ok(changes)
}

fn review(
    snap: &Snapshot,
    actor: UserId,
    claim_id: ClaimId,
    decision: Decision,
    clock: &dyn Clock,
) -> Result<Vec<Change>, DomainError> {
    let (squire, quest_id, on) = claim_meta(snap, claim_id).ok_or(DomainError::ClaimNotFound)?;
    if matches!(
        claim_resolution(snap, claim_id),
        Some(ClaimResolution::Approved) | Some(ClaimResolution::Rejected)
    ) {
        return Err(DomainError::AlreadyReviewed);
    }

    let at = clock.now();
    match decision {
        Decision::Approve => {
            // Archived quests keep their row, so this still resolves (reward from the current
            // definition, snapshotted onto the event — AR-4).
            let quest = find_quest(snap, quest_id).ok_or(DomainError::QuestNotFound)?;
            if quest.completion == Completion::Race && occurrence_closed(snap, quest_id, on) {
                // Another assignee already won this occurrence.
                return Err(DomainError::OccurrenceTaken);
            }
            Ok(approval_events(snap, claim_id, squire, quest.reward, Some(actor), at))
        }
        Decision::Reject { reason } => Ok(vec![Change::Append(Event::CompletionRejected {
            claim_id,
            squire,
            actor: Some(actor),
            reason,
            at,
        })]),
    }
}

/// Emit the approval event for a claim, snapshotting `points` (AR-4). `actor = None` denotes
/// an auto-approve / system commit.
///
/// T-0006 extends this to also append `AchievementUnlocked { squire, .. }` (+ bonus) for any
/// of this Squire's criteria that this approval newly satisfies — evaluated against the
/// post-approval log.
fn approval_events(
    _snap: &Snapshot,
    claim_id: ClaimId,
    squire: UserId,
    points: Points,
    actor: Option<UserId>,
    at: Timestamp,
) -> Vec<Change> {
    vec![Change::Append(Event::CompletionApproved {
        claim_id,
        squire,
        actor,
        points,
        at,
    })]
}
