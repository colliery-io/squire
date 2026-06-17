//! `Projections` — pure read derivations over a `Snapshot`, each scoped to one Squire
//! (ADR SQUIRE-A-0005). Everything is derived from the event log; nothing derivable is
//! stored (AR-3).
//!
//! T-0001 provides compiling stubs; the real bodies arrive in:
//! - T-0004: [`Projections::quests_due`]
//! - T-0005: [`Projections::balance`], [`Projections::can_redeem`]
//! - T-0006: [`Projections::current_streak`], [`Projections::is_unlocked`]

use crate::common::*;
use crate::contract::*;

/// Zero-sized projections implementer.
pub struct Proj;

impl Projections for Proj {
    fn balance(_snap: &Snapshot, _squire: UserId) -> i64 {
        0 // TODO(T-0005): per-Squire sum over the log.
    }

    fn quests_due(snap: &Snapshot, squire: UserId, on: Date) -> Vec<QuestId> {
        // A quest is due for `squire` on `on` when it is scheduled that day AND a fresh
        // `SubmitClaim` would currently be accepted (active + assignee + not already
        // satisfied; Race only while the occurrence is open). `submit_rejection` is the same
        // predicate the claim path uses, so the due list and the claim gate never disagree.
        snap.quests
            .iter()
            .filter(|q| {
                cadence_matches(q, on) && submit_rejection(snap, squire, q, on).is_none()
            })
            .map(|q| q.id)
            .collect()
    }

    fn current_streak(
        _snap: &Snapshot,
        _squire: UserId,
        _scope: &Scope,
        _basis: StreakBasis,
        _asof: Date,
    ) -> u32 {
        0 // TODO(T-0006): scheduled vs calendar streak.
    }

    fn is_unlocked(_snap: &Snapshot, _squire: UserId, _id: AchievementId) -> bool {
        false // TODO(T-0006): sticky per-Squire unlock lookup.
    }

    fn can_redeem(
        _snap: &Snapshot,
        _squire: UserId,
        _item: ItemId,
        _on: Date,
    ) -> Result<(), Blocked> {
        Ok(()) // TODO(T-0005): active + gate + balance + Once-out-of-stock.
    }
}

/// The status of one scheduled quest occurrence from `squire`'s point of view, for the
/// `StateView.quests_today` card (the API assembles the cards; the core derives the status).
/// Assumes the quest is scheduled on `on` and `squire` is an assignee — i.e. it would appear
/// in that Squire's "today" list (whether or not it is still claimable).
pub fn quest_status(snap: &Snapshot, squire: UserId, quest: &Quest, on: Date) -> QuestStatus {
    match quest.completion {
        Completion::Race => {
            if occurrence_closed(snap, quest.id, on) {
                // Won — by this Squire (CompletedToday) or a sibling (TakenByOther).
                if squire_satisfied(snap, squire, quest.id, on) {
                    QuestStatus::CompletedToday
                } else {
                    QuestStatus::TakenByOther
                }
            } else if squire_pending(snap, squire, quest.id, on) {
                QuestStatus::Pending
            } else {
                QuestStatus::Available
            }
        }
        Completion::EachAssignee => {
            if squire_satisfied(snap, squire, quest.id, on) {
                QuestStatus::CompletedToday
            } else if squire_pending(snap, squire, quest.id, on) {
                QuestStatus::Pending
            } else {
                QuestStatus::Available
            }
        }
    }
}
