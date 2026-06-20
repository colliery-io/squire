use super::*;

// ─── API DTOs (the only things that cross the trust boundary) ───────────────

/// GET /state — everything a Squire's phone renders, fully cacheable for offline use.
/// Scoped to one Squire (`squire`); the Knight's review queue is a separate parent read
/// (`HouseholdReview`).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Debug)]
pub struct StateView {
    pub squire: UserId,
    pub generated_at: Timestamp,
    pub balance: Points,
    pub quests_today: Vec<QuestCard>,
    pub streaks: Vec<StreakView>,
    /// Achievements the Squire has already earned (SQUIRE-T-0079). `#[serde(default)]` so a payload
    /// from an older server without the field still deserializes.
    #[cfg_attr(feature = "serde", serde(default))]
    pub badges: Vec<BadgeView>,
    /// Achievements not yet earned — "goals to unlock" on the child home (SQUIRE-T-0094 #3), so every
    /// available achievement (not only the in-progress streaks) is visible to work toward.
    /// `#[serde(default)]` so an older server's payload still deserializes.
    #[cfg_attr(feature = "serde", serde(default))]
    pub goals: Vec<GoalView>,
    pub rewards: Vec<RewardCard>,
    pub my_claims: Vec<ClaimStatus>,
    pub my_requests: Vec<RedemptionStatus>,
    /// Point adjustments the child should *see* in their activity feed — coins a grown-up granted
    /// ("+", e.g. for being a gofur) or took away ("−", e.g. a hazard), each with the reason
    /// (SQUIRE-T-0094 / SQUIRE-T-0096). `#[serde(default)]` for back-compat with older servers.
    #[cfg_attr(feature = "serde", serde(default))]
    pub adjustments: Vec<AdjustmentView>,
}

/// One point adjustment shown in the child's activity (SQUIRE-T-0096): a grant (`amount > 0`) or a
/// hazard/penalty (`amount < 0`), with the grown-up's reason, newest first.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Debug)]
pub struct AdjustmentView {
    pub amount: i64,
    pub reason: String,
    pub at: Timestamp,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Copy, Debug)]
pub enum QuestStatus { Available, Pending, CompletedToday, TakenByOther } // TakenByOther: a Race quest claimed/won by a sibling

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Debug)]
pub struct StreakView {
    pub name: String,
    pub current: u32,
    pub best: u32,
    pub alive: bool,                 // false once an occurrence has lapsed
    pub next_milestone: Option<u32>, // next achievement length in this scope
}

/// An achievement the Squire has already **earned** (an `AchievementUnlocked` event exists), shown
/// as a badge on the child home (SQUIRE-T-0079) — distinct from a [`StreakView`] (still in progress).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Debug)]
pub struct BadgeView {
    pub id: AchievementId,
    pub name: String,
    /// The bonus points this unlock awarded.
    pub bonus: Points,
    /// When it was earned (newest first in the list).
    pub at: Timestamp,
}

/// An achievement the Squire has **not yet earned**, shown as a "goal to unlock" on the child home
/// (SQUIRE-T-0094 #3) — so every available achievement, not only streaks, is visible to work toward.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Debug)]
pub struct GoalView {
    pub id: AchievementId,
    pub name: String,
    /// How to earn it, in kid-friendly words — e.g. "Complete 10 chores" or "Earn 100 coins".
    pub description: String,
    /// The bonus coins awarded when it unlocks.
    pub bonus: Points,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
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
// Flat tagged object on the wire — a `kind` discriminator plus the variant's optional payload —
// so `openapi-generator`'s Kotlin/kotlinx backend renders a clean, decodable data class instead
// of collapsing an anonymous `oneOf` into a broken merged class (ADR SQUIRE-A-0009 / SQUIRE-T-0033;
// the discriminated-`oneOf` route emits a non-decodable bare interface for kotlinx, hence the
// flat-struct fallback). Same JSON the old internally-tagged enum produced, minus an explicit
// `null` for absent payloads: `{"kind":"NeedsAchievement","name":"…"}` / `{"kind":"OutOfStock"}`.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Debug)]
pub struct LockReason {
    pub kind: LockReasonKind,
    /// Present for [`LockReasonKind::NeedsAchievement`]; the achievement's name.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none", default))]
    pub name: Option<String>,
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LockReasonKind { NeedsAchievement, OutOfStock }

impl LockReason {
    /// `{"kind":"NeedsAchievement","name":…}` — the reward needs an as-yet-unearned achievement.
    pub fn needs_achievement(name: impl Into<String>) -> Self {
        Self { kind: LockReasonKind::NeedsAchievement, name: Some(name.into()) }
    }
    /// `{"kind":"OutOfStock"}` — the reward is exhausted.
    pub fn out_of_stock() -> Self {
        Self { kind: LockReasonKind::OutOfStock, name: None }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Debug)]
pub struct ClaimStatus {
    pub claim_id: ClaimId,
    pub quest_title: String,
    pub on: Date,
    pub state: ClaimState,
}
// Flat tagged object on the wire (a `state` discriminator + the variant's optional payload) so the
// Kotlin SDK gets a clean, decodable data class (A-0009 / T-0033; flat-struct fallback — see
// [`LockReason`]): `{"state":"Pending"}` / `{"state":"Approved","points":5}` /
// `{"state":"Rejected"}` (or `…,"reason":"…"}`).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Debug)]
pub struct ClaimState {
    pub state: ClaimStateKind,
    /// Present (and required) for [`ClaimStateKind::Approved`]: the points awarded.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none", default))]
    pub points: Option<Points>,
    /// Optional rejection note for [`ClaimStateKind::Rejected`].
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none", default))]
    pub reason: Option<String>,
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClaimStateKind { Pending, Approved, Rejected }

impl ClaimState {
    /// `{"state":"Pending"}` — awaiting a Knight's review.
    pub fn pending() -> Self {
        Self { state: ClaimStateKind::Pending, points: None, reason: None }
    }
    /// `{"state":"Approved","points":…}` — awarded `points`.
    pub fn approved(points: Points) -> Self {
        Self { state: ClaimStateKind::Approved, points: Some(points), reason: None }
    }
    /// `{"state":"Rejected"}` (or with a `reason`) — declined.
    pub fn rejected(reason: Option<String>) -> Self {
        Self { state: ClaimStateKind::Rejected, points: None, reason }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Debug)]
pub struct RedemptionStatus {
    pub request_id: RequestId,
    pub item_name: String,
    pub cost: Points,
    pub state: RedemptionState,
}
// Flat tagged object on the wire (a `state` discriminator) — see [`ClaimState`] (A-0009 / T-0033):
// `{"state":"Pending"}` / `{"state":"Approved"}` / `{"state":"Rejected"}` (or `…,"reason":"…"}`).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Debug)]
pub struct RedemptionState {
    pub state: RedemptionStateKind,
    /// Optional rejection note for [`RedemptionStateKind::Rejected`].
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none", default))]
    pub reason: Option<String>,
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RedemptionStateKind { Pending, Approved, Rejected }

impl RedemptionState {
    /// `{"state":"Pending"}` — awaiting a Knight's review.
    pub fn pending() -> Self {
        Self { state: RedemptionStateKind::Pending, reason: None }
    }
    /// `{"state":"Approved"}` — granted.
    pub fn approved() -> Self {
        Self { state: RedemptionStateKind::Approved, reason: None }
    }
    /// `{"state":"Rejected"}` (or with a `reason`) — declined.
    pub fn rejected(reason: Option<String>) -> Self {
        Self { state: RedemptionStateKind::Rejected, reason }
    }
}

/// POST /claims — idempotent on `claim_id` (the phone mints it), so the offline
/// outbox can retry safely. Returns Pending, or Approved if the quest auto-approves.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Debug)]
pub struct SubmitClaimReq { pub claim_id: ClaimId, pub quest_id: QuestId, pub on: Date }
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Debug)]
pub struct SubmitClaimResp { pub claim_id: ClaimId, pub state: ClaimState }

/// POST /redemption-requests — idempotent on `request_id` (phone-minted), same as claims.
/// Affordability is re-checked at approval time, so a request can be made even if the
/// balance is borderline; the parent sees whether it still clears when they review.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Debug)]
pub struct RequestRedemptionReq { pub request_id: RequestId, pub item_id: ItemId }
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Debug)]
pub struct RequestRedemptionResp { pub request_id: RequestId, pub state: RedemptionState }

/// Parent (Knight) review read — pending work across ALL Squires, each labeled with its
/// Squire, plus per-Squire balances. The Knight renders/triages this; the exact shape is
/// still an open decision in SQUIRE-S-0003 / S-0006.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Debug)]
pub struct HouseholdReview {
    pub generated_at: Timestamp,
    pub squires: Vec<SquireSummary>,
    pub pending_claims: Vec<PendingClaim>,
    pub pending_requests: Vec<PendingRequest>,
    /// Active redeemable items the Knight can direct-redeem on a Squire's behalf (REQ-K5).
    pub items: Vec<ItemOption>,
    /// Active quests the Knight can mark done for a Squire (REQ-K3), paired with [`today`].
    pub quests: Vec<QuestOption>,
    /// The server's "today" (date number) — the `on` a mark-done claim is filed against.
    pub today: Date,
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Debug)]
pub struct SquireSummary { pub squire: UserId, pub display_name: String, pub balance: Points }
/// A redeemable item the Knight can pick for a direct redeem (REQ-K5). Affordability/availability
/// are re-checked by the Keep at commit, so this is just the catalog, not a per-Squire eligibility.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Debug)]
pub struct ItemOption { pub item_id: ItemId, pub name: String, pub cost: Points }
/// An active quest the Knight can pick for a mark-done (REQ-K3).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Debug)]
pub struct QuestOption { pub quest_id: QuestId, pub title: String }
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Debug)]
pub struct PendingClaim { pub claim_id: ClaimId, pub squire: UserId, pub quest_title: String, pub on: Date }
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, PartialEq, Eq, Hash, Debug)] pub struct HouseholdHandle(pub String);
/// A tenant-scoped bearer token proving (household, user, role). Presented on every call.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Debug)] pub struct AuthToken(pub String);

/// POST /register — create a Household (tenant) + seed its first Knight (admin); provisions
/// an isolated schema. Returns the handle and the admin's token.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Debug)]
pub struct RegisterHouseholdReq { pub household_name: String, pub admin_name: String, pub admin_secret: String }
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Debug)]
pub struct RegisterHouseholdResp { pub household: HouseholdHandle, pub admin: UserId, pub token: AuthToken }

/// Knight-only — add a member (Knight or Squire). No assumed counts or family shape.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Debug)]
pub struct AddMemberReq { pub role: Role, pub display_name: String, pub initial_secret: String }
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Debug)]
pub struct AddMemberResp { pub user: UserId }

/// POST /login (or device pair) — exchange a member secret for a tenant-scoped token.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Debug)]
pub struct LoginReq { pub household: HouseholdHandle, pub user: UserId, pub secret: String }
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Debug)]
pub struct LoginResp { pub token: AuthToken, pub role: Role }

/// POST /pair/codes (Knight-only) — mint a one-time device-pairing code for member `user`
/// (ADR SQUIRE-A-0010). The plaintext `code` is returned once for the Keep to render as a QR.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Debug)]
pub struct MintPairCodeReq { pub user: UserId }
/// The minted pairing code and its expiry (unix millis). Single-use; ≥128-bit; 30-min TTL.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Debug)]
pub struct MintPairCodeResp { pub code: String, pub expires_at: Timestamp }

/// POST /pair (unauthenticated) — a phone exchanges a one-time pairing code for the member's
/// tenant-scoped token (ADR SQUIRE-A-0010). `household` routes to the tenant (from the QR).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Debug)]
pub struct PairReq { pub household: HouseholdHandle, pub code: String }
/// The paired member's tenant-scoped token + identity (same token shape as `/login`, A-0004).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Clone, Debug)]
pub struct PairResp { pub token: AuthToken, pub household: HouseholdHandle, pub user: UserId, pub role: Role }
