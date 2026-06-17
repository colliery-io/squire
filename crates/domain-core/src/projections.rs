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
    fn balance(snap: &Snapshot, squire: UserId) -> i64 {
        // Per-Squire sum over the log; pending claims/requests contribute nothing. Derived,
        // never stored (AR-3). Only `PointsAdjusted` can take it negative.
        snap.events
            .iter()
            .map(|e| match e {
                Event::CompletionApproved { squire: s, points, .. } if *s == squire => *points as i64,
                Event::ItemRedeemed { squire: s, cost, .. } if *s == squire => -(*cost as i64),
                Event::AchievementUnlocked { squire: s, bonus, .. } if *s == squire => *bonus as i64,
                Event::PointsAdjusted { squire: s, amount, .. } if *s == squire => *amount,
                _ => 0,
            })
            .sum()
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

    fn is_unlocked(snap: &Snapshot, squire: UserId, id: AchievementId) -> bool {
        // Sticky per-Squire: true iff an `AchievementUnlocked` for this (squire, id) exists.
        // (T-0006 owns the *emission* of these events; the lookup lives here for `can_redeem`.)
        snap.events.iter().any(|e| {
            matches!(e, Event::AchievementUnlocked { squire: s, id: a, .. } if *s == squire && *a == id)
        })
    }

    fn can_redeem(snap: &Snapshot, squire: UserId, item: ItemId, _on: Date) -> Result<(), Blocked> {
        // Simplified availability (ADR SQUIRE-A-0006): active + gate-unlocked + balance +
        // (Once ⇒ not already redeemed). No rate-limit math. A missing/inactive item is
        // treated as unavailable here (`OutOfStock`); the commit path surfaces the precise
        // `ItemNotFound`/`Inactive` DomainErrors before calling this.
        let item = match find_item(snap, item) {
            Some(i) if i.active => i,
            _ => return Err(Blocked::OutOfStock),
        };
        if let Some(aid) = item.gate {
            if !Self::is_unlocked(snap, squire, aid) {
                return Err(Blocked::AchievementLocked { id: aid });
            }
        }
        if item.availability == Availability::Once && item_ever_redeemed(snap, item.id) {
            return Err(Blocked::OutOfStock);
        }
        let have = Self::balance(snap, squire);
        if have < item.cost as i64 {
            return Err(Blocked::InsufficientPoints { needed: item.cost, have });
        }
        Ok(())
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

/// The redeemability of one item from `squire`'s point of view, for a `StateView.rewards`
/// card. Returns `(affordable, lock, last_redeemed)`: `affordable` = balance ≥ cost;
/// `lock` = `NeedsAchievement` (gated & not unlocked for this Squire) or `OutOfStock`
/// (a redeemed `Once` item), else `None`; `last_redeemed` = most recent redemption of the
/// item (any Squire). The API assembles the `RewardCard` from this.
pub fn reward_view(
    snap: &Snapshot,
    squire: UserId,
    item: &RedeemableItem,
) -> (bool, Option<LockReason>, Option<Timestamp>) {
    let affordable = Proj::balance(snap, squire) >= item.cost as i64;
    let lock = item
        .gate
        .and_then(|aid| {
            (!Proj::is_unlocked(snap, squire, aid)).then(|| {
                let name = find_achievement(snap, aid).map(|a| a.name.clone()).unwrap_or_default();
                LockReason::NeedsAchievement { name }
            })
        })
        .or_else(|| {
            (item.availability == Availability::Once && item_ever_redeemed(snap, item.id))
                .then_some(LockReason::OutOfStock)
        });
    (affordable, lock, last_redeemed(snap, item.id))
}
