//! Item authoring (SQUIRE-T-0027): create / edit / archive [`RedeemableItem`]s, **engine-direct**.
//!
//! `availability` is only `Once` or `Repeatable` (no rate-limit math, A-0006). The list view
//! surfaces each item's last-editor audit (A-0007) plus its **`last_redeemed`** and, for a
//! redeemed `Once`, **out-of-stock** — both derived from the event log (AR-3), not a stored
//! counter. Writes go through [`KeepState::commit`] with `by` = the acting Knight.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;

use domain_core::contract::{Availability, Command, Event, ItemId, RedeemableItem, Snapshot};

use crate::quests::{AuditView, IdResp};
use crate::{domain_status, KeepState, Operator};

/// An item plus its audit and log-derived redemption status, for the authoring list.
#[derive(Debug, Serialize)]
pub struct ItemRow {
    pub item: RedeemableItem,
    pub audit: AuditView,
    /// Unix-millis timestamp of the most recent redemption, if any (derived from the log).
    pub last_redeemed: Option<i64>,
    /// A `Once` item that has already been redeemed shows out-of-stock (derived from the log).
    pub out_of_stock: bool,
}

/// Most-recent redemption timestamp + out-of-stock, derived from the event log for `item`.
fn redemption_status(
    snap: &Snapshot,
    item: ItemId,
    availability: Availability,
) -> (Option<i64>, bool) {
    let last_redeemed = snap
        .events
        .iter()
        .filter_map(|e| match e {
            Event::ItemRedeemed { item_id, at, .. } if *item_id == item => Some(at.0),
            _ => None,
        })
        .max();
    let out_of_stock = matches!(availability, Availability::Once) && last_redeemed.is_some();
    (last_redeemed, out_of_stock)
}

/// `GET /api/items` (Knight-only) — every item with audit + derived `last_redeemed`/out-of-stock.
pub async fn list_items(State(state): State<Arc<KeepState>>, _op: Operator) -> Json<Vec<ItemRow>> {
    let snap = state.snapshot();
    let mut guard = state.store.lock().expect("store mutex poisoned");
    let rows = snap
        .items
        .iter()
        .map(|it| {
            let audit = store::item_audit(&mut guard.connection(), it.id)
                .ok()
                .flatten()
                .map(AuditView::from)
                .unwrap_or_default();
            let (last_redeemed, out_of_stock) = redemption_status(&snap, it.id, it.availability);
            ItemRow {
                item: it.clone(),
                audit,
                last_redeemed,
                out_of_stock,
            }
        })
        .collect();
    Json(rows)
}

/// `POST /api/items` (Knight-only) — create or edit an item via `DefineItem` (upsert by `id`),
/// audited to the acting Knight. A bad definition (e.g. a `gate` to a missing achievement) is a 400.
pub async fn create_item(
    State(state): State<Arc<KeepState>>,
    Operator(op): Operator,
    Json(item): Json<RedeemableItem>,
) -> Result<Json<IdResp>, StatusCode> {
    let id = item.id;
    state
        .commit(Some(op.user), Command::DefineItem(item))
        .map_err(domain_status)?;
    Ok(Json(IdResp {
        id: id.0.to_string(),
    }))
}

/// `POST /api/items/{id}/archive` (Knight-only) — archive an item (`ArchiveItem`), never delete; a
/// missing item is a 404.
pub async fn archive_item(
    State(state): State<Arc<KeepState>>,
    Operator(op): Operator,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let iid = ItemId(id.trim().parse().map_err(|_| StatusCode::BAD_REQUEST)?);
    state
        .commit(Some(op.user), Command::ArchiveItem(iid))
        .map_err(domain_status)?;
    Ok(StatusCode::NO_CONTENT)
}
