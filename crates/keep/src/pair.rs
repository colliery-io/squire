//! Device pairing (SQUIRE-T-0045 / ADR SQUIRE-A-0010): the Keep — the authenticated, on-computer
//! minting authority — mints a **one-time pairing code** for a chosen member and renders it as a
//! **QR** for the member's phone to scan. The phone then exchanges the code at the LAN api's
//! `POST /pair` (SQUIRE-T-0044) for a per-user token.
//!
//! Minting goes through the same [`identity`](identity) port the Keep already holds (Knight-only;
//! the [`Operator`] extractor gates the route). Only the plaintext code transits this response (for
//! the QR) — nothing is persisted here beyond the hashed code the identity layer already stored.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use qrcode::render::svg;
use qrcode::QrCode;
use serde::{Deserialize, Serialize};

use domain_core::contract::UserId;
use identity::AuthError;

use crate::{KeepState, Operator};

/// `POST /api/pair/codes` body — the target member (`UserId` as a decimal string, JSON-safe).
#[derive(Debug, Deserialize)]
pub struct MintReq {
    pub user: String,
}

/// The minted pairing code, everything the phone needs (carried in the QR), and the rendered QR SVG.
#[derive(Debug, Serialize)]
pub struct PairCodeView {
    pub user: String,
    pub code: String,
    /// Expiry, unix millis (30-min TTL; ADR A-0010).
    pub expires_at: i64,
    pub household: String,
    /// The LAN address the phone should reach the **api** at (not the loopback Keep). From
    /// `SQUIRE_PAIR_HOST` / `SQUIRE_PAIR_PORT` (falling back to `API_PORT`), defaulting to the
    /// emulator-friendly `10.0.2.2:8080`. Real LAN-IP autodiscovery is the phone's job (NSD, T-0046).
    pub host: String,
    pub port: u16,
    /// The exact string encoded in the QR — a `squire://pair?...` URI the phone (T-0046) parses.
    pub payload: String,
    /// A self-contained SVG of the QR, for the page to drop straight into the DOM.
    pub qr_svg: String,
}

/// Map an [`AuthError`] onto an HTTP status (Forbidden → 403, else 401).
fn auth_status(err: AuthError) -> StatusCode {
    match err {
        AuthError::Forbidden => StatusCode::FORBIDDEN,
        AuthError::MissingToken | AuthError::BadToken | AuthError::WrongTenant => {
            StatusCode::UNAUTHORIZED
        }
    }
}

/// The api host/port to advertise in the QR. Configurable so a real deployment can publish the
/// computer's LAN IP; defaults suit the emulator demo (`10.0.2.2` is the emulator's host loopback).
fn advertised_addr() -> (String, u16) {
    let host = std::env::var("SQUIRE_PAIR_HOST").unwrap_or_else(|_| "10.0.2.2".to_string());
    let port = std::env::var("SQUIRE_PAIR_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .or_else(|| std::env::var("API_PORT").ok().and_then(|v| v.parse().ok()))
        .unwrap_or(8080);
    (host, port)
}

/// `POST /api/pair/codes` (Knight-only) — mint a one-time pairing code for `user` and render its QR.
///
/// The base64url code and the sanitized household handle are already URL-safe (`[A-Za-z0-9-_]`),
/// so the `squire://pair?...` payload needs no escaping.
pub async fn mint_pair_code(
    State(state): State<Arc<KeepState>>,
    Operator(op): Operator,
    Json(req): Json<MintReq>,
) -> Result<Json<PairCodeView>, StatusCode> {
    let user = UserId(req.user.trim().parse().map_err(|_| StatusCode::BAD_REQUEST)?);
    let minted = state.identity.mint_pairing_code(&op, user).map_err(auth_status)?;

    let household = state.household.0.clone();
    let (host, port) = advertised_addr();
    let payload =
        format!("squire://pair?host={host}&port={port}&household={household}&code={}", minted.code);

    let qr_svg = QrCode::new(payload.as_bytes())
        .map(|c| {
            c.render::<svg::Color>()
                .min_dimensions(220, 220)
                .dark_color(svg::Color("#1c1b1f"))
                .light_color(svg::Color("#ffffff"))
                .build()
        })
        .unwrap_or_default();

    Ok(Json(PairCodeView {
        user: req.user,
        code: minted.code,
        expires_at: minted.expires_at.0,
        household,
        host,
        port,
        payload,
        qr_svg,
    }))
}
