//! Achievement unlock emission (T-0006). Evaluated as part of the approval path: after a
//! `CompletionApproved` lands, any of the Squire's criteria that are now (first) satisfied
//! emit a sticky `AchievementUnlocked { squire, .. }` (+ bonus). Unlocks can cascade — a
//! bonus may itself push `PointsEarned` over the line — so we run to a fixpoint.

use crate::common::*;
use crate::contract::*;
use crate::projections::Proj;

/// The `AchievementUnlocked` changes that `squire` newly earns once `base_events` (the log
/// *including* the just-appended approval) is in effect, as of `asof` / stamped `at`. Runs to
/// a fixpoint so a bonus that itself satisfies another criterion is also emitted.
pub(crate) fn unlocks_after(
    snap: &Snapshot,
    squire: UserId,
    base_events: &[Event],
    asof: Date,
    at: Timestamp,
) -> Vec<Change> {
    let mut proj = Snapshot {
        users: snap.users.clone(),
        quests: snap.quests.clone(),
        items: snap.items.clone(),
        achievements: snap.achievements.clone(),
        events: base_events.to_vec(),
    };
    let mut out: Vec<Change> = Vec::new();
    loop {
        let mut newly: Vec<(AchievementId, Points)> = Vec::new();
        for a in &proj.achievements {
            if a.active
                && !already_unlocked(&proj, squire, a.id)
                && criterion_met(&proj, squire, &a.criterion, asof)
            {
                newly.push((a.id, a.bonus_points));
            }
        }
        if newly.is_empty() {
            break;
        }
        for (id, bonus) in newly {
            let ev = Event::AchievementUnlocked { squire, id, bonus, at };
            proj.events.push(ev.clone());
            out.push(Change::Append(ev));
        }
    }
    out
}

/// Is there already an `AchievementUnlocked` for `(squire, id)` in the projected log?
fn already_unlocked(proj: &Snapshot, squire: UserId, id: AchievementId) -> bool {
    proj.events.iter().any(|e| {
        matches!(e, Event::AchievementUnlocked { squire: s, id: a, .. } if *s == squire && *a == id)
    })
}

/// Does `squire` satisfy `criterion` in `proj` as of `asof`?
fn criterion_met(proj: &Snapshot, squire: UserId, criterion: &Criterion, asof: Date) -> bool {
    match criterion {
        Criterion::Streak { scope, length, basis } => {
            Proj::current_streak(proj, squire, scope, *basis, asof) >= *length
        }
        Criterion::TotalCompletions { scope, count } => {
            total_completions(proj, squire, scope, asof) >= *count
        }
        Criterion::PointsEarned { total } => points_earned(proj, squire) >= *total,
    }
}
