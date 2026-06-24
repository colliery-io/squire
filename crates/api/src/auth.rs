//! The auth / tenant-resolution layer: the trust boundary every request crosses.
//!
//! [`Auth`] is an axum extractor that reads `Authorization: Bearer <token>` + `X-Household:
//! <handle>`, resolves the tenant (MVP: a single local tenant, so the handle is passed
//! through as-is), and calls [`Identity::verify`]. The verified [`Principal`] is what handler
//! logic runs against — no handler ever sees a raw token.
//!
//! Status mapping (acceptance criteria):
//! * missing / garbled headers, [`AuthError::MissingToken`], [`AuthError::BadToken`],
//!   [`AuthError::WrongTenant`] → **401 Unauthorized** (we cannot authenticate the caller);
//! * [`AuthError::Forbidden`] (wrong role) → **403 Forbidden** (authenticated, not allowed).
//!
//! Routes pick the gate they need: [`Auth`] for any authenticated caller, or
//! [`RequireKnight`] / [`RequireSquire`] to additionally demand a role (403 on mismatch).

use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::http::header::{HeaderMap, AUTHORIZATION};
use axum::http::request::Parts;
use axum::http::StatusCode;

use domain_core::contract::{AuthToken, HouseholdHandle, Role};

use identity::{AuthError, Principal};

use crate::state::AppState;

/// Header carrying the opaque household handle a paired device presents to route to its tenant.
const HOUSEHOLD_HEADER: &str = "x-household";

/// Map an [`AuthError`] to its HTTP status: only [`AuthError::Forbidden`] is a 403; every other
/// variant means we could not authenticate the caller and is a 401.
fn status_for(err: AuthError) -> StatusCode {
    match err {
        AuthError::Forbidden => StatusCode::FORBIDDEN,
        AuthError::MissingToken | AuthError::BadToken | AuthError::WrongTenant => {
            StatusCode::UNAUTHORIZED
        }
    }
}

/// Pull the bearer token and household handle out of the request headers, failing with
/// [`AuthError::MissingToken`] if either is absent or malformed.
fn extract_credentials(headers: &HeaderMap) -> Result<(HouseholdHandle, AuthToken), AuthError> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(AuthError::MissingToken)?;
    // Case-insensitive "Bearer " scheme prefix; the remainder is the opaque token.
    let token = auth
        .strip_prefix("Bearer ")
        .or_else(|| auth.strip_prefix("bearer "))
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .ok_or(AuthError::MissingToken)?;

    let handle = headers
        .get(HOUSEHOLD_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|h| !h.is_empty())
        .ok_or(AuthError::MissingToken)?;

    Ok((
        HouseholdHandle(handle.to_string()),
        AuthToken(token.to_string()),
    ))
}

/// Verify a request's credentials against the state's [`Identity`], yielding a [`Principal`].
/// Shared by all three extractors below.
fn verify(state: &AppState, headers: &HeaderMap) -> Result<Principal, AuthError> {
    let (household, token) = extract_credentials(headers)?;
    // MVP: single local tenant — the handle is passed straight through to `verify`, which
    // also checks it matches the token's household (rejecting cross-tenant replay).
    state.identity.verify(&household, &token)
}

/// Extractor yielding the verified [`Principal`] for any authenticated caller (no role gate).
/// 401 on missing / invalid credentials.
pub struct Auth(pub Principal);

impl FromRequestParts<Arc<AppState>> for Auth {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        verify(state, &parts.headers).map(Auth).map_err(status_for)
    }
}

/// Extractor that requires the caller to be a [`Role::Knight`]: 401 if unauthenticated,
/// **403** if authenticated but not a Knight.
pub struct RequireKnight(pub Principal);

impl FromRequestParts<Arc<AppState>> for RequireKnight {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        require_role(state, &parts.headers, Role::Knight).map(RequireKnight)
    }
}

/// Extractor that requires the caller to be a [`Role::Squire`]: 401 if unauthenticated,
/// **403** if authenticated but not a Squire.
pub struct RequireSquire(pub Principal);

impl FromRequestParts<Arc<AppState>> for RequireSquire {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        require_role(state, &parts.headers, Role::Squire).map(RequireSquire)
    }
}

/// Verify, then enforce a role: an authenticated caller with the wrong role becomes
/// [`AuthError::Forbidden`] → 403, while authentication failures stay 401.
fn require_role(
    state: &AppState,
    headers: &HeaderMap,
    role: Role,
) -> Result<Principal, StatusCode> {
    let principal = verify(state, headers).map_err(status_for)?;
    if principal.has_role(role) {
        Ok(principal)
    } else {
        Err(status_for(AuthError::Forbidden))
    }
}
