//! Member administration (SQUIRE-T-0028): add Knights & Squires, mint their tenant-scoped tokens,
//! list members with last-editor audit, and de/reactivate (archive-not-delete).
//!
//! Adds/seed go through the [`identity`] component (the same single-writer store the Keep holds), so
//! every member write serializes on one writer and the `users` audit columns record **who added /
//! last changed** each member (A-0004 / A-0007). All routes are Knight-only (the [`Operator`]
//! extractor), and `identity::add_member` re-checks the Knight role as defense in depth.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use domain_core::contract::{
    config_keys, valid_tincture, AddMemberReq, Change, LoginReq, Repository, Role, User, UserId,
    DEFAULT_TINCTURE,
};
use identity::AuthError;
use store::{Store, SystemClock};

use crate::quests::AuditView;
use crate::{KeepState, Operator};

/// A household member plus its last-editor audit, for the members list.
#[derive(Debug, Serialize)]
pub struct MemberRow {
    /// `UserId` as a decimal string (`u128` is not a JSON-safe integer everywhere).
    pub user: String,
    pub role: Role,
    pub display_name: String,
    pub active: bool,
    /// The colour this member's screens wear (SQUIRE-T-0136); Squires only in practice.
    pub tincture: String,
    pub audit: AuditView,
}

/// The result of adding a member: its id plus a freshly-minted tenant-scoped token, so a phone can
/// be provisioned for the new member immediately.
#[derive(Debug, Serialize)]
pub struct AddedMember {
    pub user: String,
    pub token: String,
}

/// `POST /api/members/{id}/active` body.
#[derive(Debug, Deserialize)]
pub struct SetActiveReq {
    pub active: bool,
}

/// `POST /api/members/{id}/name` body — a member's new display name (SQUIRE-T-0120/0126).
#[derive(Debug, Deserialize)]
pub struct RenameReq {
    pub display_name: String,
}

/// Map an [`AuthError`] from a member-admin call onto an HTTP status (Forbidden → 403, else 401).
fn auth_status(err: AuthError) -> StatusCode {
    match err {
        AuthError::Forbidden => StatusCode::FORBIDDEN,
        AuthError::MissingToken | AuthError::BadToken | AuthError::WrongTenant => {
            StatusCode::UNAUTHORIZED
        }
        AuthError::Throttled { .. } => StatusCode::TOO_MANY_REQUESTS,
        AuthError::WeakSecret => StatusCode::BAD_REQUEST,
    }
}

/// `GET /api/members` (Knight-only) — every household member with role, active flag, and the
/// last-editor audit answering "who added / last changed this member".
pub async fn list_members(
    State(state): State<Arc<KeepState>>,
    _op: Operator,
) -> Json<Vec<MemberRow>> {
    let snap = state.snapshot();
    let mut guard = state.store.lock().expect("store mutex poisoned");
    let rows = snap
        .users
        .iter()
        .map(|u| {
            let audit = store::user_audit(&mut guard.connection(), u.id)
                .ok()
                .flatten()
                .map(AuditView::from)
                .unwrap_or_default();
            MemberRow {
                user: u.id.0.to_string(),
                role: u.role,
                display_name: u.display_name.clone(),
                active: u.active,
                tincture: tincture_of(&guard, u.id),
                audit,
            }
        })
        .collect();
    Json(rows)
}

/// `POST /api/members` (Knight-only) — add a Knight or Squire via the identity component, audited
/// to the acting Knight, then mint a tenant-scoped token for provisioning the member's phone.
pub async fn add_member(
    State(state): State<Arc<KeepState>>,
    Operator(op): Operator,
    Json(req): Json<AddMemberReq>,
) -> Result<Json<AddedMember>, StatusCode> {
    let secret = req.initial_secret.clone();
    let added = state.identity.add_member(&op, req).map_err(auth_status)?;
    // Mint the new member's token (login with the initial secret) so a phone can be paired now.
    let login = state
        .identity
        .login(LoginReq {
            household: state.household.clone(),
            user: added.user,
            secret,
        })
        .map_err(auth_status)?;
    Ok(Json(AddedMember {
        user: added.user.0.to_string(),
        token: login.token.0,
    }))
}

/// `POST /api/members/{id}/active` (Knight-only) — de/reactivate a member via `SetUserActive`
/// (archive-not-delete), audited to the acting Knight. A missing member is a 404.
pub async fn set_active(
    State(state): State<Arc<KeepState>>,
    Operator(op): Operator,
    Path(id): Path<String>,
    Json(req): Json<SetActiveReq>,
) -> Result<StatusCode, StatusCode> {
    let uid = UserId(id.trim().parse().map_err(|_| StatusCode::BAD_REQUEST)?);
    if !state.snapshot().users.iter().any(|u| u.id == uid) {
        return Err(StatusCode::NOT_FOUND);
    }
    state
        .apply_changes(Some(op.user), &[Change::SetUserActive(uid, req.active)])
        .map_err(|_| StatusCode::CONFLICT)?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/members/{id}/name` (Knight-only) — rename a member in place. Reuses the `PutUser`
/// upsert (keeps id, role, active, and `created_*` audit; moves `updated_*`), audited to the acting
/// Knight. Blank name → 400; a missing member → 404.
pub async fn rename(
    State(state): State<Arc<KeepState>>,
    Operator(op): Operator,
    Path(id): Path<String>,
    Json(req): Json<RenameReq>,
) -> Result<StatusCode, StatusCode> {
    let name = req.display_name.trim();
    if name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let uid = UserId(id.trim().parse().map_err(|_| StatusCode::BAD_REQUEST)?);
    let user = state
        .snapshot()
        .users
        .iter()
        .find(|u| u.id == uid)
        .cloned()
        .ok_or(StatusCode::NOT_FOUND)?;
    let updated = User {
        display_name: name.to_string(),
        ..user
    };
    state
        .apply_changes(Some(op.user), &[Change::PutUser(updated)])
        .map_err(|_| StatusCode::CONFLICT)?;
    Ok(StatusCode::NO_CONTENT)
}

/// The stored tincture for `user`, or the default (SQUIRE-T-0136).
pub fn tincture_of(store: &Store<SystemClock>, user: UserId) -> String {
    store
        .get_setting(&config_keys::tincture(user))
        .as_deref()
        .and_then(valid_tincture)
        .unwrap_or(DEFAULT_TINCTURE)
        .to_string()
}

/// `POST /api/members/{id}/tincture` body.
#[derive(Debug, Deserialize)]
pub struct TinctureReq {
    pub tincture: String,
}

/// `POST /api/members/{id}/tincture` (Knight-only) — set the colour a squire's screens wear.
/// Presentation only, so it lives in the config table (audited to the acting Knight), not the
/// domain model. 400 for an unknown tincture, 404 for an unknown member.
pub async fn set_tincture(
    State(state): State<Arc<KeepState>>,
    Operator(op): Operator,
    Path(id): Path<String>,
    Json(req): Json<TinctureReq>,
) -> Result<StatusCode, StatusCode> {
    let tincture = valid_tincture(&req.tincture).ok_or(StatusCode::BAD_REQUEST)?;
    let uid = UserId(id.trim().parse().map_err(|_| StatusCode::BAD_REQUEST)?);
    let store = state.store.lock().expect("store mutex poisoned");
    if !store.snapshot().users.iter().any(|u| u.id == uid) {
        return Err(StatusCode::NOT_FOUND);
    }
    store
        .set_setting(&config_keys::tincture(uid), tincture, Some(op.user))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}
