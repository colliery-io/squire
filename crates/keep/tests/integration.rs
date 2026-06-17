//! End-to-end + guarantee suite for the Keep (SQUIRE-T-0030): the full author → add-member →
//! claim → approve → state flow over the (loopback) admin API, the read-only event-log inspector
//! with `actor` attribution, the "who set / who added" audit answers, and the A-0008 guarantees
//! (engine-direct: no `api` dependency; loopback-only bind).

use std::sync::Arc;

use keep::{admin_addr, router, KeepState};

use axum::body::Body;
use axum::http::{Request, StatusCode};

use domain_core::contract::{
    Assignment, Availability, Cadence, ClaimId, Clock, Command, Completion, HouseholdHandle, ItemId,
    Quest, QuestId, RedeemableItem, RegisterHouseholdReq, Schedule, UserId,
};
use identity::TokenSigner;
use store::tenant::Backend;

use http_body_util::BodyExt;
use serde_json::{json, to_value, Value};
use tower::ServiceExt;

fn keep() -> (Arc<KeepState>, u128, String, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let backend = Backend::Sqlite { dir: dir.path().to_path_buf() };
    let state = KeepState::local(backend, HouseholdHandle("keep".into()), TokenSigner::new(b"keep-test-key"), 60 * 60 * 1000).expect("wire keep");
    let reg = state
        .identity
        .register(RegisterHouseholdReq { household_name: "Keep".into(), admin_name: "Arthur".into(), admin_secret: "x".into() })
        .expect("register admin");
    (state.clone(), reg.admin.0, reg.token.0, dir)
}

async fn send(state: &Arc<KeepState>, method: &str, path: &str, token: Option<&str>, body: Option<Value>) -> (StatusCode, Value) {
    let mut b = Request::builder().method(method).uri(path);
    if let Some(t) = token {
        b = b.header("authorization", format!("Bearer {t}"));
    }
    let req = match body {
        Some(v) => b.header("content-type", "application/json").body(Body::from(v.to_string())).unwrap(),
        None => b.body(Body::empty()).unwrap(),
    };
    let resp = router(state.clone()).oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let value = if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap_or(Value::Null) };
    (status, value)
}

fn quest(id: u128) -> Quest {
    Quest {
        id: QuestId(id),
        title: "Tidy room".into(),
        description: None,
        category: None,
        reward: 5,
        cadence: Cadence::Recurring(Schedule::Daily),
        assignment: Assignment::AllSquires,
        completion: Completion::EachAssignee,
        auto_approve: false,
        repeatable_within_day: false,
        active: true,
        icon: None,
    }
}

fn free_item(id: u128) -> RedeemableItem {
    RedeemableItem { id: ItemId(id), name: "Sticker".into(), description: None, cost: 0, gate: None, availability: Availability::Repeatable, active: true, icon: None }
}

// ─── the full flow, over the Keep API ────────────────────────────────────────────────────────

#[tokio::test]
async fn author_add_member_claim_approve_state_end_to_end() {
    let (state, admin, token, _dir) = keep();

    // Author a quest + item over the admin API.
    assert_eq!(send(&state, "POST", "/api/quests", Some(&token), Some(to_value(quest(100)).unwrap())).await.0, StatusCode::OK);
    assert_eq!(send(&state, "POST", "/api/items", Some(&token), Some(to_value(free_item(200)).unwrap())).await.0, StatusCode::OK);

    // Add a Squire over the API.
    let (_st, added) = send(&state, "POST", "/api/members", Some(&token), Some(json!({ "role": "Squire", "display_name": "Gareth", "initial_secret": "g" }))).await;
    let squire = UserId(added["user"].as_str().unwrap().parse().unwrap());

    // Seed a claim (squire submission has no Keep endpoint — the phone does that; here engine-direct).
    let today = state.clock.today();
    state.commit(None, Command::SubmitClaim { claim_id: ClaimId(9001), squire, quest_id: QuestId(100), on: today }).expect("seed claim");

    // The Keep approves over the API → the credit lands.
    assert_eq!(send(&state, "POST", "/api/review/claim", Some(&token), Some(json!({ "claim_id": 9001, "decision": "approve" }))).await.0, StatusCode::OK);

    // State reflects the credit (via the cross-Squire review summary).
    let (_st, review) = send(&state, "GET", "/api/review", Some(&token), None).await;
    let summary = review["squires"].as_array().unwrap().iter().find(|s| s["squire"].as_u64().unwrap() as u128 == squire.0).unwrap();
    assert_eq!(summary["balance"], 5, "approval credited the reward");

    // "Who set the quest?" — answerable from the quest list audit.
    let (_st, quests) = send(&state, "GET", "/api/quests", Some(&token), None).await;
    let q = quests.as_array().unwrap().iter().find(|r| r["quest"]["id"] == 100).unwrap();
    assert_eq!(q["audit"]["created_by"], admin.to_string());

    // "Who added the member?" — answerable from the members list audit.
    let (_st, members) = send(&state, "GET", "/api/members", Some(&token), None).await;
    let m = members.as_array().unwrap().iter().find(|r| r["user"] == squire.0.to_string()).unwrap();
    assert_eq!(m["audit"]["created_by"], admin.to_string());
}

// ─── event-log inspector (with actor attribution) ────────────────────────────────────────────

#[tokio::test]
async fn log_inspector_shows_the_ordered_trail_with_actor() {
    let (state, admin, token, _dir) = keep();
    send(&state, "POST", "/api/quests", Some(&token), Some(to_value(quest(100)).unwrap())).await;
    send(&state, "POST", "/api/items", Some(&token), Some(to_value(free_item(200)).unwrap())).await;
    let (_st, added) = send(&state, "POST", "/api/members", Some(&token), Some(json!({ "role": "Squire", "display_name": "Gareth", "initial_secret": "g" }))).await;
    let squire = UserId(added["user"].as_str().unwrap().parse().unwrap());

    let today = state.clock.today();
    state.commit(None, Command::SubmitClaim { claim_id: ClaimId(9001), squire, quest_id: QuestId(100), on: today }).expect("claim");
    send(&state, "POST", "/api/review/claim", Some(&token), Some(json!({ "claim_id": 9001, "decision": "approve" }))).await;
    send(&state, "POST", "/api/redeem", Some(&token), Some(json!({ "command_id": 1, "squire": squire.0, "item_id": 200 }))).await;

    // Quest log: a CompletionClaimed followed by a CompletionApproved whose actor is the Knight.
    let (st, qlog) = send(&state, "GET", "/api/log/quest/100", Some(&token), None).await;
    assert_eq!(st, StatusCode::OK);
    let events = qlog.as_array().unwrap();
    assert!(events.iter().any(|e| e.get("CompletionClaimed").is_some()), "claim in the trail");
    let approved = events.iter().find_map(|e| e.get("CompletionApproved")).expect("approval in the trail");
    assert_eq!(approved["actor"].as_u64().unwrap() as u128, admin, "the approving Knight is recorded as actor");

    // Item log: the direct redeem shows up as ItemRedeemed.
    let (_st, ilog) = send(&state, "GET", "/api/log/item/200", Some(&token), None).await;
    assert!(ilog.as_array().unwrap().iter().any(|e| e.get("ItemRedeemed").is_some()), "redeem in the item trail");

    // The inspector is Knight-only.
    let (_st, sq) = send(&state, "POST", "/api/members", Some(&token), Some(json!({ "role": "Squire", "display_name": "Spy", "initial_secret": "p" }))).await;
    let squire_token = sq["token"].as_str().unwrap();
    assert_eq!(send(&state, "GET", "/api/log/quest/100", Some(squire_token), None).await.0, StatusCode::FORBIDDEN);
}

// ─── A-0008 guarantees ───────────────────────────────────────────────────────────────────────

/// The Keep is engine-direct: its command path must not route through the network `api` crate, so
/// the `keep` crate must not depend on `api` at all (compile-time guarantee, asserted on the manifest).
#[test]
fn keep_does_not_depend_on_the_network_api() {
    let manifest = include_str!("../Cargo.toml");
    assert!(
        !manifest.contains("../api"),
        "the Keep must not depend on the `api` crate (engine-direct, A-0008)"
    );
}

/// The admin surface binds loopback only — the load-bearing control that keeps authoring off the
/// network even though it is delivered over HTTP (A-0008 / AR-8).
#[test]
fn admin_surface_binds_loopback_only() {
    let addr = admin_addr(4920);
    assert!(addr.ip().is_loopback());
    assert!(!addr.ip().is_unspecified());
}
