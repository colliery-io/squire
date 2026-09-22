use super::*;
use std::collections::BTreeSet;

// ─── DEFINITIONS (mutable tables) ───────────────────────────────────────────
//
// Items and achievements are a shared household catalog. Quests are authored once and
// ASSIGNED to one-or-more Squires (see `Assignment` / `Completion`). Completion, balance,
// streaks, and unlocks are always per-Squire (see Event).

/// A chore template. Each scheduled occurrence is claimed/completed separately;
/// editing a Quest never rewrites past completions (points are snapshotted).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct Quest {
    pub id: QuestId,
    pub title: String,
    pub description: Option<String>,
    pub category: Option<Category>, // powers category-scoped streaks + UI grouping
    pub reward: Points,             // base coin award; snapshotted at approval time
    // Real-money award in whole dollars (0 = none); accrues on approval (SQUIRE-T-0099). `serde(default)`
    // so a client that omits `cash` (e.g. the Keep's library import) gets 0 rather than a deserialize
    // error — the cash analog of the null-currency trap (SQUIRE-T-0109).
    #[cfg_attr(feature = "serde", serde(default))]
    pub cash: Points,
    pub cadence: Cadence,
    pub assignment: Assignment, // which Squires this quest is for (always ≥ 1)
    pub completion: Completion, // each assignee does their own, vs. first-to-win
    pub auto_approve: bool,     // trust-based chores skip parent review
    pub repeatable_within_day: bool, // true = claimable multiple times/day for repeat points
    pub active: bool,           // archived (not deleted) so history stays valid
    pub icon: Option<String>,   // cosmetic, for the player UI
    /// When in the day this chore wants doing — minutes since midnight in the household timezone
    /// (SQUIRE-T-0142). Drives the squire's reminder and the order of today's list, and **nothing
    /// else**: the engine never reads it, so a chore is never overdue and a late claim is never
    /// refused. `serde(default)` so existing quests and older clients are unaffected.
    #[cfg_attr(feature = "serde", serde(default))]
    pub due_time: Option<u16>,
}

/// Which Squires a quest is for — always at least one.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum Assignment {
    AllSquires,                // every active Squire (auto-includes Squires added later)
    Squires(BTreeSet<UserId>), // an explicit subset (one or more)
}

/// How an assigned occurrence is satisfied.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Completion {
    /// Every assignee completes it independently — each has their own occurrence,
    /// claim, and payout ("pick up your room").
    EachAssignee,
    /// Any one assignee completes it. The occurrence stays open until the FIRST
    /// completion is approved (auto or by a Knight), which closes it and is the only
    /// payout — "take out the trash, he who dares wins." Other assignees may still claim
    /// while it's open (so the real doer isn't locked out by a false claim); reviewing a
    /// claim against an already-closed occurrence yields `OccurrenceTaken`.
    Race,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum Cadence {
    OneOff { due: Option<Date> },
    Recurring(Schedule),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum Schedule {
    Daily,
    Weekly {
        days: BTreeSet<Weekday>,
    },
    /// `anchor` fixes the phase so the engine knows which days land.
    EveryNDays {
        n: u16,
        anchor: Date,
    },
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct RedeemableItem {
    pub id: ItemId,
    pub name: String,
    pub description: Option<String>,
    pub cost: Points,
    pub gate: Option<AchievementId>, // must be unlocked before it can be redeemed
    pub availability: Availability,
    pub active: bool,
    pub icon: Option<String>,
}

/// MVP keeps this deliberately simple — no rate-limit math; parents eyeball the last
/// redemption and adjudicate. See ADR SQUIRE-A-0006.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Availability {
    /// A one-off (e.g. a toy): a single household-wide redemption. After the first
    /// `ItemRedeemed` for the item it shows out-of-stock — derived from the log, no
    /// stored counter (AR-3).
    Once,
    /// Redeemable repeatedly; no limit enforced. The UI surfaces the last redemption
    /// (`RewardCard.last_redeemed`) so a parent can see recent use.
    Repeatable,
}

/// Defining an achievement IS how you "define a streak" with teeth — it attaches
/// a reward and/or (via an item's `gate`) unlocks something in the store.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct Achievement {
    pub id: AchievementId,
    pub name: String,
    pub description: Option<String>,
    pub criterion: Criterion,
    pub bonus_points: Points, // 0 if the reward is purely an unlock
    pub active: bool,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum Criterion {
    Streak {
        scope: Scope,
        length: u32,
        basis: StreakBasis,
    },
    TotalCompletions {
        scope: Scope,
        count: u32,
    },
    PointsEarned {
        total: Points,
    },
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum Scope {
    Quest(QuestId),
    Category(Category),
    Any,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug)]
pub enum StreakBasis {
    /// Respects the quest's schedule — a Mon/Wed/Fri chore isn't broken by Saturday.
    ScheduledOccurrences,
    /// "At least one in scope per calendar day" — for Category / Any scopes.
    CalendarDays,
}
