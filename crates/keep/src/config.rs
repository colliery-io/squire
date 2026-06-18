//! Household settings (ADR SQUIRE-A-0011 / SQUIRE-T-0068): read + change the household timezone.
//!
//! Knight-only (the [`Operator`] extractor). A change validates the IANA zone, persists it
//! (`set_setting`, audit-stamped with the acting Knight, A-0007), and **hot-swaps the live config
//! cell** the clock reads — so the day boundary updates for both the Keep and the LAN api **without
//! a restart** (they share one cell, wired in `squire-home::serve`).

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use domain_core::contract::{config_keys, HouseholdConfig};
use store::{valid_timezone, ConfigView};

use crate::{KeepState, Operator};

/// `GET /api/config` (Knight-only) — the current household settings (the typed view).
pub async fn get_config(State(state): State<Arc<KeepState>>, _op: Operator) -> Json<HouseholdConfig> {
    let cfg = state.store.lock().expect("store mutex poisoned").load_config();
    Json(cfg)
}

/// `PUT /api/config` body — the settings a parent can change (just the timezone for now).
#[derive(Debug, Deserialize)]
pub struct UpdateConfigReq {
    pub timezone: String,
}

/// `PUT /api/config` (Knight-only) — change the household timezone. Validates the IANA zone (400 on
/// an unknown one), persists it audit-stamped, hot-swaps the live cell so the clock changes live,
/// and echoes the new config.
pub async fn update_config(
    State(state): State<Arc<KeepState>>,
    Operator(op): Operator,
    Json(req): Json<UpdateConfigReq>,
) -> Result<Json<HouseholdConfig>, StatusCode> {
    let tz = req.timezone.trim();
    if !valid_timezone(tz) {
        return Err(StatusCode::BAD_REQUEST);
    }
    {
        let store = state.store.lock().expect("store mutex poisoned");
        store
            .set_setting(config_keys::TIMEZONE, tz, Some(op.user))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    // Re-load the typed view and hot-swap the shared live cell (the clock reads it lock-free).
    let cfg = state.store.lock().expect("store mutex poisoned").load_config();
    state.clock.live().store(Arc::new(ConfigView::resolve(cfg.clone())));
    Ok(Json(cfg))
}
