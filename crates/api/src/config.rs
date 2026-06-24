//! Household settings on the LAN api (SQUIRE-T-0113 / ADR SQUIRE-A-0011): read + change the
//! household timezone from the **phone**, at parity with the Keep's `/api/config`.
//!
//! Knight-gated ([`RequireKnight`]). A change validates the IANA zone, persists it audit-stamped
//! (`set_setting`, A-0007), and **hot-swaps the shared live config cell** the clock reads — so the
//! day boundary updates for both the api and the Keep **without a restart** (they share one cell,
//! wired in `squire-home::serve`). Mirrors `crates/keep/src/config.rs`.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use domain_core::contract::{config_keys, HouseholdConfig};
use store::{valid_timezone, ConfigView};

use crate::auth::RequireKnight;
use crate::AppState;

/// `GET /admin/config` (Knight-only) — the current household settings (the typed view).
#[utoipa::path(
    get, path = "/admin/config", tag = "knight",
    params(("X-Household" = String, Header, description = "Tenant household handle")),
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Current household config", body = HouseholdConfig)),
)]
pub async fn get_config(
    State(state): State<Arc<AppState>>,
    _knight: RequireKnight,
) -> Json<HouseholdConfig> {
    let cfg = state
        .store
        .lock()
        .expect("store mutex poisoned")
        .load_config();
    Json(cfg)
}

/// `PUT /admin/config` body — the settings a parent can change (just the timezone for now).
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateConfigReq {
    /// IANA timezone name (e.g. `"America/Detroit"`).
    pub timezone: String,
}

/// `PUT /admin/config` (Knight-only) — change the household timezone. Validates the IANA zone (400
/// on an unknown one), persists it audit-stamped, hot-swaps the live cell so the day boundary
/// changes live (no restart), and echoes the new config.
#[utoipa::path(
    put, path = "/admin/config", tag = "knight",
    params(("X-Household" = String, Header, description = "Tenant household handle")),
    security(("bearer_auth" = [])),
    request_body = UpdateConfigReq,
    responses(
        (status = 200, description = "Updated household config", body = HouseholdConfig),
        (status = 400, description = "Unknown/invalid IANA timezone"),
    ),
)]
pub async fn update_config(
    State(state): State<Arc<AppState>>,
    RequireKnight(principal): RequireKnight,
    Json(req): Json<UpdateConfigReq>,
) -> Result<Json<HouseholdConfig>, StatusCode> {
    let tz = req.timezone.trim();
    if !valid_timezone(tz) {
        return Err(StatusCode::BAD_REQUEST);
    }
    {
        let store = state.store.lock().expect("store mutex poisoned");
        store
            .set_setting(config_keys::TIMEZONE, tz, Some(principal.user))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    // Re-load the typed view and hot-swap the shared live cell (the clock reads it lock-free).
    let cfg = state
        .store
        .lock()
        .expect("store mutex poisoned")
        .load_config();
    state
        .clock
        .live()
        .store(Arc::new(ConfigView::resolve(cfg.clone())));
    Ok(Json(cfg))
}
