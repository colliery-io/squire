//! The Keep's **cross-Squire review queue** and privileged action subset (SQUIRE-T-0029):
//! approve/reject claims & redemption requests in place, direct redeem, and reason-required adjust
//! — all **engine-direct** and actor-stamped to the operating Knight.
//!
//! This mirrors the api's Knight surface (SQUIRE-T-0016) but commits in-process via
//! [`KeepState::commit`] rather than over the network. The queue is the cross-Squire
//! [`HouseholdReview`] assembled from one snapshot (A-0005). Idempotency follows A-0001: redeem /
//! adjust dedupe on `command_id` (a replay yields an empty change set → still success), while a
//! second review of an already-resolved subject is `AlreadyReviewed` → 409.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use domain_core::contract::{
    ClaimId, Clock, Command, CommandId, Decision, Event, HouseholdReview, ItemId, PendingClaim,
    PendingRequest, Projections, RequestId, Role, Snapshot, SquireSummary, Timestamp, UserId,
};
use domain_core::Proj;

use crate::{domain_status, KeepState, Operator};

// ─── request DTOs (the body never carries `actor` — it's the operating Knight) ───────────────

/// A review decision on the wire: `{"approve":{}}` or `{"reject":{"reason":...}}`.
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

/// `POST /api/review/claim` body.
#[derive(Clone, Debug, Deserialize)]
pub struct ReviewClaimReq {
    pub claim_id: ClaimId,
    pub decision: DecisionDto,
}

/// `POST /api/review/redemption` body.
#[derive(Clone, Debug, Deserialize)]
pub struct ReviewRedemptionReq {
    pub request_id: RequestId,
    pub decision: DecisionDto,
}

/// `POST /api/redeem` body — a direct redeem on `squire`, deduped on `command_id`.
#[derive(Clone, Debug, Deserialize)]
pub struct RedeemReq {
    pub command_id: CommandId,
    pub squire: UserId,
    pub item_id: ItemId,
}

/// `POST /api/adjust` body — a signed balance adjustment on `squire`, with a REQUIRED reason.
#[derive(Clone, Debug, Deserialize)]
pub struct AdjustReq {
    pub command_id: CommandId,
    pub squire: UserId,
    pub amount: i64,
    pub reason: String,
}

/// The small JSON ack the actions return on success.
#[derive(Clone, Debug, Serialize)]
pub struct Ack {
    pub ok: bool,
}

fn ack() -> Json<Ack> {
    Json(Ack { ok: true })
}

// ─── GET /api/review (the cross-Squire queue) ────────────────────────────────────────────────

/// `GET /api/review` (Knight-only) — the cross-Squire triage view: every active Squire with a
/// clamped balance, plus the household-wide pending claims and redemption requests (each labelled
/// with its Squire and the quest/item name).
pub async fn get_review(State(state): State<Arc<KeepState>>, _op: Operator) -> Json<HouseholdReview> {
    let snap = state.snapshot();
    let now = state.clock.now();
    Json(assemble_review(&snap, now))
}

// ─── action handlers (Knight-only, engine-direct) ────────────────────────────────────────────

/// `POST /api/review/claim` — approve / reject a pending completion claim; the actor is the
/// operating Knight. A second review of an already-resolved claim is `AlreadyReviewed` → 409.
pub async fn review_claim(
    State(state): State<Arc<KeepState>>,
    Operator(op): Operator,
    Json(req): Json<ReviewClaimReq>,
) -> Result<Json<Ack>, StatusCode> {
    let cmd = Command::ReviewClaim { actor: op.user, claim_id: req.claim_id, decision: req.decision.into() };
    state.commit(None, cmd).map_err(domain_status)?;
    Ok(ack())
}

/// `POST /api/review/redemption` — approve (→ `ItemRedeemed`) / reject a redemption request. A
/// drained balance at approval time fails affordability → 409.
pub async fn review_redemption(
    State(state): State<Arc<KeepState>>,
    Operator(op): Operator,
    Json(req): Json<ReviewRedemptionReq>,
) -> Result<Json<Ack>, StatusCode> {
    let cmd = Command::ReviewRedemption { actor: op.user, request_id: req.request_id, decision: req.decision.into() };
    state.commit(None, cmd).map_err(domain_status)?;
    Ok(ack())
}

/// `POST /api/redeem` — a direct redeem for `squire` (no prior request). Idempotent on
/// `command_id`: a replay yields an empty change set → still success.
pub async fn redeem(
    State(state): State<Arc<KeepState>>,
    Operator(op): Operator,
    Json(req): Json<RedeemReq>,
) -> Result<Json<Ack>, StatusCode> {
    let cmd = Command::RedeemItem { command_id: req.command_id, actor: op.user, squire: req.squire, item_id: req.item_id };
    state.commit(None, cmd).map_err(domain_status)?;
    Ok(ack())
}

/// `POST /api/adjust` — a signed balance override on `squire`. The reason is REQUIRED: a blank
/// reason is a 400 before the engine is touched. Idempotent on `command_id`.
pub async fn adjust(
    State(state): State<Arc<KeepState>>,
    Operator(op): Operator,
    Json(req): Json<AdjustReq>,
) -> Result<Json<Ack>, StatusCode> {
    if req.reason.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let cmd = Command::AdjustPoints { command_id: req.command_id, actor: op.user, squire: req.squire, amount: req.amount, reason: req.reason };
    state.commit(None, cmd).map_err(domain_status)?;
    Ok(ack())
}

// ─── HouseholdReview assembly (pure over one snapshot) ───────────────────────────────────────

/// Build the [`HouseholdReview`] from one snapshot (cross-Squire); `now` from the clock.
fn assemble_review(snap: &Snapshot, now: Timestamp) -> HouseholdReview {
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

/// Every unresolved `CompletionClaimed`, labelled with its Squire, quest title, and date.
fn pending_claims(snap: &Snapshot) -> Vec<PendingClaim> {
    snap.events
        .iter()
        .filter_map(|e| match e {
            Event::CompletionClaimed { claim_id, squire, quest_id, on, .. } if !claim_resolved(snap, *claim_id) => {
                let quest_title = snap.quests.iter().find(|q| q.id == *quest_id).map(|q| q.title.clone()).unwrap_or_default();
                Some(PendingClaim { claim_id: *claim_id, squire: *squire, quest_title, on: *on })
            }
            _ => None,
        })
        .collect()
}

fn claim_resolved(snap: &Snapshot, claim_id: ClaimId) -> bool {
    snap.events.iter().any(|e| {
        matches!(e,
            Event::CompletionApproved { claim_id: c, .. } | Event::CompletionRejected { claim_id: c, .. } if *c == claim_id)
    })
}

/// Every unresolved `RedemptionRequested`, labelled with its Squire, item name, and cost.
fn pending_requests(snap: &Snapshot) -> Vec<PendingRequest> {
    snap.events
        .iter()
        .filter_map(|e| match e {
            Event::RedemptionRequested { request_id, squire, item_id, .. } if !request_resolved(snap, *request_id) => {
                let it = snap.items.iter().find(|i| i.id == *item_id);
                Some(PendingRequest {
                    request_id: *request_id,
                    squire: *squire,
                    item_name: it.map(|i| i.name.clone()).unwrap_or_default(),
                    cost: it.map(|i| i.cost).unwrap_or(0),
                })
            }
            _ => None,
        })
        .collect()
}

fn request_resolved(snap: &Snapshot, request_id: RequestId) -> bool {
    snap.events.iter().any(|e| {
        matches!(e,
            Event::ItemRedeemed { request_id: Some(r), .. } | Event::RedemptionRejected { request_id: r, .. } if *r == request_id)
    })
}
