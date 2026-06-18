//! Authoring commands (T-0002): Define/Archive for Quest / RedeemableItem / Achievement →
//! validated `Change`s (`Put*` / `SetXActive`). Define is upsert (create or edit); Archive
//! flips `active` to false and never deletes, so historical events stay valid (AR-2).
//!
//! Validation is referential + structural: a quest's `Squires(set)` assignees must be active
//! Squires; an item's `gate` must reference an existing achievement; an achievement's
//! `Quest`-scope must reference an existing quest; empty/degenerate definitions are rejected
//! `InvalidDefinition`. The business rules that *consume* these definitions live in later
//! tasks; here we only validate and emit the definition `Change`.

use crate::common::*;
use crate::contract::*;

pub(crate) fn handle(
    snap: &Snapshot,
    cmd: Command,
    _clock: &dyn Clock,
) -> Result<Vec<Change>, DomainError> {
    match cmd {
        Command::DefineQuest(q) => {
            validate_quest(snap, &q)?;
            Ok(vec![Change::PutQuest(q)])
        }
        Command::ArchiveQuest(id) => {
            find_quest(snap, id).ok_or(DomainError::QuestNotFound)?;
            Ok(vec![Change::SetQuestActive(id, false)])
        }
        Command::DefineItem(item) => {
            validate_item(snap, &item)?;
            Ok(vec![Change::PutItem(item)])
        }
        Command::ArchiveItem(id) => {
            find_item(snap, id).ok_or(DomainError::ItemNotFound)?;
            Ok(vec![Change::SetItemActive(id, false)])
        }
        Command::DefineAchievement(a) => {
            validate_achievement(snap, &a)?;
            Ok(vec![Change::PutAchievement(a)])
        }
        Command::ArchiveAchievement(id) => {
            find_achievement(snap, id).ok_or(DomainError::AchievementNotFound)?;
            Ok(vec![Change::SetAchievementActive(id, false)])
        }
        _ => unreachable!("authoring::handle only receives authoring commands"),
    }
}

fn validate_quest(snap: &Snapshot, q: &Quest) -> Result<(), DomainError> {
    match &q.assignment {
        Assignment::AllSquires => {}
        Assignment::Squires(set) => {
            if set.is_empty() {
                return Err(DomainError::InvalidDefinition);
            }
            for uid in set {
                require_active_squire(snap, *uid)?;
            }
        }
    }
    match &q.cadence {
        Cadence::OneOff { .. } | Cadence::Recurring(Schedule::Daily) => {}
        Cadence::Recurring(Schedule::Weekly { days }) => {
            if days.is_empty() {
                return Err(DomainError::InvalidDefinition);
            }
        }
        Cadence::Recurring(Schedule::EveryNDays { n, .. }) => {
            if *n == 0 {
                return Err(DomainError::InvalidDefinition);
            }
        }
    }
    Ok(())
}

fn validate_item(snap: &Snapshot, item: &RedeemableItem) -> Result<(), DomainError> {
    if let Some(aid) = item.gate {
        find_achievement(snap, aid).ok_or(DomainError::AchievementNotFound)?;
    }
    Ok(())
}

fn validate_achievement(snap: &Snapshot, a: &Achievement) -> Result<(), DomainError> {
    let scope_ok = |scope: &Scope| -> Result<(), DomainError> {
        match scope {
            // A Quest scope must name an existing quest.
            Scope::Quest(qid) => {
                find_quest(snap, *qid).ok_or(DomainError::QuestNotFound)?;
            }
            // A Category scope must be a non-blank label (an empty category matches nothing useful).
            Scope::Category(c) if c.0.trim().is_empty() => {
                return Err(DomainError::InvalidDefinition);
            }
            _ => {}
        }
        Ok(())
    };
    match &a.criterion {
        Criterion::Streak { scope, length, .. } => {
            scope_ok(scope)?;
            if *length == 0 {
                return Err(DomainError::InvalidDefinition);
            }
        }
        Criterion::TotalCompletions { scope, count } => {
            scope_ok(scope)?;
            if *count == 0 {
                return Err(DomainError::InvalidDefinition);
            }
        }
        Criterion::PointsEarned { total } => {
            if *total == 0 {
                return Err(DomainError::InvalidDefinition);
            }
        }
    }
    Ok(())
}
