//! # Squire `api` — the LAN HTTP server (SQUIRE-T-0014)
//!
//! This crate is Squire's **only network seam and trust boundary**: every request crosses an
//! authenticated edge ([`auth`]) and is reduced to a verified [`identity::Principal`] before
//! any handler logic runs. The Squire ([`squire`], T-0015), Knight ([`knight`], T-0016), and
//! control-plane ([`control`], T-0017) endpoints are all wired here, atop the framework, app
//! state, identity port, and auth/tenant layer; `GET /health` is the unauthenticated liveness
//! probe.
//!
//! ## HTTP framework: axum (decision recorded for SQUIRE-S-0003)
//!
//! axum is the de-facto Rust HTTP standard: it composes directly with the `tower`/`tower-http`
//! middleware ecosystem (auth, tracing, limits) and, critically for this codebase, it is
//! testable without a socket — the [`router`] is driven in tests via
//! `tower::ServiceExt::oneshot`, so the whole request path is exercised in-process.

pub mod auth;
pub mod control;
pub mod identity;
pub mod knight;
pub mod squire;
pub mod state;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;

pub use state::AppState;

/// Build the application [`Router`] with the shared [`AppState`] wired in.
///
/// `GET /health` plus the Squire-role (T-0015) and Knight-role (T-0016) feature routes, each
/// gated by the [`auth`] extractors. Returned as a fully-`State`-wrapped `Router` so callers
/// (and tests) can serve or `oneshot` it directly.
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        // ── Control-plane endpoints (SQUIRE-T-0017) ────────────────────────────
        // /register and /login are unauthenticated (bootstrap / token exchange);
        // /members is Knight-only (the extractor 403s a Squire token).
        .route("/register", post(control::register))
        .route("/login", post(control::login))
        .route("/members", post(control::add_member))
        // ── Squire-role endpoints (SQUIRE-T-0015) ──────────────────────────────
        .route("/state", get(squire::get_state))
        .route("/claims", post(squire::submit_claim))
        .route("/redemption-requests", post(squire::request_redemption))
        // ── Knight-role privileged endpoints (SQUIRE-T-0016) ───────────────────
        .route("/admin/review-claim", post(knight::review_claim))
        .route("/admin/review-redemption", post(knight::review_redemption))
        .route("/admin/redeem", post(knight::redeem))
        .route("/admin/adjust", post(knight::adjust))
        .route("/admin/mark-done", post(knight::mark_done))
        .route("/household-review", get(knight::household_review))
        .with_state(state)
}

/// Liveness probe — no auth, no state touched. 200 `"ok"`.
async fn health() -> &'static str {
    "ok"
}

/// Bind the configurable LAN `addr` (`host:port`) and serve the [`router`] until the process
/// exits. The library [`router`] is what tests exercise; this is the thin production entry.
pub async fn serve(
    state: Arc<AppState>,
    addr: SocketAddr,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router(state)).await?;
    Ok(())
}
