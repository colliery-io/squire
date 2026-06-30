//! Per-request **access log** for the LAN api (SQUIRE-T-0127).
//!
//! The events table only records *successful* state changes, and the text log is boot-only, so a
//! phone retrying a rejected `POST /redemption-requests` left no trace. This middleware closes that
//! gap: every request emits one `tracing` line with method, path, status, latency, the
//! authenticated caller, and a coarse outcome (ok / rejected / error). It logs **no** secrets — no
//! `Authorization` header, no request/response bodies.
//!
//! ## Identity without a second verification
//!
//! The authenticated [`identity::Principal`] is resolved inside the [`crate::auth`] extractors,
//! *after* this outer middleware has already handed the request down the stack — so the middleware
//! cannot see the extractor's return value. Rather than verify the token a second time (the hot
//! path), the middleware seeds a shared [`LogSlot`] into the request's extensions; the auth
//! extractor fills it on success, and the middleware reads it back once the response is in hand.
//! Unauthenticated routes (or auth failures) simply leave the slot empty.

use std::sync::{Arc, OnceLock};
use std::time::Instant;

use axum::extract::Request;
use axum::http::request::Parts;
use axum::middleware::Next;
use axum::response::Response;

use domain_core::contract::{Role, UserId};
use identity::Principal;

/// The verified caller recorded for the access log: just the id + role (no token, no name lookup —
/// resolving the display name would mean a store snapshot on every request).
#[derive(Clone, Copy)]
pub(crate) struct LogIdentity {
    pub user: UserId,
    pub role: Role,
}

/// A write-once slot the auth extractor fills with the verified caller and the access-log
/// middleware reads back. Shared (an `Arc`) between the request extensions the extractor sees and
/// the middleware's retained handle; `OnceLock` so it is set at most once per request.
#[derive(Clone)]
pub(crate) struct LogSlot(Arc<OnceLock<LogIdentity>>);

impl LogSlot {
    /// Record the verified caller (no-op if already set). Called by the auth extractors.
    pub(crate) fn record(&self, principal: &Principal) {
        let _ = self.0.set(LogIdentity {
            user: principal.user,
            role: principal.role,
        });
    }
}

/// Fetch the access-log slot a request carries, if the [`access_log`] middleware seeded one. The
/// auth extractors call this to record the caller; absent (e.g. a unit test driving the router
/// without the layer) it is simply skipped.
pub(crate) fn slot_of(parts: &Parts) -> Option<&LogSlot> {
    parts.extensions.get::<LogSlot>()
}

/// Map an HTTP status to a coarse, greppable outcome word for the log line.
fn outcome_of(status: u16) -> &'static str {
    match status {
        s if s < 400 => "ok",
        s if s < 500 => "rejected", // client error: bad input, auth, insufficient funds, …
        _ => "error",               // 5xx: our fault
    }
}

/// axum middleware (mount via `axum::middleware::from_fn`) that logs one line per request.
///
/// Seeds a [`LogSlot`] for the auth extractor, runs the handler, then logs method / path / status /
/// latency / caller / outcome. `/health` (the liveness probe — hit often, says nothing) drops to
/// `debug` so it does not drown the log; everything else logs at `info`.
pub(crate) async fn access_log(mut req: Request, next: Next) -> Response {
    let slot = LogSlot(Arc::new(OnceLock::new()));
    req.extensions_mut().insert(slot.clone());

    let method = req.method().clone();
    let path = req.uri().path().to_owned();
    let start = Instant::now();

    let resp = next.run(req).await;

    let status = resp.status().as_u16();
    let latency_ms = start.elapsed().as_millis() as u64;
    let outcome = outcome_of(status);

    // `user`/`role` are absent on unauthenticated routes (/health, /login, /pair, …) or when auth
    // failed before the extractor recorded anyone — render a placeholder so the field is stable.
    let (user, role) = match slot.0.get() {
        Some(id) => (id.user.0.to_string(), format!("{:?}", id.role)),
        None => ("-".to_string(), "-".to_string()),
    };

    if path == "/health" {
        tracing::debug!(
            target: "squire::access",
            %method, %path, status, latency_ms, %user, %role, outcome,
            "request",
        );
    } else {
        tracing::info!(
            target: "squire::access",
            %method, %path, status, latency_ms, %user, %role, outcome,
            "request",
        );
    }

    resp
}
