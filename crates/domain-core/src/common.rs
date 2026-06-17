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
