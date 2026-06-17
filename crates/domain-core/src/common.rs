//! Shared lookup + validation helpers used by the per-family command handlers and the
//! projections. Pure functions over a `Snapshot`.

use crate::contract::*;

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
        Event::CompletionClaimed { claim_id: c, squire, quest_id, on, .. } if *c == claim_id => {
            Some((*squire, *quest_id, *on))
        }
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

/// EachAssignee: does `squire` already have an approved completion for `(quest, on)`?
pub fn squire_satisfied(snap: &Snapshot, squire: UserId, quest_id: QuestId, on: Date) -> bool {
    snap.events.iter().any(|e| match e {
        Event::CompletionApproved { claim_id, squire: s, .. } if *s == squire => {
            claim_meta(snap, *claim_id).is_some_and(|(_, q, d)| q == quest_id && d == on)
        }
        _ => false,
    })
}

/// Does `squire` hold a *live* (pending or approved) claim for `(quest, on)`? A rejected
/// claim is not live (the Squire may claim again).
pub fn squire_has_live_claim(snap: &Snapshot, squire: UserId, quest_id: QuestId, on: Date) -> bool {
    snap.events.iter().any(|e| match e {
        Event::CompletionClaimed { claim_id, squire: s, quest_id: q, on: d, .. }
            if *s == squire && *q == quest_id && *d == on =>
        {
            !matches!(claim_resolution(snap, *claim_id), Some(ClaimResolution::Rejected))
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
