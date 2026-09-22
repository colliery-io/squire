//! The **Knight-role** privileged surface (SQUIRE-T-0016): the parent's quick-action endpoints
//! (`POST /admin/...`) plus the cross-Squire `GET /household-review` triage read.
//!
//! Every route here is gated by [`RequireKnight`]: a Squire token (or any non-Knight) is a 403,
//! a missing token a 401 — the trust boundary is enforced in the extractor, before any handler
//! body runs. The Keep remains the **sole writer**: each privileged command is run through the
//! shared [`handle_command`] (snapshot → engine → apply under the store mutex), and the `actor`
//! on every command is filled from the **Knight's verified token** ([`identity::Principal::user`]), never
//! from the client body — the request DTOs below intentionally omit it.
//!
//! ## Idempotency
//! The Knight's offline outbox retries, so replays must be safe:
//! * **Direct redeem / adjust** dedupe on the client-minted `command_id`: the engine returns an
//!   empty change set on a replay of an already-applied command, which this module treats as
//!   **success** (re-returns the ack), not an error.
//! * **Reviews** dedupe on `claim_id` / `request_id`: a second review of an *already-resolved*
//!   subject is a genuine double-action → the engine raises `AlreadyReviewed` → **409**. (A
//!   replay that the engine would no-op is fine; a real second decision is the 409.)

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use domain_core::contract::{
    ClaimId, Clock, Command, CommandId, Currency, Date, Decision, Event, Hazard, HouseholdReview,
    ItemId, ItemOption, PendingCashOut, PendingClaim, PendingRequest, Projections, QuestId,
    QuestOption, Repository, RequestId, Role, Snapshot, SquireSummary, StateView, UserId,
};
use domain_core::Proj;

use crate::auth::RequireKnight;
use crate::squire::{assemble_state, domain_status, handle_command};
use crate::state::AppState;

// ─── wire request DTOs (the body never carries `actor` — it's token-derived) ─────────────

/// The two review verdicts, as a flat string discriminant on the wire.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DecisionKind {
    Approve,
    Reject,
}

/// A review decision as it arrives on the wire. **Flat struct** (not an externally-tagged enum):
/// `{"verdict":"approve"}` or `{"verdict":"reject","reason":"…"}`. The tagged-enum form made
/// openapi-generator emit an undecodable Kotlin class (it couldn't represent the bare `Approve`),
/// so — exactly like the T-0033 state enums — we discriminate on a `verdict` field plus an optional
/// payload. Maps onto the domain [`Decision`].
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DecisionDto {
    pub verdict: DecisionKind,
    /// Optional note for a [`DecisionKind::Reject`]; omitted on the wire when absent.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reason: Option<String>,
}

impl DecisionDto {
    /// `{"verdict":"approve"}` — accept the claim/request.
    pub fn approve() -> Self {
        Self {
            verdict: DecisionKind::Approve,
            reason: None,
        }
    }

    /// `{"verdict":"reject"}` (optionally with a `reason`) — decline it.
    pub fn reject(reason: Option<String>) -> Self {
        Self {
            verdict: DecisionKind::Reject,
            reason,
        }
    }
}

impl From<DecisionDto> for Decision {
    fn from(d: DecisionDto) -> Self {
        match d.verdict {
            DecisionKind::Approve => Decision::Approve,
            DecisionKind::Reject => Decision::Reject { reason: d.reason },
        }
    }
}

/// `POST /admin/review-claim` body. `actor` is the acting Knight, filled from the token.
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ReviewClaimReq {
    pub claim_id: ClaimId,
    pub decision: DecisionDto,
}

/// `POST /admin/review-redemption` body.
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ReviewRedemptionReq {
    pub request_id: RequestId,
    pub decision: DecisionDto,
}

/// `POST /admin/review-cashout` body (SQUIRE-T-0118).
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ReviewCashOutReq {
    pub request_id: RequestId,
    pub decision: DecisionDto,
}

/// `POST /admin/redeem` body — a direct Knight redeem on a target `squire`. Deduped on
/// `command_id`.
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RedeemReq {
    pub command_id: CommandId,
    pub squire: UserId,
    pub item_id: ItemId,
}

/// `POST /admin/adjust` body — a Knight balance override on `squire`. Deduped on `command_id`.
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AdjustReq {
    pub command_id: CommandId,
    pub squire: UserId,
    /// Which currency to adjust (SQUIRE-A-0013). **`Option` is deliberate:** clients (the Kotlin SDK
    /// with `encodeDefaults = true`) send an explicit `"currency": null` for the default — a plain
    /// `#[serde(default)] Currency` rejects that null with a 400. `Option` accepts absent *and* null;
    /// both mean Coins (resolved at the use site).
    #[serde(default)]
    pub currency: Option<Currency>,
    pub amount: i64,
    pub reason: String,
}

/// `POST /admin/mark-done` body — submit-then-approve a claim for `squire` in one shot. The
/// `claim_id` is Knight-minted for idempotency.
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MarkDoneReq {
    pub claim_id: ClaimId,
    pub squire: UserId,
    pub quest_id: QuestId,
    pub on: Date,
}

/// The small JSON ack every quick-action returns on success.
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Ack {
    pub ok: bool,
}

impl Ack {
    fn ok() -> Json<Self> {
        Json(Ack { ok: true })
    }
}

// ─── quick-action handlers (RequireKnight) ───────────────────────────────────────────────

/// `POST /admin/review-claim` — approve / reject a Squire's pending completion claim. The acting
/// Knight is the token's user. A second review of an already-resolved claim is `AlreadyReviewed`
/// → 409.
#[utoipa::path(
    post,
    path = "/admin/review-claim",
    tag = "knight",
    security(("bearer_auth" = [])),
    params(
        ("X-Household" = String, Header, description = "Opaque household handle routing the request to its tenant"),
    ),
    request_body = ReviewClaimReq,
    responses(
        (status = 200, description = "Acknowledged", body = Ack),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Authenticated but not a Knight"),
        (status = 404, description = "Claim not found"),
        (status = 409, description = "Claim already reviewed"),
    ),
)]
pub async fn review_claim(
    State(state): State<Arc<AppState>>,
    RequireKnight(principal): RequireKnight,
    Json(req): Json<ReviewClaimReq>,
) -> Result<Json<Ack>, StatusCode> {
    let cmd = Command::ReviewClaim {
        actor: principal.user,
        claim_id: req.claim_id,
        decision: req.decision.into(),
    };
    // Review: an authoring/committing action — `by` is None (the audit `by` is for definition
    // edits; the appended event carries its own `actor`).
    handle_command(&state, None, cmd).map_err(domain_status)?;
    Ok(Ack::ok())
}

/// `POST /admin/review-redemption` — approve (→ `ItemRedeemed`) / reject a Squire's redemption
/// request. A second review is `AlreadyReviewed` → 409.
#[utoipa::path(
    post,
    path = "/admin/review-redemption",
    tag = "knight",
    security(("bearer_auth" = [])),
    params(
        ("X-Household" = String, Header, description = "Opaque household handle routing the request to its tenant"),
    ),
    request_body = ReviewRedemptionReq,
    responses(
        (status = 200, description = "Acknowledged", body = Ack),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Authenticated but not a Knight"),
        (status = 404, description = "Request not found"),
        (status = 409, description = "Request already reviewed, or blocked at commit"),
    ),
)]
pub async fn review_redemption(
    State(state): State<Arc<AppState>>,
    RequireKnight(principal): RequireKnight,
    Json(req): Json<ReviewRedemptionReq>,
) -> Result<Json<Ack>, StatusCode> {
    let cmd = Command::ReviewRedemption {
        actor: principal.user,
        request_id: req.request_id,
        decision: req.decision.into(),
    };
    handle_command(&state, None, cmd).map_err(domain_status)?;
    Ok(Ack::ok())
}

/// `POST /admin/review-cashout` — approve (→ `CashOutApproved`, drawing down owed Cash) / reject a
/// Squire's cash-out request (SQUIRE-T-0118). A second review is `AlreadyReviewed` → 409.
#[utoipa::path(
    post,
    path = "/admin/review-cashout",
    tag = "knight",
    security(("bearer_auth" = [])),
    params(
        ("X-Household" = String, Header, description = "Opaque household handle routing the request to its tenant"),
    ),
    request_body = ReviewCashOutReq,
    responses(
        (status = 200, description = "Acknowledged", body = Ack),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Authenticated but not a Knight"),
        (status = 404, description = "Request not found"),
        (status = 409, description = "Request already reviewed, or owed Cash no longer covers it"),
    ),
)]
pub async fn review_cashout(
    State(state): State<Arc<AppState>>,
    RequireKnight(principal): RequireKnight,
    Json(req): Json<ReviewCashOutReq>,
) -> Result<Json<Ack>, StatusCode> {
    let cmd = Command::ReviewCashOut {
        actor: principal.user,
        request_id: req.request_id,
        decision: req.decision.into(),
    };
    handle_command(&state, None, cmd).map_err(domain_status)?;
    Ok(Ack::ok())
}

/// `POST /admin/redeem` — a direct Knight redeem for `squire`. `by = None` (an activity event,
/// not authoring). Idempotent on `command_id`: an empty-changeset replay is treated as success.
#[utoipa::path(
    post,
    path = "/admin/redeem",
    tag = "knight",
    security(("bearer_auth" = [])),
    params(
        ("X-Household" = String, Header, description = "Opaque household handle routing the request to its tenant"),
    ),
    request_body = RedeemReq,
    responses(
        (status = 200, description = "Acknowledged", body = Ack),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Authenticated but not a Knight"),
        (status = 404, description = "Item or Squire not found"),
        (status = 409, description = "Redeem blocked by current state"),
    ),
)]
pub async fn redeem(
    State(state): State<Arc<AppState>>,
    RequireKnight(principal): RequireKnight,
    Json(req): Json<RedeemReq>,
) -> Result<Json<Ack>, StatusCode> {
    let cmd = Command::RedeemItem {
        command_id: req.command_id,
        actor: principal.user,
        squire: req.squire,
        item_id: req.item_id,
    };
    // A replay of an already-applied `command_id` yields an empty change set — that is the
    // outbox retrying, so it's success (re-return ok), not a double-spend or an error.
    handle_command(&state, None, cmd).map_err(domain_status)?;
    Ok(Ack::ok())
}

/// `POST /admin/adjust` — a Knight balance override on `squire`. An empty / whitespace reason is
/// rejected with **400** before the engine is touched. Idempotent on `command_id`.
#[utoipa::path(
    post,
    path = "/admin/adjust",
    tag = "knight",
    security(("bearer_auth" = [])),
    params(
        ("X-Household" = String, Header, description = "Opaque household handle routing the request to its tenant"),
    ),
    request_body = AdjustReq,
    responses(
        (status = 200, description = "Acknowledged", body = Ack),
        (status = 400, description = "Blank reason"),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Authenticated but not a Knight"),
        (status = 404, description = "Squire not found"),
    ),
)]
pub async fn adjust(
    State(state): State<Arc<AppState>>,
    RequireKnight(principal): RequireKnight,
    Json(req): Json<AdjustReq>,
) -> Result<Json<Ack>, StatusCode> {
    // Guard the malformed input up front — a blank reason is a bad request (the engine would
    // also reject it as `InvalidDefinition`, but we fail fast and unambiguously here).
    if req.reason.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let cmd = Command::AdjustPoints {
        command_id: req.command_id,
        actor: principal.user,
        squire: req.squire,
        currency: req.currency.unwrap_or(Currency::Coins),
        amount: req.amount,
        reason: req.reason,
    };
    // Replay of the same `command_id` → empty change set → still success (one adjustment).
    handle_command(&state, None, cmd).map_err(domain_status)?;
    Ok(Ack::ok())
}

/// Config key holding the household's hazard catalog (a JSON array of [`Hazard`]).
const HAZARDS_KEY: &str = "hazards";

/// `GET /admin/hazards` (Knight-only) — the household's hazard catalog (shared config); empty if
/// unset (SQUIRE-T-0096).
#[utoipa::path(
    get,
    path = "/admin/hazards",
    tag = "knight",
    security(("bearer_auth" = [])),
    params(("X-Household" = String, Header, description = "Opaque household handle routing the request to its tenant")),
    responses(
        (status = 200, description = "The hazard catalog", body = Vec<Hazard>),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Authenticated but not a Knight"),
    ),
)]
pub async fn list_hazards(
    State(state): State<Arc<AppState>>,
    RequireKnight(_principal): RequireKnight,
) -> Json<Vec<Hazard>> {
    let raw = state
        .store
        .lock()
        .expect("store mutex poisoned")
        .get_setting(HAZARDS_KEY);
    let hazards = raw
        .and_then(|s| serde_json::from_str::<Vec<Hazard>>(&s).ok())
        .unwrap_or_default();
    Json(hazards)
}

/// `PUT /admin/hazards` (Knight-only) — replace the household's hazard catalog; blank-named entries
/// are dropped. Stored as one config value, so the phone and the Keep edit the same list.
#[utoipa::path(
    put,
    path = "/admin/hazards",
    tag = "knight",
    security(("bearer_auth" = [])),
    params(("X-Household" = String, Header, description = "Opaque household handle routing the request to its tenant")),
    request_body = Vec<Hazard>,
    responses(
        (status = 200, description = "Saved", body = Ack),
        (status = 400, description = "Malformed catalog"),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Authenticated but not a Knight"),
    ),
)]
pub async fn set_hazards(
    State(state): State<Arc<AppState>>,
    RequireKnight(principal): RequireKnight,
    Json(hazards): Json<Vec<Hazard>>,
) -> Result<Json<Ack>, StatusCode> {
    let cleaned: Vec<Hazard> = hazards
        .into_iter()
        .filter(|h| !h.name.trim().is_empty())
        .collect();
    let json = serde_json::to_string(&cleaned).map_err(|_| StatusCode::BAD_REQUEST)?;
    state
        .store
        .lock()
        .expect("store mutex poisoned")
        .set_setting(HAZARDS_KEY, &json, Some(principal.user))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Ack::ok())
}

/// `POST /admin/mark-done` — the Knight marks a quest done *for* a Squire: submit the claim
/// (`by = None`, child submission) then immediately approve it (acting Knight). Two commands
/// through the shared writer. The `claim_id` is Knight-minted so a retried outbox is idempotent:
/// the submit replay no-ops, and a re-approve of the now-resolved claim is `AlreadyReviewed` →
/// 409 (a genuine second mark-done), which is correct.
#[utoipa::path(
    post,
    path = "/admin/mark-done",
    tag = "knight",
    security(("bearer_auth" = [])),
    params(
        ("X-Household" = String, Header, description = "Opaque household handle routing the request to its tenant"),
    ),
    request_body = MarkDoneReq,
    responses(
        (status = 200, description = "Acknowledged", body = Ack),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Authenticated but not a Knight, or occurrence taken"),
        (status = 404, description = "Quest or Squire not found"),
        (status = 409, description = "Already reviewed"),
    ),
)]
pub async fn mark_done(
    State(state): State<Arc<AppState>>,
    RequireKnight(principal): RequireKnight,
    Json(req): Json<MarkDoneReq>,
) -> Result<Json<Ack>, StatusCode> {
    let submit = Command::SubmitClaim {
        claim_id: req.claim_id,
        squire: req.squire,
        quest_id: req.quest_id,
        on: req.on,
    };
    handle_command(&state, None, submit).map_err(domain_status)?;

    let approve = Command::ReviewClaim {
        actor: principal.user,
        claim_id: req.claim_id,
        decision: Decision::Approve,
    };
    handle_command(&state, None, approve).map_err(domain_status)?;
    Ok(Ack::ok())
}

// ─── GET /household-review (RequireKnight) ───────────────────────────────────────────────

/// `GET /household-review` — the Knight's cross-Squire triage view, assembled from one snapshot:
/// every active Squire with a clamped balance, plus the household-wide pending claims and pending
/// redemption requests (each labelled with its Squire and the quest/item name).
#[utoipa::path(
    get,
    path = "/household-review",
    tag = "knight",
    security(("bearer_auth" = [])),
    params(
        ("X-Household" = String, Header, description = "Opaque household handle routing the request to its tenant"),
    ),
    responses(
        (status = 200, description = "Cross-Squire triage view", body = HouseholdReview),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Authenticated but not a Knight"),
    ),
)]
pub async fn household_review(
    State(state): State<Arc<AppState>>,
    RequireKnight(_principal): RequireKnight,
) -> Json<HouseholdReview> {
    let store = state.store.lock().expect("store mutex poisoned");
    let snap = store.snapshot();
    let now = state.clock.now();
    let today = state.clock.today();
    let mut review = assemble_review(&snap, now, today);
    crate::tincture::fill(&store, &mut review.squires);
    Json(review)
}

/// `GET /admin/squire/{id}/state` — the **"assume Squire"** read (SQUIRE-T-0053): a Knight fetches
/// any Squire's full [`StateView`] (the same assembly `GET /state` gives that Squire), so the parent
/// can view + operate a child's home. Read-only — acting still goes through the Knight's privileged
/// commands (mark-done / redeem / adjust), so the audit/single-writer model is untouched. An id that
/// isn't an active Squire is a 404.
#[utoipa::path(
    get,
    path = "/admin/squire/{id}/state",
    tag = "knight",
    security(("bearer_auth" = [])),
    params(
        ("X-Household" = String, Header, description = "Opaque household handle routing the request to its tenant"),
        ("id" = i64, Path, description = "The Squire's user id"),
    ),
    responses(
        (status = 200, description = "The Squire's player state", body = StateView),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Authenticated but not a Knight"),
        (status = 404, description = "No such active Squire"),
    ),
)]
pub async fn squire_state(
    State(state): State<Arc<AppState>>,
    RequireKnight(_principal): RequireKnight,
    Path(id): Path<u64>,
) -> Result<Json<StateView>, StatusCode> {
    let squire = UserId(u128::from(id));
    let (snap, today, now) = {
        let store = state.store.lock().expect("store mutex poisoned");
        (store.snapshot(), state.clock.today(), state.clock.now())
    };
    // Only an active Squire in this tenant can be assumed.
    let is_squire = snap
        .users
        .iter()
        .any(|u| u.id == squire && u.active && matches!(u.role, Role::Squire));
    if !is_squire {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(Json(assemble_state(&snap, squire, today, now)))
}

/// Build the [`HouseholdReview`] from one snapshot — pure over `snap`; `now`/`today` from the clock.
fn assemble_review(
    snap: &Snapshot,
    now: domain_core::contract::Timestamp,
    today: Date,
) -> HouseholdReview {
    HouseholdReview {
        generated_at: now,
        squires: squire_summaries(snap),
        pending_claims: pending_claims(snap),
        pending_requests: pending_requests(snap),
        pending_cashouts: pending_cashouts(snap),
        items: item_options(snap),
        quests: quest_options(snap),
        today,
    }
}

/// The active redeemable-item catalog the Knight can direct-redeem from (REQ-K5).
fn item_options(snap: &Snapshot) -> Vec<ItemOption> {
    snap.items
        .iter()
        .filter(|i| i.active)
        .map(|i| ItemOption {
            item_id: i.id,
            name: i.name.clone(),
            cost: i.cost,
        })
        .collect()
}

/// The active quests the Knight can mark done for a Squire (REQ-K3).
fn quest_options(snap: &Snapshot) -> Vec<QuestOption> {
    snap.quests
        .iter()
        .filter(|q| q.active)
        .map(|q| QuestOption {
            quest_id: q.id,
            title: q.title.clone(),
        })
        .collect()
}

/// One [`SquireSummary`] per active Squire, balance clamped `>= 0`.
fn squire_summaries(snap: &Snapshot) -> Vec<SquireSummary> {
    snap.users
        .iter()
        .filter(|u| u.active && matches!(u.role, Role::Squire))
        .map(|u| SquireSummary {
            squire: u.id,
            display_name: u.display_name.clone(),
            tincture: String::new(), // config-table value; the handler fills it in
            balance: Proj::balance(snap, u.id).max(0) as domain_core::contract::Points,
            cash_balance: Proj::balance_in(snap, u.id, Currency::Cash).max(0)
                as domain_core::contract::Points,
        })
        .collect()
}

/// Every `CompletionClaimed` with no matching approval/rejection — the household-wide pending
/// claim queue, labelled with each claim's Squire, quest title, and date.
fn pending_claims(snap: &Snapshot) -> Vec<PendingClaim> {
    snap.events
        .iter()
        .filter_map(|e| match e {
            Event::CompletionClaimed {
                claim_id,
                squire,
                quest_id,
                on,
                ..
            } if !claim_resolved(snap, *claim_id) => {
                let quest_title = snap
                    .quests
                    .iter()
                    .find(|q| q.id == *quest_id)
                    .map(|q| q.title.clone())
                    .unwrap_or_default();
                Some(PendingClaim {
                    claim_id: *claim_id,
                    squire: *squire,
                    quest_title,
                    on: *on,
                })
            }
            _ => None,
        })
        .collect()
}

/// Whether `claim_id` has been approved or rejected.
fn claim_resolved(snap: &Snapshot, claim_id: ClaimId) -> bool {
    snap.events.iter().any(|e| {
        matches!(
            e,
            Event::CompletionApproved { claim_id: c, .. } | Event::CompletionRejected { claim_id: c, .. }
                if *c == claim_id
        )
    })
}

/// Every `RedemptionRequested` with no matching `ItemRedeemed { request_id }` / `RedemptionRejected`
/// — the pending redemption queue, labelled with Squire, item name, and cost.
fn pending_requests(snap: &Snapshot) -> Vec<PendingRequest> {
    snap.events
        .iter()
        .filter_map(|e| match e {
            Event::RedemptionRequested {
                request_id,
                squire,
                item_id,
                ..
            } if !request_resolved(snap, *request_id) => {
                let item = snap.items.iter().find(|i| i.id == *item_id);
                Some(PendingRequest {
                    request_id: *request_id,
                    squire: *squire,
                    item_name: item.map(|i| i.name.clone()).unwrap_or_default(),
                    cost: item.map(|i| i.cost).unwrap_or(0),
                })
            }
            _ => None,
        })
        .collect()
}

/// Whether `request_id` has been fulfilled (`ItemRedeemed`) or rejected.
fn request_resolved(snap: &Snapshot, request_id: RequestId) -> bool {
    snap.events.iter().any(|e| {
        matches!(
            e,
            Event::ItemRedeemed { request_id: Some(r), .. } | Event::RedemptionRejected { request_id: r, .. }
                if *r == request_id
        )
    })
}

/// Every `CashOutRequested` not yet approved/rejected — the pending cash-out queue (SQUIRE-T-0118),
/// labelled with Squire + the dollar amount.
fn pending_cashouts(snap: &Snapshot) -> Vec<PendingCashOut> {
    snap.events
        .iter()
        .filter_map(|e| match e {
            Event::CashOutRequested {
                request_id,
                squire,
                amount,
                ..
            } if !cashout_resolved(snap, *request_id) => Some(PendingCashOut {
                request_id: *request_id,
                squire: *squire,
                amount: *amount,
            }),
            _ => None,
        })
        .collect()
}

/// Whether a cash-out `request_id` has been approved (`CashOutApproved`) or rejected.
fn cashout_resolved(snap: &Snapshot, request_id: RequestId) -> bool {
    snap.events.iter().any(|e| {
        matches!(
            e,
            Event::CashOutApproved { request_id: r, .. } | Event::CashOutRejected { request_id: r, .. }
                if *r == request_id
        )
    })
}
