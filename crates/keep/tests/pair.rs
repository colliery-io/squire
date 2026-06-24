//! Device-pairing tests for the Keep (SQUIRE-T-0045 / ADR A-0010): a Knight mints a one-time
//! pairing code + QR for a member; a Squire operator can't; the minted code actually works at the
//! identity layer's consume (the same consume the api's `POST /pair` calls).

use std::sync::Arc;

use keep::{router, KeepState};

use axum::body::Body;
use axum::http::{Request, StatusCode};

use domain_core::contract::{HouseholdHandle, RegisterHouseholdReq};
use identity::TokenSigner;
use store::tenant::Backend;

use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

fn keep() -> (Arc<KeepState>, HouseholdHandle, String, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let backend = Backend::Sqlite {
        dir: dir.path().to_path_buf(),
    };
    let handle = HouseholdHandle("keep".into());
    let state = KeepState::local(
        backend,
        handle.clone(),
        TokenSigner::new(b"keep-test-key"),
        60 * 60 * 1000,
    )
    .expect("wire keep");
    let reg = state
        .identity
        .register(RegisterHouseholdReq {
            household_name: "Keep".into(),
            admin_name: "Arthur".into(),
            admin_secret: "x".into(),
        })
        .expect("register admin");
    (state.clone(), handle, reg.token.0, dir)
}

async fn send(
    state: &Arc<KeepState>,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut b = Request::builder().method(method).uri(path);
    if let Some(t) = token {
        b = b.header("authorization", format!("Bearer {t}"));
    }
    let req = match body {
        Some(v) => b
            .header("content-type", "application/json")
            .body(Body::from(v.to_string()))
            .unwrap(),
        None => b.body(Body::empty()).unwrap(),
    };
    let resp = router(state.clone()).oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, value)
}

async fn add_squire(state: &Arc<KeepState>, token: &str) -> Value {
    let (st, sq) = send(
        state,
        "POST",
        "/api/members",
        Some(token),
        Some(json!({ "role": "Squire", "display_name": "Gawain", "initial_secret": "s" })),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    sq
}

#[tokio::test]
async fn knight_mints_a_pairing_qr_and_the_code_consumes_to_that_member() {
    let (state, handle, token, _dir) = keep();
    let squire = add_squire(&state, &token).await;
    let squire_id = squire["user"].as_str().unwrap().to_string();

    // Knight mints a pairing code + QR for the Squire.
    let (st, p) = send(
        &state,
        "POST",
        "/api/pair/codes",
        Some(&token),
        Some(json!({ "user": squire_id })),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let code = p["code"].as_str().unwrap().to_string();
    assert!(
        !code.is_empty(),
        "a plaintext code is returned (once) for the QR"
    );
    assert!(
        p["qr_svg"].as_str().unwrap().contains("<svg"),
        "renders an SVG QR"
    );
    assert!(
        p["payload"].as_str().unwrap().starts_with("squire://pair?"),
        "the QR encodes a squire://pair URI"
    );
    assert!(p["expires_at"].as_i64().unwrap() > 0);

    // The minted code consumes (the SAME path the api's POST /pair calls) to the Squire's token.
    let paired = state
        .identity
        .consume_pairing_code(&handle, &code)
        .expect("code consumes");
    assert_eq!(paired.user.0.to_string(), squire_id);
    assert_eq!(paired.role, domain_core::contract::Role::Squire);

    // Single-use: a second mint+consume needs a fresh code; replaying the old one fails.
    assert!(
        state.identity.consume_pairing_code(&handle, &code).is_err(),
        "code is single-use"
    );
}

#[tokio::test]
async fn a_squire_operator_cannot_mint_pairing_codes() {
    let (state, _handle, token, _dir) = keep();
    let squire = add_squire(&state, &token).await;
    let squire_token = squire["token"].as_str().unwrap().to_string();
    let squire_id = squire["user"].as_str().unwrap().to_string();

    let (st, _) = send(
        &state,
        "POST",
        "/api/pair/codes",
        Some(&squire_token),
        Some(json!({ "user": squire_id })),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN, "minting is Knight-only");
}

#[tokio::test]
async fn unauthenticated_mint_is_401() {
    let (state, _handle, _token, _dir) = keep();
    let (st, _) = send(
        &state,
        "POST",
        "/api/pair/codes",
        Some("garbage"),
        Some(json!({ "user": "1" })),
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
}
