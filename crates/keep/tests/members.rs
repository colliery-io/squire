//! Member-administration tests for the Keep (SQUIRE-T-0028): add Knights/Squires through the real
//! identity flow, mint their tokens, audit "who added X", de/reactivate, and the Knight-only gate.

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

fn keep() -> (Arc<KeepState>, u128, String, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let backend = Backend::Sqlite {
        dir: dir.path().to_path_buf(),
    };
    let state = KeepState::local(
        backend,
        HouseholdHandle("keep".into()),
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
    (state.clone(), reg.admin.0, reg.token.0, dir)
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

async fn add(
    state: &Arc<KeepState>,
    token: &str,
    role: &str,
    name: &str,
    secret: &str,
) -> (StatusCode, Value) {
    send(
        state,
        "POST",
        "/api/members",
        Some(token),
        Some(json!({ "role": role, "display_name": name, "initial_secret": secret })),
    )
    .await
}

#[tokio::test]
async fn add_members_mint_tokens_and_audit_to_the_knight() {
    let (state, admin, token, _dir) = keep();

    // Add a second Knight; the minted token works as a Knight (proves token minting).
    let (st, k2) = add(&state, &token, "Knight", "Lancelot", "k2").await;
    assert_eq!(st, StatusCode::OK);
    let k2_token = k2["token"].as_str().unwrap().to_string();
    let (st, who) = send(&state, "GET", "/api/whoami", Some(&k2_token), None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(
        who["role"], "Knight",
        "the minted token authenticates as a Knight"
    );

    // Add a Squire.
    let (st, sq) = add(&state, &token, "Squire", "Gareth", "s").await;
    assert_eq!(st, StatusCode::OK);
    let squire_id = sq["user"].as_str().unwrap().to_string();

    // The household now has 3 members; the added ones are audited to the admin Knight.
    let (st, list) = send(&state, "GET", "/api/members", Some(&token), None).await;
    assert_eq!(st, StatusCode::OK);
    let rows = list.as_array().unwrap();
    assert_eq!(rows.len(), 3, "admin + Knight + Squire");
    let squire = rows.iter().find(|r| r["user"] == squire_id).unwrap();
    assert_eq!(squire["role"], "Squire");
    assert_eq!(
        squire["audit"]["created_by"],
        admin.to_string(),
        "who added the Squire = the admin"
    );
}

#[tokio::test]
async fn unauthenticated_add_is_401() {
    let (state, _admin, _token, _dir) = keep();
    let (st, _) = add(&state, "garbage-token", "Squire", "X", "s").await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_squire_cannot_administer_members() {
    let (state, _admin, token, _dir) = keep();
    // Add a Squire and use its minted token — the Operator gate is Knight-only.
    let (_st, sq) = add(&state, &token, "Squire", "Gareth", "s").await;
    let squire_token = sq["token"].as_str().unwrap().to_string();
    let (st, _) = add(&state, &squire_token, "Squire", "Mordred", "m").await;
    assert_eq!(
        st,
        StatusCode::FORBIDDEN,
        "a Squire token cannot add members"
    );
}

#[tokio::test]
async fn deactivate_and_reactivate_a_member() {
    let (state, _admin, token, _dir) = keep();
    let (_st, sq) = add(&state, &token, "Squire", "Gareth", "s").await;
    let id = sq["user"].as_str().unwrap().to_string();

    // Deactivate (archive-not-delete) → 204; list shows inactive.
    let (st, _) = send(
        &state,
        "POST",
        &format!("/api/members/{id}/active"),
        Some(&token),
        Some(json!({ "active": false })),
    )
    .await;
    assert_eq!(st, StatusCode::NO_CONTENT);
    let (_st, list) = send(&state, "GET", "/api/members", Some(&token), None).await;
    let row = list
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["user"] == id)
        .unwrap();
    assert_eq!(row["active"], false);

    // Reactivate → active again.
    let (st, _) = send(
        &state,
        "POST",
        &format!("/api/members/{id}/active"),
        Some(&token),
        Some(json!({ "active": true })),
    )
    .await;
    assert_eq!(st, StatusCode::NO_CONTENT);
    let (_st, list) = send(&state, "GET", "/api/members", Some(&token), None).await;
    let row = list
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["user"] == id)
        .unwrap();
    assert_eq!(row["active"], true);

    // A missing member is a 404.
    let (st, _) = send(
        &state,
        "POST",
        "/api/members/424242/active",
        Some(&token),
        Some(json!({ "active": false })),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
}
