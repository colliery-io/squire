//! A squire's **tincture** — the colour their screens wear (SQUIRE-T-0136).
//!
//! Stored per member in the household config table (`config_keys::tincture(user)`), not in the
//! domain model: it is presentation, audited like any setting, and nothing in the rules depends
//! on it. The squire sets their own (`POST /me/tincture`); a Knight may set anyone's
//! (`POST /admin/members/{id}/tincture`). Unset reads as [`DEFAULT_TINCTURE`].

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use utoipa::ToSchema;

use domain_core::contract::{
    config_keys, valid_tincture, Repository, SquireSummary, UserId, DEFAULT_TINCTURE,
};
use store::{Store, SystemClock};

use crate::auth::{RequireKnight, RequireSquire};
use crate::state::AppState;

/// The stored tincture for `user`, or the default.
pub fn of(store: &Store<SystemClock>, user: UserId) -> String {
    store
        .get_setting(&config_keys::tincture(user))
        .as_deref()
        .and_then(valid_tincture)
        .unwrap_or(DEFAULT_TINCTURE)
        .to_string()
}

/// Fill every summary's tincture from the config table.
pub fn fill(store: &Store<SystemClock>, squires: &mut [SquireSummary]) {
    for s in squires {
        s.tincture = of(store, s.squire);
    }
}

/// `POST /me/tincture` / `POST /admin/members/{id}/tincture` body.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SetTinctureReq {
    /// One of `gules`, `azure`, `vert`, `purpure`, `tenne`, `sable`.
    pub tincture: String,
}

fn set(
    state: &AppState,
    by: UserId,
    target: UserId,
    req: &SetTinctureReq,
) -> Result<StatusCode, StatusCode> {
    let tincture = valid_tincture(&req.tincture).ok_or(StatusCode::BAD_REQUEST)?;
    let store = state.store.lock().expect("store mutex poisoned");
    if !store.snapshot().users.iter().any(|u| u.id == target) {
        return Err(StatusCode::NOT_FOUND);
    }
    store
        .set_setting(&config_keys::tincture(target), tincture, Some(by))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /me/tincture` ([`RequireSquire`]) — the signed-in squire picks their own colour.
#[utoipa::path(
    post,
    path = "/me/tincture",
    tag = "squire",
    security(("bearer_auth" = [])),
    params(("X-Household" = String, Header, description = "Opaque household handle routing the request to its tenant")),
    request_body = SetTinctureReq,
    responses(
        (status = 204, description = "Tincture saved"),
        (status = 400, description = "Not one of the known tinctures"),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Not a Squire"),
    ),
)]
pub async fn set_my_tincture(
    State(state): State<Arc<AppState>>,
    RequireSquire(principal): RequireSquire,
    Json(req): Json<SetTinctureReq>,
) -> Result<StatusCode, StatusCode> {
    set(&state, principal.user, principal.user, &req)
}

/// `POST /admin/members/{id}/tincture` ([`RequireKnight`]) — a Knight sets a member's colour.
#[utoipa::path(
    post,
    path = "/admin/members/{id}/tincture",
    tag = "knight",
    security(("bearer_auth" = [])),
    params(
        ("X-Household" = String, Header, description = "Opaque household handle routing the request to its tenant"),
        ("id" = u64, Path, description = "Member user id"),
    ),
    request_body = SetTinctureReq,
    responses(
        (status = 204, description = "Tincture saved"),
        (status = 400, description = "Not one of the known tinctures"),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Authenticated but not a Knight"),
        (status = 404, description = "No such member"),
    ),
)]
pub async fn set_member_tincture(
    State(state): State<Arc<AppState>>,
    RequireKnight(principal): RequireKnight,
    Path(id): Path<u64>,
    Json(req): Json<SetTinctureReq>,
) -> Result<StatusCode, StatusCode> {
    set(&state, principal.user, UserId(u128::from(id)), &req)
}
