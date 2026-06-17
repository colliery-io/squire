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

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use domain_core::contract::{
    ClaimId, Clock, Command, CommandId, Date, Decision, Event, HouseholdReview, ItemId,
    PendingClaim, PendingRequest, Projections, QuestId, Repository, RequestId, Role, Snapshot,
    SquireSummary, UserId,
};
use domain_core::Proj;

use crate::auth::RequireKnight;
use crate::squire::{domain_status, handle_command};
use crate::state::AppState;

// ─── wire request DTOs (the body never carries `actor` — it's token-derived) ─────────────

/// A review decision as it arrives on the wire: `approve`, or `reject` with an optional reason.
/// Maps onto the domain [`Decision`]. Tagged so the JSON is `{"approve":{}}` / `{"reject":{...}}`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionDto {
    Approve,
    Reject { reason: Option<String> },
}

impl From<DecisionDto> for Decision {
    fn from(d: DecisionDto) -> Self {
        match d {
            DecisionDto::Approve => Decision::Approve,
            DecisionDto::Reject { reason } => Decision::Reject { reason },
        }
    }
}

/// `POST /admin/review-claim` body. `actor` is the acting Knight, filled from the token.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReviewClaimReq {
    pub claim_id: ClaimId,
    pub decision: DecisionDto,
}

/// `POST /admin/review-redemption` body.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReviewRedemptionReq {
    pub request_id: RequestId,
    pub decision: DecisionDto,
}

/// `POST /admin/redeem` body — a direct Knight redeem on a target `squire`. Deduped on
/// `command_id`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RedeemReq {
    pub command_id: CommandId,
    pub squire: UserId,
    pub item_id: ItemId,
}

/// `POST /admin/adjust` body — a Knight balance override on `squire`. Deduped on `command_id`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdjustReq {
    pub command_id: CommandId,
    pub squire: UserId,
    pub amount: i64,
    pub reason: String,
}

/// `POST /admin/mark-done` body — submit-then-approve a claim for `squire` in one shot. The
/// `claim_id` is Knight-minted for idempotency.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarkDoneReq {
    pub claim_id: ClaimId,
    pub squire: UserId,
    pub quest_id: QuestId,
    pub on: Date,
}

/// The small JSON ack every quick-action returns on success.
#[derive(Clone, Debug, Serialize, Deserialize)]
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

/// `POST /admin/redeem` — a direct Knight redeem for `squire`. `by = None` (an activity event,
/// not authoring). Idempotent on `command_id`: an empty-changeset replay is treated as success.
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
        amount: req.amount,
        reason: req.reason,
    };
    // Replay of the same `command_id` → empty change set → still success (one adjustment).
    handle_command(&state, None, cmd).map_err(domain_status)?;
    Ok(Ack::ok())
}

/// `POST /admin/mark-done` — the Knight marks a quest done *for* a Squire: submit the claim
/// (`by = None`, child submission) then immediately approve it (acting Knight). Two commands
/// through the shared writer. The `claim_id` is Knight-minted so a retried outbox is idempotent:
/// the submit replay no-ops, and a re-approve of the now-resolved claim is `AlreadyReviewed` →
/// 409 (a genuine second mark-done), which is correct.
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
pub async fn household_review(
    State(state): State<Arc<AppState>>,
    RequireKnight(_principal): RequireKnight,
) -> Json<HouseholdReview> {
    let snap = {
        let store = state.store.lock().expect("store mutex poisoned");
        store.snapshot()
    };
    let now = state.clock.now();
    Json(assemble_review(&snap, now))
}

/// Build the [`HouseholdReview`] from one snapshot — pure over `snap`; `now` from the clock.
fn assemble_review(snap: &Snapshot, now: domain_core::contract::Timestamp) -> HouseholdReview {
    HouseholdReview {
        generated_at: now,
        squires: squire_summaries(snap),
        pending_claims: pending_claims(snap),
        pending_requests: pending_requests(snap),
    }
}

/// One [`SquireSummary`] per active Squire, balance clamped `>= 0`.
fn squire_summaries(snap: &Snapshot) -> Vec<SquireSummary> {
    snap.users
        .iter()
        .filter(|u| u.active && matches!(u.role, Role::Squire))
        .map(|u| SquireSummary {
            squire: u.id,
            display_name: u.display_name.clone(),
            balance: Proj::balance(snap, u.id).max(0) as domain_core::contract::Points,
        })
        .collect()
}

/// Every `CompletionClaimed` with no matching approval/rejection — the household-wide pending
/// claim queue, labelled with each claim's Squire, quest title, and date.
fn pending_claims(snap: &Snapshot) -> Vec<PendingClaim> {
    snap.events
        .iter()
        .filter_map(|e| match e {
            Event::CompletionClaimed { claim_id, squire, quest_id, on, .. }
                if !claim_resolved(snap, *claim_id) =>
            {
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
            Event::RedemptionRequested { request_id, squire, item_id, .. }
                if !request_resolved(snap, *request_id) =>
            {
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
