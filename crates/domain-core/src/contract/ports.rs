use super::*;

// ─── PORTS ──────────────────────────────────────────────────────────────────

/// Read snapshot the engine and projections reason over.
pub struct Snapshot {
    pub users: Vec<User>,   // the household's Knights + Squires (attribution + enumeration)
    pub quests: Vec<Quest>,
    pub items: Vec<RedeemableItem>,
    pub achievements: Vec<Achievement>,
    pub events: Vec<Event>,
}

pub trait Repository {
    fn snapshot(&self) -> Snapshot;
    /// Single writer. `by` = the user applying the change (a Knight for authoring; `None`
    /// for system/seed) — used to stamp last-editor audit columns (`created_by`/`updated_by`,
    /// + timestamps) on definition rows. Ignored for `Append` (an event already carries its
    /// own `actor`/`squire`). See ADR SQUIRE-A-0007.
    fn apply(&mut self, by: Option<UserId>, changes: &[Change]) -> Result<(), RepoError>;
}

pub trait Clock {
    fn today(&self) -> Date;
    fn now(&self) -> Timestamp;
}

/// The one validated entry point. Definition edits, approvals, and redemptions
/// all pass through here, so every rule lives in one testable place.
pub trait Engine {
    fn handle(&self, snap: &Snapshot, cmd: Command, clock: &dyn Clock)
        -> Result<Vec<Change>, DomainError>;
}

/// Pure derivations over a snapshot, each scoped to one Squire.
pub trait Projections {
    fn balance(snap: &Snapshot, squire: UserId) -> i64;
    fn quests_due(snap: &Snapshot, squire: UserId, on: Date) -> Vec<QuestId>;
    fn current_streak(snap: &Snapshot, squire: UserId, scope: &Scope, basis: StreakBasis, asof: Date) -> u32;
    fn is_unlocked(snap: &Snapshot, squire: UserId, id: AchievementId) -> bool;
    fn can_redeem(snap: &Snapshot, squire: UserId, item: ItemId, on: Date) -> Result<(), Blocked>;
}
