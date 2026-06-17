//! # Squire `api` — the LAN HTTP server (SQUIRE-T-0014)
//!
//! This crate is Squire's **only network seam and trust boundary**: every request crosses an
//! authenticated edge ([`auth`]) and is reduced to a verified [`identity::Principal`] before
//! any handler logic runs. The Squire/Knight/control-plane endpoints land in later tasks
//! (T-0015/16/17); this scaffold provides the framework, app state, identity port, and the
//! auth/tenant layer, plus a single real route — `GET /health`.
//!
//! ## HTTP framework: axum (decision recorded for SQUIRE-S-0003)
//!
//! axum is the de-facto Rust HTTP standard: it composes directly with the `tower`/`tower-http`
//! middleware ecosystem (auth, tracing, limits) and, critically for this codebase, it is
//! testable without a socket — the [`router`] is driven in tests via
//! `tower::ServiceExt::oneshot`, so the whole request path is exercised in-process.

pub mod auth;
pub mod identity;
pub mod state;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::routing::get;
use axum::Router;

pub use state::AppState;

/// Build the application [`Router`] with the shared [`AppState`] wired in.
///
/// Only `GET /health` exists today (returns 200 `"ok"`); the protected feature routes are
/// added by later tasks using the [`auth`] extractors. Returned as a fully-`State`-wrapped
/// `Router` so callers (and tests) can serve or `oneshot` it directly.
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
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
