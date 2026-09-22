//! The signed-in operator's own account (SQUIRE-T-0131): change **my** login secret.
//!
//! Until this, the only way to rotate a secret was the `reset_secret` recovery tool — run by
//! whoever holds the data dir, with the server stopped. This is the everyday path: the operator
//! proves the *current* secret and picks a new one, from the Keep's Settings tab.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use identity::AuthError;

use crate::{KeepState, Operator};

/// `POST /api/me/secret` body.
#[derive(Deserialize)]
pub struct ChangeSecretReq {
    pub current_secret: String,
    pub new_secret: String,
}

/// `POST /api/me/secret` (Knight-only, **self only** — the target is the verified caller, never a
/// request field) — replace the operator's login secret. `204` on success; `401` wrong current
/// secret; `400` new secret too short; `429` locked (it shares the login throttle, SQUIRE-T-0130,
/// so this is not a side door for guessing). The session cookie stays valid: tokens are not revoked.
pub async fn change_my_secret(
    State(state): State<Arc<KeepState>>,
    Operator(op): Operator,
    Json(req): Json<ChangeSecretReq>,
) -> StatusCode {
    match state
        .identity
        .change_secret(&op, &req.current_secret, &req.new_secret)
    {
        Ok(()) => {
            tracing::info!(user = op.user.0, "operator changed their login secret");
            StatusCode::NO_CONTENT
        }
        Err(AuthError::WeakSecret) => StatusCode::BAD_REQUEST,
        Err(AuthError::Throttled { retry_after_s }) => {
            tracing::warn!(
                user = op.user.0,
                retry_after_s,
                "change-secret refused: account locked after repeated failures"
            );
            StatusCode::TOO_MANY_REQUESTS
        }
        Err(_) => StatusCode::UNAUTHORIZED,
    }
}
