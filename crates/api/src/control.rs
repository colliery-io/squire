//! The three **control-plane** endpoints (SQUIRE-T-0017): household registration, member login,
//! and Knight-only member creation. Each is a thin handler that delegates straight to the
//! [`Identity`](crate::identity::Identity) port — the API owns no credential logic itself.
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
use axum::http::StatusCode;
use axum::Json;

use domain_core::contract::{
    AddMemberReq, AddMemberResp, LoginReq, LoginResp, RegisterHouseholdReq, RegisterHouseholdResp,
};

use crate::auth::RequireKnight;
use crate::identity::AuthError;
use crate::state::AppState;

/// Map an [`AuthError`] from a control-plane call onto its HTTP status: only `Forbidden` is a 403;
/// every other variant means the caller could not be authenticated and is a 401.
fn status_for(err: AuthError) -> StatusCode {
    match err {
        AuthError::Forbidden => StatusCode::FORBIDDEN,
        AuthError::MissingToken | AuthError::BadToken | AuthError::WrongTenant => {
            StatusCode::UNAUTHORIZED
        }
    }
}

/// `POST /register` (unauthenticated) — create a household + seed its first Knight, returning the
/// new handle, the admin's [`UserId`](domain_core::contract::UserId), and the admin's token.
pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterHouseholdReq>,
) -> Result<Json<RegisterHouseholdResp>, StatusCode> {
    state.identity.register(req).map(Json).map_err(status_for)
}

/// `POST /login` (unauthenticated) — exchange a member secret for a tenant-scoped token. A bad
/// secret (or unknown member) is a 401.
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginReq>,
) -> Result<Json<LoginResp>, StatusCode> {
    state.identity.login(req).map(Json).map_err(status_for)
}

/// `POST /members` ([`RequireKnight`]) — add a member (Knight or Squire). The acting caller comes
/// from the verified token; a Squire token never reaches here (the extractor 403s it).
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
