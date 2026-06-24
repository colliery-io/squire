//! Hazard catalog (SQUIRE-T-0096): read/write the household's named-penalty list. Stored as one
//! shared config value under the SAME key the LAN api uses, so the phone and the Keep edit one list
//! (config not phone-only). Applying a hazard reuses `POST /api/adjust` (a negative amount + the
//! hazard name as the reason) — floored at zero and surfaced in the child's activity feed.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use domain_core::contract::Hazard;

use crate::{KeepState, Operator};

/// Config key holding the hazard catalog (matches the LAN api's key).
const HAZARDS_KEY: &str = "hazards";

/// `GET /api/hazards` (Knight-only) — the household's hazard catalog; empty if unset.
pub async fn list_hazards(State(state): State<Arc<KeepState>>, _op: Operator) -> Json<Vec<Hazard>> {
    let raw = state
        .store
        .lock()
        .expect("store mutex poisoned")
        .get_setting(HAZARDS_KEY);
    Json(
        raw.and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default(),
    )
}

/// `PUT /api/hazards` (Knight-only) — replace the catalog; blank-named entries dropped.
pub async fn set_hazards(
    State(state): State<Arc<KeepState>>,
    Operator(op): Operator,
    Json(hazards): Json<Vec<Hazard>>,
) -> Result<Json<Vec<Hazard>>, StatusCode> {
    let cleaned: Vec<Hazard> = hazards
        .into_iter()
        .filter(|h| !h.name.trim().is_empty())
        .collect();
    let json = serde_json::to_string(&cleaned).map_err(|_| StatusCode::BAD_REQUEST)?;
    state
        .store
        .lock()
        .expect("store mutex poisoned")
        .set_setting(HAZARDS_KEY, &json, Some(op.user))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(cleaned))
}
