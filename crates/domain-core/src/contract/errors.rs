use super::*;

// ─── ERRORS ─────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub enum DomainError {
    QuestNotFound,
    ItemNotFound,
    AchievementNotFound,
    ClaimNotFound,
    RequestNotFound,
    UserNotFound,
    NotASquire,        // a command's `squire` must be an active Squire in the household
    NotAssigned,       // the Squire isn't an assignee of this quest
    OccurrenceTaken,   // a `Race` occurrence was already won by another assignee
    InvalidDefinition, // a malformed authoring input (e.g. empty assignment / weekly days / zero interval)
    Inactive,
    AlreadyClaimedToday, // one open claim per (squire, quest, `on`); per assignee for EachAssignee
    AlreadyReviewed,     // claim already approved/rejected
    Redeem(Blocked),
    BadCommandForActor, // e.g. an approve arriving over the child surface
}

#[derive(Clone, Debug)]
pub enum Blocked {
    InsufficientPoints { needed: Points, have: i64 },
    AchievementLocked { id: AchievementId },
    OutOfStock, // a `Once` item that has already been redeemed
}

#[derive(Clone, Debug)]
pub enum RepoError {
    Conflict,
    Io(String),
}
