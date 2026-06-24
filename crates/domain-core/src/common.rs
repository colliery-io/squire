//! Shared lookup + validation helpers used by the per-family command handlers and the
//! projections. Pure functions over a `Snapshot`.

use crate::contract::*;
use std::collections::{BTreeSet, HashMap};

// ── Claim index (T-0007 perf, NFR-1.1.2) ──
//
// Several log helpers below call `claim_meta` inside an `any()`/`filter` over the event log,
// which is O(n²) at the few-thousand-event scale (NFR-1.1.2). For the projection sweep we build
// a one-pass `ClaimId → (squire, quest, on)` map once per call and use the `*_with` variants
// instead of re-scanning. Behaviour is identical to the scanning helpers; only the cost changes.
// Public signatures are unchanged — callers that don't care (the engine command path, which is
// per-command and small) keep using the plain helpers.

/// One-pass index of `CompletionClaimed` facts and per-claim resolution.
pub struct ClaimIndex {
    /// claim_id → (squire, quest, on), from `CompletionClaimed`.
    meta: HashMap<u128, (UserId, QuestId, Date)>,
    /// claim_id of every claim that has been approved or rejected.
    resolved: HashMap<u128, ClaimResolution>,
}

impl ClaimIndex {
    /// Build in a single pass over the event log.
    pub fn build(snap: &Snapshot) -> Self {
        let mut meta = HashMap::new();
        let mut resolved = HashMap::new();
        for e in &snap.events {
            match e {
                Event::CompletionClaimed {
                    claim_id,
                    squire,
                    quest_id,
                    on,
                    ..
                } => {
                    meta.insert(claim_id.0, (*squire, *quest_id, *on));
                }
                Event::CompletionApproved { claim_id, .. } => {
                    resolved.insert(claim_id.0, ClaimResolution::Approved);
                }
                Event::CompletionRejected { claim_id, .. } => {
                    resolved.insert(claim_id.0, ClaimResolution::Rejected);
                }
                _ => {}
            }
        }
        Self { meta, resolved }
    }

    /// `(squire, quest, on)` of a claim from the index, or `None`. Mirrors `claim_meta`.
    pub fn meta(&self, claim_id: ClaimId) -> Option<(UserId, QuestId, Date)> {
        self.meta.get(&claim_id.0).copied()
    }

    fn resolution(&self, claim_id: ClaimId) -> Option<ClaimResolution> {
        if !self.meta.contains_key(&claim_id.0) {
            return None;
        }
        Some(
            self.resolved
                .get(&claim_id.0)
                .copied()
                .unwrap_or(ClaimResolution::Pending),
        )
    }
}

pub fn find_quest(snap: &Snapshot, id: QuestId) -> Option<&Quest> {
    snap.quests.iter().find(|q| q.id == id)
}

pub fn find_item(snap: &Snapshot, id: ItemId) -> Option<&RedeemableItem> {
    snap.items.iter().find(|i| i.id == id)
}

pub fn find_achievement(snap: &Snapshot, id: AchievementId) -> Option<&Achievement> {
    snap.achievements.iter().find(|a| a.id == id)
}

pub fn find_user(snap: &Snapshot, id: UserId) -> Option<&User> {
    snap.users.iter().find(|u| u.id == id)
}

pub fn is_active_squire(snap: &Snapshot, id: UserId) -> bool {
    find_user(snap, id).is_some_and(|u| u.role == Role::Squire && u.active)
}

/// A `UserId` must name an existing, active `Squire` (REQ-1.1.5): unknown → `UserNotFound`,
/// non-Squire or inactive → `NotASquire`.
pub fn require_active_squire(snap: &Snapshot, id: UserId) -> Result<(), DomainError> {
    match find_user(snap, id) {
        None => Err(DomainError::UserNotFound),
        Some(u) if u.role == Role::Squire && u.active => Ok(()),
        Some(_) => Err(DomainError::NotASquire),
    }
}

/// Is `squire` an assignee of `quest`? `AllSquires` ⇒ any currently active Squire
/// (auto-includes Squires added later); `Squires(set)` ⇒ explicit membership.
pub fn is_assignee(snap: &Snapshot, quest: &Quest, squire: UserId) -> bool {
    match &quest.assignment {
        Assignment::AllSquires => is_active_squire(snap, squire),
        Assignment::Squires(set) => set.contains(&squire),
    }
}

// ── Claim / occurrence log queries (used by claims (T-0003) and due logic (T-0004)) ──
//
// These scan the event log; at the scales of NFR-1.1.2 an index would help, but correctness
// first — T-0007 can optimise behind these signatures.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClaimResolution {
    Pending,
    Approved,
    Rejected,
}

/// `(squire, quest, on)` of a claim, from its `CompletionClaimed` — `None` if no such claim.
pub fn claim_meta(snap: &Snapshot, claim_id: ClaimId) -> Option<(UserId, QuestId, Date)> {
    snap.events.iter().find_map(|e| match e {
        Event::CompletionClaimed {
            claim_id: c,
            squire,
            quest_id,
            on,
            ..
        } if *c == claim_id => Some((*squire, *quest_id, *on)),
        _ => None,
    })
}

/// Has the claim been approved/rejected yet? `None` if the claim doesn't exist.
pub fn claim_resolution(snap: &Snapshot, claim_id: ClaimId) -> Option<ClaimResolution> {
    claim_meta(snap, claim_id)?;
    for e in &snap.events {
        match e {
            Event::CompletionApproved { claim_id: c, .. } if *c == claim_id => {
                return Some(ClaimResolution::Approved)
            }
            Event::CompletionRejected { claim_id: c, .. } if *c == claim_id => {
                return Some(ClaimResolution::Rejected)
            }
            _ => {}
        }
    }
    Some(ClaimResolution::Pending)
}

/// Race: is the `(quest, on)` occurrence closed (some approved completion exists)?
pub fn occurrence_closed(snap: &Snapshot, quest_id: QuestId, on: Date) -> bool {
    snap.events.iter().any(|e| match e {
        Event::CompletionApproved { claim_id, .. } => {
            claim_meta(snap, *claim_id).is_some_and(|(_, q, d)| q == quest_id && d == on)
        }
        _ => false,
    })
}

/// `occurrence_closed` over a prebuilt index (perf path; identical result).
pub fn occurrence_closed_with(
    snap: &Snapshot,
    idx: &ClaimIndex,
    quest_id: QuestId,
    on: Date,
) -> bool {
    snap.events.iter().any(|e| match e {
        Event::CompletionApproved { claim_id, .. } => idx
            .meta(*claim_id)
            .is_some_and(|(_, q, d)| q == quest_id && d == on),
        _ => false,
    })
}

/// EachAssignee: does `squire` already have an approved completion for `(quest, on)`?
/// (Index variant `squire_satisfied_with` is the perf path used by the projections.)
pub fn squire_satisfied_with(
    snap: &Snapshot,
    idx: &ClaimIndex,
    squire: UserId,
    quest_id: QuestId,
    on: Date,
) -> bool {
    snap.events.iter().any(|e| match e {
        Event::CompletionApproved {
            claim_id,
            squire: s,
            ..
        } if *s == squire => idx
            .meta(*claim_id)
            .is_some_and(|(_, q, d)| q == quest_id && d == on),
        _ => false,
    })
}

/// Does `squire` hold a *live* (pending or approved) claim for `(quest, on)`? A rejected
/// claim is not live (the Squire may claim again).
pub fn squire_has_live_claim(snap: &Snapshot, squire: UserId, quest_id: QuestId, on: Date) -> bool {
    snap.events.iter().any(|e| match e {
        Event::CompletionClaimed {
            claim_id,
            squire: s,
            quest_id: q,
            on: d,
            ..
        } if *s == squire && *q == quest_id && *d == on => !matches!(
            claim_resolution(snap, *claim_id),
            Some(ClaimResolution::Rejected)
        ),
        _ => false,
    })
}

/// `squire_has_live_claim` over a prebuilt index (perf path; identical result).
pub fn squire_has_live_claim_with(
    snap: &Snapshot,
    idx: &ClaimIndex,
    squire: UserId,
    quest_id: QuestId,
    on: Date,
) -> bool {
    snap.events.iter().any(|e| match e {
        Event::CompletionClaimed {
            claim_id,
            squire: s,
            quest_id: q,
            on: d,
            ..
        } if *s == squire && *q == quest_id && *d == on => {
            !matches!(idx.resolution(*claim_id), Some(ClaimResolution::Rejected))
        }
        _ => false,
    })
}

// ── Cadence / scheduling (used by quests_due (T-0004); pure integer date math, tz-stable) ──

/// Convention: `Date(0)` is a Monday. The `Clock`/date layer fixes the real epoch + timezone
/// (NFR-1.1.1); the engine only needs a consistent, pure mapping from the day-integer.
pub fn weekday_of(on: Date) -> Weekday {
    match ((on.0 % 7) + 7) % 7 {
        0 => Weekday::Mon,
        1 => Weekday::Tue,
        2 => Weekday::Wed,
        3 => Weekday::Thu,
        4 => Weekday::Fri,
        5 => Weekday::Sat,
        _ => Weekday::Sun,
    }
}

/// Does `schedule` land an occurrence on `on`?
pub fn schedule_matches(schedule: &Schedule, on: Date) -> bool {
    match schedule {
        Schedule::Daily => true,
        Schedule::Weekly { days } => days.contains(&weekday_of(on)),
        Schedule::EveryNDays { n, anchor } => {
            if *n == 0 {
                return false; // guarded at authoring time, but never divide by zero
            }
            let delta = on.0 - anchor.0;
            delta >= 0 && delta % (*n as i32) == 0
        }
    }
}

/// Does `quest`'s cadence schedule an occurrence on `on`? (Schedule only — assignment and
/// satisfaction are layered on by the caller / `quests_due`.)
pub fn cadence_matches(quest: &Quest, on: Date) -> bool {
    match &quest.cadence {
        Cadence::OneOff { due } => due.map_or(true, |d| d == on),
        Cadence::Recurring(s) => schedule_matches(s, on),
    }
}

/// Does `squire` have a *pending* (un-reviewed) claim for `(quest, on)`?
/// (Index variant `squire_pending_with` is the perf path used by the projections.)
pub fn squire_pending_with(
    snap: &Snapshot,
    idx: &ClaimIndex,
    squire: UserId,
    quest_id: QuestId,
    on: Date,
) -> bool {
    snap.events.iter().any(|e| match e {
        Event::CompletionClaimed {
            claim_id,
            squire: s,
            quest_id: q,
            on: d,
            ..
        } if *s == squire && *q == quest_id && *d == on => {
            matches!(idx.resolution(*claim_id), Some(ClaimResolution::Pending))
        }
        _ => false,
    })
}

/// `submit_rejection` over a prebuilt index (perf path; identical result). Used by
/// `quests_due` so a full per-squire due sweep stays linear-ish in the log size.
pub fn submit_rejection_with(
    snap: &Snapshot,
    idx: &ClaimIndex,
    squire: UserId,
    quest: &Quest,
    on: Date,
) -> Option<DomainError> {
    if !quest.active {
        return Some(DomainError::Inactive);
    }
    if !is_assignee(snap, quest, squire) {
        return Some(DomainError::NotAssigned);
    }
    match quest.completion {
        Completion::Race => {
            if occurrence_closed_with(snap, idx, quest.id, on) {
                return Some(DomainError::OccurrenceTaken);
            }
            if squire_has_live_claim_with(snap, idx, squire, quest.id, on) {
                return Some(DomainError::AlreadyClaimedToday);
            }
        }
        Completion::EachAssignee => {
            if !quest.repeatable_within_day
                && squire_has_live_claim_with(snap, idx, squire, quest.id, on)
            {
                return Some(DomainError::AlreadyClaimedToday);
            }
        }
    }
    None
}

/// The reason a `SubmitClaim` for `(squire, quest, on)` would be rejected *right now*, or
/// `None` if it would be accepted. Assumes `squire` is an active Squire and `quest` exists;
/// does NOT enforce scheduling (claims are deliberately lenient — see `claims`) and does NOT
/// check `claim_id` idempotency. Shared by `claims::submit` and `quests_due` so they agree.
pub fn submit_rejection(
    snap: &Snapshot,
    squire: UserId,
    quest: &Quest,
    on: Date,
) -> Option<DomainError> {
    if !quest.active {
        return Some(DomainError::Inactive);
    }
    if !is_assignee(snap, quest, squire) {
        return Some(DomainError::NotAssigned);
    }
    match quest.completion {
        Completion::Race => {
            if occurrence_closed(snap, quest.id, on) {
                return Some(DomainError::OccurrenceTaken);
            }
            if squire_has_live_claim(snap, squire, quest.id, on) {
                return Some(DomainError::AlreadyClaimedToday);
            }
        }
        Completion::EachAssignee => {
            if !quest.repeatable_within_day && squire_has_live_claim(snap, squire, quest.id, on) {
                return Some(DomainError::AlreadyClaimedToday);
            }
        }
    }
    None
}

// ── Redemption / ledger log queries (T-0005) ──

/// `(squire, item)` of a redemption request, from its `RedemptionRequested` — `None` if none.
pub fn request_meta(snap: &Snapshot, request_id: RequestId) -> Option<(UserId, ItemId)> {
    snap.events.iter().find_map(|e| match e {
        Event::RedemptionRequested {
            request_id: r,
            squire,
            item_id,
            ..
        } if *r == request_id => Some((*squire, *item_id)),
        _ => None,
    })
}

/// Has a request been resolved (approved → `ItemRedeemed` with this `request_id`, or rejected)?
pub fn request_resolved(snap: &Snapshot, request_id: RequestId) -> bool {
    snap.events.iter().any(|e| match e {
        Event::ItemRedeemed {
            request_id: Some(r),
            ..
        } if *r == request_id => true,
        Event::RedemptionRejected { request_id: r, .. } if *r == request_id => true,
        _ => false,
    })
}

/// `(squire, amount)` of a cash-out request, from its `CashOutRequested` — `None` if none (T-0118).
pub fn cashout_meta(snap: &Snapshot, request_id: RequestId) -> Option<(UserId, i64)> {
    snap.events.iter().find_map(|e| match e {
        Event::CashOutRequested {
            request_id: r,
            squire,
            amount,
            ..
        } if *r == request_id => Some((*squire, *amount)),
        _ => None,
    })
}

/// Has a cash-out request been resolved (approved → `CashOutApproved`, or rejected)?
pub fn cashout_resolved(snap: &Snapshot, request_id: RequestId) -> bool {
    snap.events.iter().any(|e| match e {
        Event::CashOutApproved { request_id: r, .. } if *r == request_id => true,
        Event::CashOutRejected { request_id: r, .. } if *r == request_id => true,
        _ => false,
    })
}

/// Has a privileged command with this `command_id` already been committed (direct redeem or
/// adjustment)? The basis of `RedeemItem` / `AdjustPoints` idempotency (ADR SQUIRE-A-0001) —
/// derived from the log, not a side table.
pub fn command_already_applied(snap: &Snapshot, command_id: CommandId) -> bool {
    snap.events.iter().any(|e| match e {
        Event::ItemRedeemed {
            command_id: Some(c),
            ..
        } if *c == command_id => true,
        Event::Adjusted { command_id: c, .. } if *c == command_id => true,
        _ => false,
    })
}

/// Has the item ever been redeemed (any Squire)? `Once` items are out-of-stock after one.
pub fn item_ever_redeemed(snap: &Snapshot, item_id: ItemId) -> bool {
    snap.events
        .iter()
        .any(|e| matches!(e, Event::ItemRedeemed { item_id: i, .. } if *i == item_id))
}

/// The most-recent redemption timestamp for an item (any Squire), for `RewardCard.last_redeemed`.
pub fn last_redeemed(snap: &Snapshot, item_id: ItemId) -> Option<Timestamp> {
    snap.events
        .iter()
        .filter_map(|e| match e {
            Event::ItemRedeemed { item_id: i, at, .. } if *i == item_id => Some(*at),
            _ => None,
        })
        .max()
}

// ── Streaks / achievements scope + scheduling helpers (T-0006) ──

/// Does `quest_id` fall within `scope`? `Any` ⇒ always; `Quest(qid)` ⇒ exact match;
/// `Category(cat)` ⇒ the quest exists and carries that category.
pub fn quest_in_scope(snap: &Snapshot, quest_id: QuestId, scope: &Scope) -> bool {
    match scope {
        Scope::Any => true,
        Scope::Quest(qid) => quest_id == *qid,
        Scope::Category(cat) => {
            find_quest(snap, quest_id).and_then(|q| q.category.as_ref()) == Some(cat)
        }
    }
}

/// Did `squire` have ANY approved completion in `scope` on `on`?
/// (Index variant `squire_completed_in_scope_on_with` is the perf path used by the projections.)
pub fn squire_completed_in_scope_on_with(
    snap: &Snapshot,
    idx: &ClaimIndex,
    squire: UserId,
    scope: &Scope,
    on: Date,
) -> bool {
    snap.events.iter().any(|e| match e {
        Event::CompletionApproved {
            claim_id,
            squire: s,
            ..
        } if *s == squire => idx
            .meta(*claim_id)
            .is_some_and(|(_, q, d)| d == on && quest_in_scope(snap, q, scope)),
        _ => false,
    })
}

/// Count of DISTINCT `(quest, day)` occurrences `squire` has approved in `scope` on/before
/// `asof` — a repeatable quest completed twice in one day still counts once.
pub fn total_completions(snap: &Snapshot, squire: UserId, scope: &Scope, asof: Date) -> u32 {
    let mut seen: BTreeSet<(u128, i32)> = BTreeSet::new();
    for e in &snap.events {
        if let Event::CompletionApproved {
            claim_id,
            squire: s,
            ..
        } = e
        {
            if *s == squire {
                if let Some((_, q, d)) = claim_meta(snap, *claim_id) {
                    if d.0 <= asof.0 && quest_in_scope(snap, q, scope) {
                        seen.insert((q.0, d.0));
                    }
                }
            }
        }
    }
    seen.len() as u32
}

/// Lifetime positive earnings for `squire`: approval points + achievement bonuses. This is
/// NOT the net balance (spends/adjustments are excluded) — it backs the `PointsEarned`
/// criterion.
pub fn points_earned(snap: &Snapshot, squire: UserId) -> u32 {
    snap.events
        .iter()
        .map(|e| match e {
            Event::CompletionApproved {
                squire: s, points, ..
            } if *s == squire => *points,
            Event::AchievementUnlocked {
                squire: s, bonus, ..
            } if *s == squire => *bonus,
            _ => 0,
        })
        .sum()
}

/// The latest day on/before `day` on which `quest` is scheduled, or `None` if there is none.
/// Drives the `ScheduledOccurrences` streak walk (skips non-scheduled days like weekends).
pub fn latest_scheduled_on_or_before(quest: &Quest, day: Date) -> Option<Date> {
    match &quest.cadence {
        Cadence::OneOff { due } => due.filter(|d| d.0 <= day.0),
        Cadence::Recurring(Schedule::Daily) => Some(day),
        Cadence::Recurring(Schedule::Weekly { days }) => (0..7)
            .map(|k| Date(day.0 - k))
            .find(|d| days.contains(&weekday_of(*d))),
        Cadence::Recurring(Schedule::EveryNDays { n, anchor }) => {
            if *n == 0 || day.0 < anchor.0 {
                None
            } else {
                let step = *n as i32;
                Some(Date(anchor.0 + ((day.0 - anchor.0) / step) * step))
            }
        }
    }
}
