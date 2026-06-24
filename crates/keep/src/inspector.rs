//! The read-only **event-log inspector** (SQUIRE-T-0030): the raw, ordered `Event`s for a given
//! quest or item, so a Knight can trace exactly how a balance or streak was reached — including
//! **who** (`actor`) committed each Knight-committed fact (REQ-1.4.1 / NFR-1.1.1).
//!
//! Pure reads over the store's `seq`-ordered raw log (`store::Store::raw_log_for_quest` /
//! `raw_log_for_item`); no writes. Definition last-editor metadata and a reward's `last_redeemed`
//! are surfaced by the authoring list endpoints (quests/items/achievements), so "who set X" and
//! "when was this last redeemed" are answerable there; this module adds the underlying event trail.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;

use domain_core::contract::{Event, ItemId, QuestId};

use crate::{KeepState, Operator};

/// `GET /api/log/quest/{id}` (Knight-only) — the raw ordered event trail for a quest (its claims +
/// their approvals/rejections), each Knight-committed fact carrying its `actor`.
pub async fn quest_log(
    State(state): State<Arc<KeepState>>,
    _op: Operator,
    Path(id): Path<String>,
) -> Result<Json<Vec<Event>>, StatusCode> {
    let qid = QuestId(id.trim().parse().map_err(|_| StatusCode::BAD_REQUEST)?);
    let log = state
        .store
        .lock()
        .expect("store mutex poisoned")
        .raw_log_for_quest(qid);
    Ok(Json(log))
}

/// `GET /api/log/item/{id}` (Knight-only) — the raw ordered event trail for an item (its
/// redemption requests, rejections, and `ItemRedeemed`s).
pub async fn item_log(
    State(state): State<Arc<KeepState>>,
    _op: Operator,
    Path(id): Path<String>,
) -> Result<Json<Vec<Event>>, StatusCode> {
    let iid = ItemId(id.trim().parse().map_err(|_| StatusCode::BAD_REQUEST)?);
    let log = state
        .store
        .lock()
        .expect("store mutex poisoned")
        .raw_log_for_item(iid);
    Ok(Json(log))
}
