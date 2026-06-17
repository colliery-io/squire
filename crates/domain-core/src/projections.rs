//! `Projections` — pure read derivations over a `Snapshot`, each scoped to one Squire
//! (ADR SQUIRE-A-0005). Everything is derived from the event log; nothing derivable is
//! stored (AR-3).
//!
//! T-0001 provides compiling stubs; the real bodies arrive in:
//! - T-0004: [`Projections::quests_due`]
//! - T-0005: [`Projections::balance`], [`Projections::can_redeem`]
//! - T-0006: [`Projections::current_streak`], [`Projections::is_unlocked`]

use crate::contract::*;

/// Zero-sized projections implementer.
pub struct Proj;

impl Projections for Proj {
    fn balance(_snap: &Snapshot, _squire: UserId) -> i64 {
        0 // TODO(T-0005): per-Squire sum over the log.
    }

    fn quests_due(_snap: &Snapshot, _squire: UserId, _on: Date) -> Vec<QuestId> {
        Vec::new() // TODO(T-0004): cadence + assignment + Race-open − already satisfied.
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
