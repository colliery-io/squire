//! The three **control-plane** endpoints (SQUIRE-T-0017): household registration, member login,
//! and Knight-only member creation. Each is a thin handler that delegates straight to the
//! [`Identity`](identity::Identity) port — the API owns no credential logic itself.
//!
//! ## Auth posture
//! * `POST /register` — **unauthenticated**: it bootstraps a household and seeds its first
//!   Knight, so there is no token to present yet.
//! * `POST /login` — **unauthenticated**: it resolves the tenant from the handle and exchanges a
//!   member secret for a tenant-scoped token. A bad secret is a **401** (we couldn't authenticate).
//! * `POST /members` — **[`RequireKnight`]**: a Squire token is a **403** at the extractor, before
//!   the handler runs; `add_member`'s own role check is defense-in-depth.
//!
//! [`AuthError`] maps onto HTTP the same way the auth layer does: `Forbidden` → 403, every other
//! variant (`MissingToken` / `BadToken` / `WrongTenant`) → 401.

use std::sync::Arc;

use axum::extract::State;
use axum::http::header::RETRY_AFTER;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

use domain_core::contract::{
    AddMemberReq, AddMemberResp, LoginReq, LoginResp, MintPairCodeReq, MintPairCodeResp, PairReq,
    PairResp, RegisterHouseholdReq, RegisterHouseholdResp,
};

use identity::AuthError;

use crate::auth::RequireKnight;
use crate::state::AppState;

/// Map an [`AuthError`] from a control-plane call onto its HTTP status: only `Forbidden` is a 403;
/// every other variant means the caller could not be authenticated and is a 401.
fn status_for(err: AuthError) -> StatusCode {
    match err {
        AuthError::Forbidden => StatusCode::FORBIDDEN,
        AuthError::MissingToken | AuthError::BadToken | AuthError::WrongTenant => {
            StatusCode::UNAUTHORIZED
        }
        AuthError::Throttled { .. } => StatusCode::TOO_MANY_REQUESTS,
        AuthError::WeakSecret => StatusCode::BAD_REQUEST,
    }
}

/// `POST /register` (unauthenticated) — create a household + seed its first Knight, returning the
/// new handle, the admin's [`UserId`](domain_core::contract::UserId), and the admin's token.
#[utoipa::path(
    post,
    path = "/register",
    tag = "control",
    request_body = RegisterHouseholdReq,
    responses(
        (status = 200, description = "Household created; returns the handle, admin id, and admin token", body = RegisterHouseholdResp),
    ),
)]
pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterHouseholdReq>,
) -> Result<Json<RegisterHouseholdResp>, StatusCode> {
    state.identity.register(req).map(Json).map_err(status_for)
}

/// `POST /login` (unauthenticated) — exchange a member secret for a tenant-scoped token. A bad
/// secret (or unknown member) is a 401; repeated bad secrets lock the account → 429 (SQUIRE-T-0130).
#[utoipa::path(
    post,
    path = "/login",
    tag = "control",
    request_body = LoginReq,
    responses(
        (status = 200, description = "A tenant-scoped token and the member's role", body = LoginResp),
        (status = 401, description = "Bad secret or unknown member"),
        (status = 429, description = "Too many failed logins for this member; the secret was not tested. Retry after the `Retry-After` header's seconds."),
    ),
)]
pub async fn login(State(state): State<Arc<AppState>>, Json(req): Json<LoginReq>) -> Response {
    let user = req.user.0;
    match state.identity.login(req) {
        Ok(resp) => Json(resp).into_response(),
        Err(AuthError::Throttled { retry_after_s }) => {
            tracing::warn!(
                user,
                retry_after_s,
                "login refused: account locked after repeated failures"
            );
            throttled(retry_after_s)
        }
        Err(e) => status_for(e).into_response(),
    }
}

/// `429` + `Retry-After` for a locked account (SQUIRE-T-0130).
fn throttled(retry_after_s: u32) -> Response {
    (
        StatusCode::TOO_MANY_REQUESTS,
        [(RETRY_AFTER, retry_after_s.to_string())],
    )
        .into_response()
}

/// `POST /members` ([`RequireKnight`]) — add a member (Knight or Squire). The acting caller comes
/// from the verified token; a Squire token never reaches here (the extractor 403s it).
#[utoipa::path(
    post,
    path = "/members",
    tag = "control",
    security(("bearer_auth" = [])),
    params(
        ("X-Household" = String, Header, description = "Opaque household handle routing the request to its tenant"),
    ),
    request_body = AddMemberReq,
    responses(
        (status = 200, description = "The new member's user id", body = AddMemberResp),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Authenticated but not a Knight"),
    ),
)]
pub async fn add_member(
    State(state): State<Arc<AppState>>,
    RequireKnight(principal): RequireKnight,
    Json(req): Json<AddMemberReq>,
) -> Result<Json<AddMemberResp>, StatusCode> {
    state
        .identity
        .add_member(&principal, req)
        .map(Json)
        .map_err(status_for)
}

/// `POST /pair/codes` ([`RequireKnight`]) — mint a one-time device-pairing code for a member
/// (ADR SQUIRE-A-0010). The Keep renders the returned code as a QR; a Squire token is 403'd at the
/// extractor.
#[utoipa::path(
    post,
    path = "/pair/codes",
    tag = "control",
    security(("bearer_auth" = [])),
    params(
        ("X-Household" = String, Header, description = "Opaque household handle routing the request to its tenant"),
    ),
    request_body = MintPairCodeReq,
    responses(
        (status = 200, description = "The minted pairing code and its expiry", body = MintPairCodeResp),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Authenticated but not a Knight, or unknown/inactive target member"),
    ),
)]
pub async fn mint_pair_code(
    State(state): State<Arc<AppState>>,
    RequireKnight(principal): RequireKnight,
    Json(req): Json<MintPairCodeReq>,
) -> Result<Json<MintPairCodeResp>, StatusCode> {
    state
        .identity
        .mint_pairing_code(&principal, req.user)
        .map(Json)
        .map_err(status_for)
}

/// `POST /pair` (**unauthenticated**) — a phone exchanges a one-time pairing code for the member's
/// tenant-scoped token (ADR SQUIRE-A-0010). An unknown / expired / already-used code is a uniform
/// 401 (no enumeration oracle). `household` routes to the tenant (carried from the QR).
#[utoipa::path(
    post,
    path = "/pair",
    tag = "control",
    request_body = PairReq,
    responses(
        (status = 200, description = "The paired member's token and identity", body = PairResp),
        (status = 401, description = "Unknown, expired, or already-used pairing code"),
    ),
)]
pub async fn pair(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PairReq>,
) -> Result<Json<PairResp>, StatusCode> {
    state
        .identity
        .consume_pairing_code(&req.household, &req.code)
        .map(Json)
        .map_err(status_for)
}
