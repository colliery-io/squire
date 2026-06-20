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
        Self::balance_in(snap, squire, Currency::Coins)
    }

    fn balance_in(snap: &Snapshot, squire: UserId, currency: Currency) -> i64 {
        // Per-Squire sum over the log in one currency; pending claims/requests contribute nothing.
        // Derived, never stored (AR-3). Only `Adjusted` (−) can take a balance negative. The
        // quest/redeem/achievement flows are the COINS path (per `Currency` policy); every currency
        // also accrues via `Adjusted{currency}` (grants, payouts, currency-tagged earnings).
        snap.events
            .iter()
            .map(|e| match e {
                Event::CompletionApproved { squire: s, points, .. } if *s == squire && currency == Currency::Coins => *points as i64,
                Event::ItemRedeemed { squire: s, cost, .. } if *s == squire && currency == Currency::Coins => -(*cost as i64),
                Event::AchievementUnlocked { squire: s, bonus, .. } if *s == squire && currency == Currency::Coins => *bonus as i64,
                Event::Adjusted { squire: s, currency: c, amount, .. } if *s == squire && *c == currency => *amount,
                _ => 0,
            })
            .sum()
    }

    fn quests_due(snap: &Snapshot, squire: UserId, on: Date) -> Vec<QuestId> {
        // A quest is due for `squire` on `on` when it is scheduled that day AND a fresh
        // `SubmitClaim` would currently be accepted (active + assignee + not already
        // satisfied; Race only while the occurrence is open). `submit_rejection` is the same
        // predicate the claim path uses, so the due list and the claim gate never disagree.
        // Build the claim index once (NFR-1.1.2): `submit_rejection_with` reuses it instead of
        // re-scanning the log per quest. Behaviour matches `submit_rejection` exactly.
        let idx = ClaimIndex::build(snap);
        snap.quests
            .iter()
            .filter(|q| {
                cadence_matches(q, on) && submit_rejection_with(snap, &idx, squire, q, on).is_none()
            })
            .map(|q| q.id)
            .collect()
    }

    fn current_streak(
        snap: &Snapshot,
        squire: UserId,
        scope: &Scope,
        basis: StreakBasis,
        asof: Date,
    ) -> u32 {
        // One claim index per call (NFR-1.1.2): the streak walks step day-by-day and would
        // otherwise re-scan the log at each step (O(n²)). The `_with` walks use this index.
        let idx = ClaimIndex::build(snap);
        match (basis, scope) {
            (StreakBasis::ScheduledOccurrences, Scope::Quest(qid)) => {
                scheduled_streak(snap, &idx, squire, *qid, asof)
            }
            _ => calendar_streak(snap, &idx, squire, scope, asof),
        }
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
    // One index per call keeps the StateView assembly (a card per quest) off the O(n²) path.
    let idx = ClaimIndex::build(snap);
    match quest.completion {
        Completion::Race => {
            if occurrence_closed_with(snap, &idx, quest.id, on) {
                // Won — by this Squire (CompletedToday) or a sibling (TakenByOther).
                if squire_satisfied_with(snap, &idx, squire, quest.id, on) {
                    QuestStatus::CompletedToday
                } else {
                    QuestStatus::TakenByOther
                }
            } else if squire_pending_with(snap, &idx, squire, quest.id, on) {
                QuestStatus::Pending
            } else {
                QuestStatus::Available
            }
        }
        Completion::EachAssignee => {
            if squire_satisfied_with(snap, &idx, squire, quest.id, on) {
                QuestStatus::CompletedToday
            } else if squire_pending_with(snap, &idx, squire, quest.id, on) {
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
                LockReason::needs_achievement(name)
            })
        })
        .or_else(|| {
            (item.availability == Availability::Once && item_ever_redeemed(snap, item.id))
                .then(LockReason::out_of_stock)
        });
    (affordable, lock, last_redeemed(snap, item.id))
}

// ── Streak walks (T-0006) ───────────────────────────────────────────────────

/// Consecutive *scheduled* occurrences of `qid` that `squire` has completed, counting back
/// from `asof`. A non-scheduled gap (e.g. a weekend for a Mon/Wed/Fri quest) is skipped, not
/// a break. A pending occurrence *today* (`asof` itself) doesn't break an otherwise-live run.
fn scheduled_streak(snap: &Snapshot, idx: &ClaimIndex, squire: UserId, qid: QuestId, asof: Date) -> u32 {
    let quest = match find_quest(snap, qid) {
        Some(q) => q,
        None => return 0,
    };
    let mut cursor = asof;
    let mut streak = 0u32;
    let mut first = true;
    loop {
        let sd = match latest_scheduled_on_or_before(quest, cursor) {
            Some(d) => d,
            None => break,
        };
        let done = squire_satisfied_with(snap, idx, squire, qid, sd);
        // A not-yet-done scheduled occurrence *today* is forgiven once: step past it.
        if first && sd == asof && !done {
            first = false;
            cursor = Date(sd.0 - 1);
            continue;
        }
        first = false;
        if done {
            streak += 1;
            cursor = Date(sd.0 - 1);
        } else {
            break;
        }
    }
    streak
}

/// Consecutive calendar days (ending at `asof`) with ≥1 in-scope completion. A repeatable
/// quest contributes once/day. A missing `asof` doesn't break a run that ends the day before.
fn calendar_streak(snap: &Snapshot, idx: &ClaimIndex, squire: UserId, scope: &Scope, asof: Date) -> u32 {
    let earliest = snap
        .events
        .iter()
        .filter_map(|e| match e {
            Event::CompletionApproved { claim_id, squire: s, .. } if *s == squire => {
                idx.meta(*claim_id)
                    .filter(|(_, q, _)| quest_in_scope(snap, *q, scope))
                    .map(|(_, _, d)| d.0)
            }
            _ => None,
        })
        .min();
    let earliest = match earliest {
        Some(e) => e,
        None => return 0,
    };
    let mut cursor = asof;
    let mut streak = 0u32;
    let mut first = true;
    while cursor.0 >= earliest {
        let has = squire_completed_in_scope_on_with(snap, idx, squire, scope, cursor);
        if first && !has {
            first = false;
            cursor = Date(cursor.0 - 1);
            continue;
        }
        first = false;
        if has {
            streak += 1;
            cursor = Date(cursor.0 - 1);
        } else {
            break;
        }
    }
    streak
}

/// `Scope` has no `PartialEq`; compare the three arms structurally.
fn scopes_eq(a: &Scope, b: &Scope) -> bool {
    match (a, b) {
        (Scope::Any, Scope::Any) => true,
        (Scope::Quest(x), Scope::Quest(y)) => x == y,
        (Scope::Category(x), Scope::Category(y)) => x == y,
        _ => false,
    }
}

/// Derive a `StreakView`'s `(current, best, alive, next_milestone)` for one Squire/scope.
/// `best` = the high-water mark over every distinct in-scope completion day; `alive` =
/// the current run is non-zero; `next_milestone` = the smallest active `Streak` achievement
/// length in this scope that still lies ahead of `current`.
pub fn streak_view(
    snap: &Snapshot,
    squire: UserId,
    scope: &Scope,
    basis: StreakBasis,
    asof: Date,
) -> (u32, u32, bool, Option<u32>) {
    let current = Proj::current_streak(snap, squire, scope, basis, asof);
    // High-water mark: the streak ending at each day the Squire completed something in scope.
    let mut best = current;
    let mut days: std::collections::BTreeSet<i32> = std::collections::BTreeSet::new();
    let idx = ClaimIndex::build(snap);
    for e in &snap.events {
        if let Event::CompletionApproved { claim_id, squire: s, .. } = e {
            if *s == squire {
                if let Some((_, q, d)) = idx.meta(*claim_id) {
                    if quest_in_scope(snap, q, scope) {
                        days.insert(d.0);
                    }
                }
            }
        }
    }
    for d in days {
        best = best.max(Proj::current_streak(snap, squire, scope, basis, Date(d)));
    }
    let alive = current > 0;
    let next_milestone = snap
        .achievements
        .iter()
        .filter(|a| a.active)
        .filter_map(|a| match &a.criterion {
            Criterion::Streak { scope: s2, length, .. }
                if scopes_eq(s2, scope) && *length > current =>
            {
                Some(*length)
            }
            _ => None,
        })
        .min();
    (current, best, alive, next_milestone)
}
