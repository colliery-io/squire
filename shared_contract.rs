//! Squire — shared contract
//!
//! The single seam all sides build against:
//!   * the Keep (computer, Rust): engine + store + admin UI + local API; the only writer
//!   * the Squire (child phone, Kotlin): renders `StateView`, posts claims/requests
//!   * the Knight (parent phone, Kotlin): posts privileged quick-actions under a parent credential
//!
//! Design-first: this file is types + signatures only. No engine logic here.
//!
//! Identity & tenancy (ADR SQUIRE-A-0002 / A-0004):
//!   * A tenant = a Household, fully isolated at the schema level (own Postgres schema /
//!     own SQLite file). Tenancy never appears in these types — a `Snapshot` IS one
//!     household's data, so the domain core stays tenant-agnostic.
//!   * A Household has one-or-more Knights (adults) and Squires (children). All player
//!     activity is attributed to a Squire (`UserId`); every `Event` carries one.
//!
//! Storage split (within a tenant schema):
//!   * IDENTITY (User) + DEFINITIONS (Quest / RedeemableItem / Achievement) = mutable tables.
//!   * ACTIVITY (Event) = append-only log; the only thing that moves a Squire's balance.
//!
//! The placeholder primitives below (Date, Timestamp, *Id) keep this file
//! self-contained — swap them for `chrono` / `uuid` when wiring real deps.

use std::collections::BTreeSet;

// ─── Primitives (placeholders) ──────────────────────────────────────────────
pub type Points = u32;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Date(pub i32); //   days since an epoch
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Timestamp(pub i64); // unix millis

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] pub struct QuestId(pub u128);
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] pub struct ItemId(pub u128);
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] pub struct AchievementId(pub u128);
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] pub struct ClaimId(pub u128);
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] pub struct RequestId(pub u128);
/// Client-minted idempotency key for privileged commands that lack a natural one
/// (`RedeemItem` direct, `AdjustPoints`). The parent phone's offline outbox mints it
/// once and reuses it on every retry; the engine dedupes by finding an emitted event
/// that already carries it, so retry-safety stays *derived from the append-only log*
/// — the same mechanism as `claim_id` / `request_id`. See ADR SQUIRE-A-0001.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] pub struct CommandId(pub u128);

/// Identifies a household member. A household has one-or-more Knights (adult/parent) and
/// Squires (child/player) — counts are fixed nowhere. Player activity is attributed to a
/// Squire via this id. See ADR SQUIRE-A-0004. (Tenancy is schema-level and absent from
/// these types — ADR SQUIRE-A-0002.)
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)] pub struct UserId(pub u128);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Role { Knight, Squire }

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Weekday { Mon, Tue, Wed, Thu, Fri, Sat, Sun }

/// Free-form text labels — the parent types whatever grouping they like.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Category(pub String);

// ─── IDENTITY (mutable table, per tenant) ───────────────────────────────────

/// A household member as the domain sees them — no credentials (those live in the
/// identity/auth layer, never in the shared domain). Deactivated, not deleted, so
/// historical events keep referring to a valid user.
#[derive(Clone, Debug)]
pub struct User {
    pub id: UserId,
    pub role: Role,
    pub display_name: String,
    pub active: bool,
}

// ─── DEFINITIONS (mutable tables) ───────────────────────────────────────────
//
// Items and achievements are a shared household catalog. Quests are authored once and
// ASSIGNED to one-or-more Squires (see `Assignment` / `Completion`). Completion, balance,
// streaks, and unlocks are always per-Squire (see Event).

/// A chore template. Each scheduled occurrence is claimed/completed separately;
/// editing a Quest never rewrites past completions (points are snapshotted).
#[derive(Clone, Debug)]
pub struct Quest {
    pub id: QuestId,
    pub title: String,
    pub description: Option<String>,
    pub category: Option<Category>, // powers category-scoped streaks + UI grouping
    pub reward: Points,             // base award; snapshotted at approval time
    pub cadence: Cadence,
    pub assignment: Assignment,     // which Squires this quest is for (always ≥ 1)
    pub completion: Completion,     // each assignee does their own, vs. first-to-win
    pub auto_approve: bool,         // trust-based chores skip parent review
    pub repeatable_within_day: bool, // true = claimable multiple times/day for repeat points
    pub active: bool,               // archived (not deleted) so history stays valid
    pub icon: Option<String>,       // cosmetic, for the player UI
}

/// Which Squires a quest is for — always at least one.
#[derive(Clone, Debug)]
pub enum Assignment {
    AllSquires,                  // every active Squire (auto-includes Squires added later)
    Squires(BTreeSet<UserId>),   // an explicit subset (one or more)
}

/// How an assigned occurrence is satisfied.
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

#[derive(Clone, Debug)]
pub enum Cadence {
    OneOff { due: Option<Date> },
    Recurring(Schedule),
}

#[derive(Clone, Debug)]
pub enum Schedule {
    Daily,
    Weekly { days: BTreeSet<Weekday> },
    /// `anchor` fixes the phase so the engine knows which days land.
    EveryNDays { n: u16, anchor: Date },
}

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
#[derive(Clone, Debug)]
pub struct Achievement {
    pub id: AchievementId,
    pub name: String,
    pub description: Option<String>,
    pub criterion: Criterion,
    pub bonus_points: Points, // 0 if the reward is purely an unlock
    pub active: bool,
}

#[derive(Clone, Debug)]
pub enum Criterion {
    Streak { scope: Scope, length: u32, basis: StreakBasis },
    TotalCompletions { scope: Scope, count: u32 },
    PointsEarned { total: Points },
}

#[derive(Clone, Debug)]
pub enum Scope {
    Quest(QuestId),
    Category(Category),
    Any,
}

#[derive(Clone, Copy, Debug)]
pub enum StreakBasis {
    /// Respects the quest's schedule — a Mon/Wed/Fri chore isn't broken by Saturday.
    ScheduledOccurrences,
    /// "At least one in scope per calendar day" — for Category / Any scopes.
    CalendarDays,
}

// ─── ACTIVITY (append-only event log) ───────────────────────────────────────

/// Facts, in order. Only these move the balance. **Every event is scoped to one Squire**
/// (`squire`) — the household's log is the union of its Squires' sub-logs, so per-Squire
/// balance / streaks / `StateView` are derived by filtering on `squire`. Knight-committed
/// events also record the acting Knight (`actor`; `None` = auto-approve / system).
#[derive(Clone, Debug)]
pub enum Event {
    /// Proposed by/for a Squire; appended by the server on submit.
    CompletionClaimed  { claim_id: ClaimId, squire: UserId, quest_id: QuestId, on: Date, at: Timestamp },
    /// Committed by a Knight (or auto, if Quest.auto_approve) — point-bearing (+).
    /// `actor` = the approving Knight, or `None` when auto-approved.
    CompletionApproved { claim_id: ClaimId, squire: UserId, actor: Option<UserId>, points: Points, at: Timestamp },
    CompletionRejected { claim_id: ClaimId, squire: UserId, actor: Option<UserId>, reason: Option<String>, at: Timestamp },
    /// Committed spend — point-bearing (−). `request_id` is set when it came from a
    /// Squire's request (deduped via the request lifecycle); `command_id` is set for a
    /// direct Knight redeem (deduped via that key). For client-originated redeems exactly
    /// one is `Some`; a local Keep redeem may leave both `None`.
    ItemRedeemed       { request_id: Option<RequestId>, command_id: Option<CommandId>, squire: UserId, actor: Option<UserId>, item_id: ItemId, cost: Points, at: Timestamp },
    /// Emitted by the engine when a Squire first meets a criterion — sticky thereafter.
    AchievementUnlocked{ squire: UserId, id: AchievementId, bonus: Points, at: Timestamp },
    /// Knight override / correction targeting a Squire's balance. `command_id` is the
    /// idempotency key (always present: minted by the Knight's outbox, or by the Keep for
    /// a local adjustment) so a replay is deduped from the log.
    PointsAdjusted     { command_id: CommandId, squire: UserId, actor: Option<UserId>, amount: i64, reason: String, at: Timestamp },
    /// Squire-initiated, via the API. Reviewed exactly like a completion claim —
    /// it never spends on its own; approval emits `ItemRedeemed`.
    RedemptionRequested{ request_id: RequestId, squire: UserId, item_id: ItemId, at: Timestamp },
    RedemptionRejected { request_id: RequestId, squire: UserId, actor: Option<UserId>, reason: Option<String>, at: Timestamp },
}

// ─── COMMANDS (intents into the one validated door) ─────────────────────────

#[derive(Clone, Debug)]
pub enum Command {
    // Parent / admin. Authoring (Define*/Archive*) is local to the Keep and never on the
    // network. The review/redeem/adjust subset below is ALSO reachable over the network
    // under a separate parent credential (the Knight); either way the Keep is the sole
    // writer and every command passes this one door. See ADR SQUIRE-A-0001, spec SQUIRE-S-0003.
    DefineQuest(Quest),
    ArchiveQuest(QuestId),
    DefineItem(RedeemableItem),
    ArchiveItem(ItemId),
    DefineAchievement(Achievement),
    ArchiveAchievement(AchievementId),
    ReviewClaim { actor: UserId, claim_id: ClaimId, decision: Decision },          // actor = acting Knight; target Squire derived from the claim
    ReviewRedemption { actor: UserId, request_id: RequestId, decision: Decision }, // approve → ItemRedeemed; Squire from the request
    RedeemItem { command_id: CommandId, actor: UserId, squire: UserId, item_id: ItemId },          // Knight `actor` redeems for `squire`; command_id = idempotency key
    AdjustPoints { command_id: CommandId, actor: UserId, squire: UserId, amount: i64, reason: String }, // Knight `actor` adjusts `squire`; "add funds" when amount > 0

    // Squire — arrives via the API; `squire` is the authenticated user, filled by the API
    // from the caller's token (the wire request DTOs intentionally omit it):
    SubmitClaim { claim_id: ClaimId, squire: UserId, quest_id: QuestId, on: Date },
    RequestRedemption { request_id: RequestId, squire: UserId, item_id: ItemId },
}

#[derive(Clone, Debug)]
pub enum Decision { Approve, Reject { reason: Option<String> } }

/// What `handle` emits: a unified set of mutations across both storage shapes.
#[derive(Clone, Debug)]
pub enum Change {
    Append(Event),
    PutQuest(Quest),
    PutItem(RedeemableItem),
    PutAchievement(Achievement),
    SetQuestActive(QuestId, bool),
    SetItemActive(ItemId, bool),
    SetAchievementActive(AchievementId, bool),
    // Identity lifecycle — produced by the Identity/Registration component, not by
    // `Engine::handle` (which never manages users). Still applied via the single writer.
    PutUser(User),
    SetUserActive(UserId, bool),
}

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

// ─── ERRORS ─────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub enum DomainError {
    QuestNotFound, ItemNotFound, AchievementNotFound, ClaimNotFound, RequestNotFound,
    UserNotFound, NotASquire, // a command's `squire` must be an active Squire in the household
    NotAssigned,              // the Squire isn't an assignee of this quest
    OccurrenceTaken,          // a `Race` occurrence was already won by another assignee
    Inactive,
    AlreadyClaimedToday,      // one open claim per (squire, quest, `on`); per assignee for EachAssignee
    AlreadyReviewed,          // claim already approved/rejected
    Redeem(Blocked),
    BadCommandForActor,       // e.g. an approve arriving over the child surface
}

#[derive(Clone, Debug)]
pub enum Blocked {
    InsufficientPoints { needed: Points, have: i64 },
    AchievementLocked { id: AchievementId },
    OutOfStock, // a `Once` item that has already been redeemed
}

#[derive(Clone, Debug)]
pub enum RepoError { Conflict, Io(String) }

// ─── API DTOs (the only things that cross the trust boundary) ───────────────

/// GET /state — everything a Squire's phone renders, fully cacheable for offline use.
/// Scoped to one Squire (`squire`); the Knight's review queue is a separate parent read
/// (`HouseholdReview`).
#[derive(Clone, Debug)]
pub struct StateView {
    pub squire: UserId,
    pub generated_at: Timestamp,
    pub balance: Points,
    pub quests_today: Vec<QuestCard>,
    pub streaks: Vec<StreakView>,
    pub rewards: Vec<RewardCard>,
    pub my_claims: Vec<ClaimStatus>,
    pub my_requests: Vec<RedemptionStatus>,
}

#[derive(Clone, Debug)]
pub struct QuestCard {
    pub quest_id: QuestId,
    pub title: String,
    pub reward: Points,
    pub category: Option<Category>,
    pub icon: Option<String>,
    pub on: Date,
    pub status: QuestStatus,
}
#[derive(Clone, Copy, Debug)]
pub enum QuestStatus { Available, Pending, CompletedToday, TakenByOther } // TakenByOther: a Race quest claimed/won by a sibling

#[derive(Clone, Debug)]
pub struct StreakView {
    pub name: String,
    pub current: u32,
    pub best: u32,
    pub alive: bool,                 // false once an occurrence has lapsed
    pub next_milestone: Option<u32>, // next achievement length in this scope
}

#[derive(Clone, Debug)]
pub struct RewardCard {
    pub item_id: ItemId,
    pub name: String,
    pub cost: Points,
    pub icon: Option<String>,
    pub affordable: bool,
    pub lock: Option<LockReason>,         // None = currently redeemable
    pub last_redeemed: Option<Timestamp>, // most recent redemption of this item (any Squire); informational
}
#[derive(Clone, Debug)]
pub enum LockReason { NeedsAchievement { name: String }, OutOfStock }

#[derive(Clone, Debug)]
pub struct ClaimStatus {
    pub claim_id: ClaimId,
    pub quest_title: String,
    pub on: Date,
    pub state: ClaimState,
}
#[derive(Clone, Debug)]
pub enum ClaimState {
    Pending,
    Approved { points: Points },
    Rejected { reason: Option<String> },
}

#[derive(Clone, Debug)]
pub struct RedemptionStatus {
    pub request_id: RequestId,
    pub item_name: String,
    pub cost: Points,
    pub state: RedemptionState,
}
#[derive(Clone, Debug)]
pub enum RedemptionState {
    Pending,
    Approved,
    Rejected { reason: Option<String> },
}

/// POST /claims — idempotent on `claim_id` (the phone mints it), so the offline
/// outbox can retry safely. Returns Pending, or Approved if the quest auto-approves.
#[derive(Clone, Debug)]
pub struct SubmitClaimReq { pub claim_id: ClaimId, pub quest_id: QuestId, pub on: Date }
#[derive(Clone, Debug)]
pub struct SubmitClaimResp { pub claim_id: ClaimId, pub state: ClaimState }

/// POST /redemption-requests — idempotent on `request_id` (phone-minted), same as claims.
/// Affordability is re-checked at approval time, so a request can be made even if the
/// balance is borderline; the parent sees whether it still clears when they review.
#[derive(Clone, Debug)]
pub struct RequestRedemptionReq { pub request_id: RequestId, pub item_id: ItemId }
#[derive(Clone, Debug)]
pub struct RequestRedemptionResp { pub request_id: RequestId, pub state: RedemptionState }

/// Parent (Knight) review read — pending work across ALL Squires, each labeled with its
/// Squire, plus per-Squire balances. The Knight renders/triages this; the exact shape is
/// still an open decision in SQUIRE-S-0003 / S-0006.
#[derive(Clone, Debug)]
pub struct HouseholdReview {
    pub generated_at: Timestamp,
    pub squires: Vec<SquireSummary>,
    pub pending_claims: Vec<PendingClaim>,
    pub pending_requests: Vec<PendingRequest>,
}
#[derive(Clone, Debug)]
pub struct SquireSummary { pub squire: UserId, pub display_name: String, pub balance: Points }
#[derive(Clone, Debug)]
pub struct PendingClaim { pub claim_id: ClaimId, pub squire: UserId, pub quest_title: String, pub on: Date }
#[derive(Clone, Debug)]
pub struct PendingRequest { pub request_id: RequestId, pub squire: UserId, pub item_name: String, pub cost: Points }

// ─── IDENTITY, REGISTRATION & AUTH DTOs (control plane; ADR SQUIRE-A-0002 / A-0004) ──
//
// Registration creates a Household (tenant) and seeds its first Knight; a Knight then adds
// more members. Auth is per-user: a member secret is exchanged for a tenant-scoped token
// carrying (household, user, role); the API authorizes every call by that triple.
// Credentials are never in these types — only opaque secrets in requests; hashes are
// server-internal. The child submission DTOs above intentionally omit `squire`: the API
// fills it from the caller's token when building the `Command`.

/// Opaque handle a paired device presents to route to its household (and, hosted, to
/// select the schema — ADR SQUIRE-A-0002). Established at registration/pairing.
#[derive(Clone, PartialEq, Eq, Hash, Debug)] pub struct HouseholdHandle(pub String);
/// A tenant-scoped bearer token proving (household, user, role). Presented on every call.
#[derive(Clone, Debug)] pub struct AuthToken(pub String);

/// POST /register — create a Household (tenant) + seed its first Knight (admin); provisions
/// an isolated schema. Returns the handle and the admin's token.
#[derive(Clone, Debug)]
pub struct RegisterHouseholdReq { pub household_name: String, pub admin_name: String, pub admin_secret: String }
#[derive(Clone, Debug)]
pub struct RegisterHouseholdResp { pub household: HouseholdHandle, pub admin: UserId, pub token: AuthToken }

/// Knight-only — add a member (Knight or Squire). No assumed counts or family shape.
#[derive(Clone, Debug)]
pub struct AddMemberReq { pub role: Role, pub display_name: String, pub initial_secret: String }
#[derive(Clone, Debug)]
pub struct AddMemberResp { pub user: UserId }

/// POST /login (or device pair) — exchange a member secret for a tenant-scoped token.
#[derive(Clone, Debug)]
pub struct LoginReq { pub household: HouseholdHandle, pub user: UserId, pub secret: String }
#[derive(Clone, Debug)]
pub struct LoginResp { pub token: AuthToken, pub role: Role }
