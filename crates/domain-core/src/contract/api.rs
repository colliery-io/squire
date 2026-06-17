use super::*;

// ─── API DTOs (the only things that cross the trust boundary) ───────────────

/// GET /state — everything a Squire's phone renders, fully cacheable for offline use.
/// Scoped to one Squire (`squire`); the Knight's review queue is a separate parent read
/// (`HouseholdReview`).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[derive(Clone, Copy, Debug)]
pub enum QuestStatus { Available, Pending, CompletedToday, TakenByOther } // TakenByOther: a Race quest claimed/won by a sibling

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct StreakView {
    pub name: String,
    pub current: u32,
    pub best: u32,
    pub alive: bool,                 // false once an occurrence has lapsed
    pub next_milestone: Option<u32>, // next achievement length in this scope
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum LockReason { NeedsAchievement { name: String }, OutOfStock }

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct ClaimStatus {
    pub claim_id: ClaimId,
    pub quest_title: String,
    pub on: Date,
    pub state: ClaimState,
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum ClaimState {
    Pending,
    Approved { points: Points },
    Rejected { reason: Option<String> },
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct RedemptionStatus {
    pub request_id: RequestId,
    pub item_name: String,
    pub cost: Points,
    pub state: RedemptionState,
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum RedemptionState {
    Pending,
    Approved,
    Rejected { reason: Option<String> },
}

/// POST /claims — idempotent on `claim_id` (the phone mints it), so the offline
/// outbox can retry safely. Returns Pending, or Approved if the quest auto-approves.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct SubmitClaimReq { pub claim_id: ClaimId, pub quest_id: QuestId, pub on: Date }
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct SubmitClaimResp { pub claim_id: ClaimId, pub state: ClaimState }

/// POST /redemption-requests — idempotent on `request_id` (phone-minted), same as claims.
/// Affordability is re-checked at approval time, so a request can be made even if the
/// balance is borderline; the parent sees whether it still clears when they review.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct RequestRedemptionReq { pub request_id: RequestId, pub item_id: ItemId }
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
