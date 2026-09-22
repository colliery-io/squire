//! Review-queue + redeem + adjust tests for the Keep (SQUIRE-T-0029): the cross-Squire queue,
//! approve/reject, direct redeem (idempotent), reason-required adjust, affordability-at-approval,
//! and the Knight-only gate. Setup (authoring, claims, requests) uses the engine-direct `commit`;
//! the actions under test go over the HTTP API.

use std::sync::Arc;

use keep::{router, KeepState};

use axum::body::Body;
use axum::http::{Request, StatusCode};

use domain_core::contract::{
    AddMemberReq, Assignment, Availability, Cadence, ClaimId, Command, Completion, HouseholdHandle,
    ItemId, Projections, Quest, QuestId, RedeemableItem, RegisterHouseholdReq, RequestId, Role,
    Schedule, UserId,
};
use domain_core::Proj;
use identity::{Principal, TokenSigner};
use store::tenant::Backend;

use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

const HANDLE: &str = "keep";
const QUEST: u128 = 100; // reward 5, AllSquires, EachAssignee, manual review
const ITEM: u128 = 200; // cost 3, Repeatable

fn keep() -> (Arc<KeepState>, u128, String, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let backend = Backend::Sqlite {
        dir: dir.path().to_path_buf(),
    };
    let state = KeepState::local(
        backend,
        HouseholdHandle(HANDLE.into()),
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
    // Author a quest + an item via the engine-direct seam (authoring is not under test here).
    state
        .commit(
            Some(reg.admin),
            Command::DefineQuest(Quest {
                id: QuestId(QUEST),
                title: "Tidy room".into(),
                description: None,
                category: None,
                reward: 5,
                cash: 0,
                cadence: Cadence::Recurring(Schedule::Daily),
                assignment: Assignment::AllSquires,
                completion: Completion::EachAssignee,
                auto_approve: false,
                repeatable_within_day: false,
                active: true,
                icon: None,
                due_time: None,
            }),
        )
        .expect("author quest");
    state
        .commit(
            Some(reg.admin),
            Command::DefineItem(RedeemableItem {
                id: ItemId(ITEM),
                name: "Ice cream".into(),
                description: None,
                cost: 3,
                gate: None,
                availability: Availability::Repeatable,
                active: true,
                icon: None,
            }),
        )
        .expect("author item");
    (state.clone(), reg.admin.0, reg.token.0, dir)
}

fn add_squire(state: &Arc<KeepState>, admin: u128, name: &str, secret: &str) -> UserId {
    let caller = Principal {
        household: HouseholdHandle(HANDLE.into()),
        user: UserId(admin),
        role: Role::Knight,
    };
    state
        .identity
        .add_member(
            &caller,
            AddMemberReq {
                role: Role::Squire,
                display_name: name.into(),
                initial_secret: secret.into(),
            },
        )
        .expect("add squire")
        .user
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

fn balance(state: &Arc<KeepState>, squire: UserId) -> i64 {
    Proj::balance(&state.snapshot(), squire)
}

fn submit_claim(state: &Arc<KeepState>, squire: UserId, claim: u128) {
    let on = {
        use domain_core::contract::Clock;
        state.clock.today()
    };
    state
        .commit(
            None,
            Command::SubmitClaim {
                claim_id: ClaimId(claim),
                squire,
                quest_id: QuestId(QUEST),
                on,
            },
        )
        .expect("submit claim");
}

// ─── queue ───────────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn review_queue_lists_pending_across_squires() {
    let (state, admin, token, _dir) = keep();
    let a = add_squire(&state, admin, "Arthur Jr", "a");
    let b = add_squire(&state, admin, "Bedivere", "b");

    submit_claim(&state, a, 9001);
    state
        .commit(
            None,
            Command::RequestRedemption {
                request_id: RequestId(7001),
                squire: b,
                item_id: ItemId(ITEM),
            },
        )
        .expect("request");

    let (st, review) = send(&state, "GET", "/api/review", Some(&token), None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(
        review["squires"].as_array().unwrap().len(),
        2,
        "both squires summarised"
    );
    let claims = review["pending_claims"].as_array().unwrap();
    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0]["squire"].as_u64().unwrap() as u128, a.0);
    assert_eq!(claims[0]["quest_title"], "Tidy room");
    let reqs = review["pending_requests"].as_array().unwrap();
    assert_eq!(reqs.len(), 1);
    assert_eq!(reqs[0]["squire"].as_u64().unwrap() as u128, b.0);
    assert_eq!(reqs[0]["item_name"], "Ice cream");
}

#[tokio::test]
async fn approve_claim_credits_and_clears_pending() {
    let (state, admin, token, _dir) = keep();
    let a = add_squire(&state, admin, "Arthur Jr", "a");
    submit_claim(&state, a, 9001);

    let (st, _) = send(
        &state,
        "POST",
        "/api/review/claim",
        Some(&token),
        Some(json!({ "claim_id": 9001, "decision": "approve" })),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(balance(&state, a), 5, "approval credits the reward");

    let (_st, review) = send(&state, "GET", "/api/review", Some(&token), None).await;
    assert!(
        review["pending_claims"].as_array().unwrap().is_empty(),
        "claim cleared"
    );
}

#[tokio::test]
async fn reject_claim_with_reason_does_not_credit() {
    let (state, admin, token, _dir) = keep();
    let a = add_squire(&state, admin, "Arthur Jr", "a");
    submit_claim(&state, a, 9002);

    let (st, _) = send(
        &state,
        "POST",
        "/api/review/claim",
        Some(&token),
        Some(json!({ "claim_id": 9002, "decision": { "reject": { "reason": "not done" } } })),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(balance(&state, a), 0, "a rejected claim pays nothing");
}

// ─── direct redeem + adjust ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn direct_redeem_is_idempotent_on_command_id() {
    let (state, admin, token, _dir) = keep();
    let a = add_squire(&state, admin, "Arthur Jr", "a");
    // Fund 6 so a cost-3 redeem is affordable.
    send(
        &state,
        "POST",
        "/api/adjust",
        Some(&token),
        Some(json!({ "command_id": 1, "squire": a.0, "amount": 6, "reason": "seed" })),
    )
    .await;

    let body = json!({ "command_id": 50, "squire": a.0, "item_id": ITEM });
    let (st1, _) = send(
        &state,
        "POST",
        "/api/redeem",
        Some(&token),
        Some(body.clone()),
    )
    .await;
    let (st2, _) = send(&state, "POST", "/api/redeem", Some(&token), Some(body)).await;
    assert_eq!((st1, st2), (StatusCode::OK, StatusCode::OK));
    assert_eq!(
        balance(&state, a),
        3,
        "redeemed exactly once (6 - 3), not twice"
    );
}

#[tokio::test]
async fn adjust_requires_reason_and_is_idempotent() {
    let (state, admin, token, _dir) = keep();
    let a = add_squire(&state, admin, "Arthur Jr", "a");

    // Blank reason → 400.
    let (st, _) = send(
        &state,
        "POST",
        "/api/adjust",
        Some(&token),
        Some(json!({ "command_id": 2, "squire": a.0, "amount": 10, "reason": "  " })),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);

    // Same command_id twice → applied once.
    let body = json!({ "command_id": 3, "squire": a.0, "amount": 10, "reason": "bonus" });
    send(
        &state,
        "POST",
        "/api/adjust",
        Some(&token),
        Some(body.clone()),
    )
    .await;
    send(&state, "POST", "/api/adjust", Some(&token), Some(body)).await;
    assert_eq!(balance(&state, a), 10, "adjust applied exactly once");
}

#[tokio::test]
async fn redemption_approval_fails_when_drained_before_review() {
    let (state, admin, token, _dir) = keep();
    let a = add_squire(&state, admin, "Arthur Jr", "a");
    // Fund 5, request the cost-3 item while affordable.
    send(
        &state,
        "POST",
        "/api/adjust",
        Some(&token),
        Some(json!({ "command_id": 1, "squire": a.0, "amount": 5, "reason": "seed" })),
    )
    .await;
    state
        .commit(
            None,
            Command::RequestRedemption {
                request_id: RequestId(7001),
                squire: a,
                item_id: ItemId(ITEM),
            },
        )
        .expect("request");
    // Drain before review.
    send(
        &state,
        "POST",
        "/api/adjust",
        Some(&token),
        Some(json!({ "command_id": 2, "squire": a.0, "amount": -5, "reason": "drain" })),
    )
    .await;

    let (st, _) = send(
        &state,
        "POST",
        "/api/review/redemption",
        Some(&token),
        Some(json!({ "request_id": 7001, "decision": "approve" })),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT, "can no longer afford → 409");

    let (_st, review) = send(&state, "GET", "/api/review", Some(&token), None).await;
    assert_eq!(
        review["pending_requests"].as_array().unwrap().len(),
        1,
        "still pending after failed approval"
    );
}

/// Rejecting a redemption request over the HTTP surface spends nothing and resolves the request
/// (it leaves the pending queue). Mirrors `reject_claim_with_reason_does_not_credit` (SQUIRE-T-0077).
#[tokio::test]
async fn reject_redemption_with_reason_does_not_credit_and_resolves() {
    let (state, admin, token, _dir) = keep();
    let a = add_squire(&state, admin, "Arthur Jr", "a");
    send(
        &state,
        "POST",
        "/api/adjust",
        Some(&token),
        Some(json!({ "command_id": 1, "squire": a.0, "amount": 10, "reason": "seed" })),
    )
    .await;
    state
        .commit(
            None,
            Command::RequestRedemption {
                request_id: RequestId(7100),
                squire: a,
                item_id: ItemId(ITEM),
            },
        )
        .expect("request");

    let (st, _) = send(&state, "POST", "/api/review/redemption", Some(&token), Some(json!({ "request_id": 7100, "decision": { "reject": { "reason": "Maybe next week" } } }))).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(balance(&state, a), 10, "a rejected request spends nothing");

    let (_st, review) = send(&state, "GET", "/api/review", Some(&token), None).await;
    assert!(
        review["pending_requests"].as_array().unwrap().is_empty(),
        "rejected request leaves the queue"
    );
}

// ─── trust boundary ──────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn a_squire_cannot_use_the_review_surface() {
    let (state, admin, token, _dir) = keep();
    let a = add_squire(&state, admin, "Arthur Jr", "a");
    // Mint the squire's token via the members API.
    let (_st, added) = send(
        &state,
        "POST",
        "/api/members",
        Some(&token),
        Some(json!({ "role": "Squire", "display_name": "Spy", "initial_secret": "p" })),
    )
    .await;
    let squire_token = added["token"].as_str().unwrap().to_string();

    for (method, path, body) in [
        ("GET", "/api/review", None),
        (
            "POST",
            "/api/review/claim",
            Some(json!({ "claim_id": 1, "decision": "approve" })),
        ),
        (
            "POST",
            "/api/redeem",
            Some(json!({ "command_id": 1, "squire": a.0, "item_id": ITEM })),
        ),
        (
            "POST",
            "/api/adjust",
            Some(json!({ "command_id": 1, "squire": a.0, "amount": 1, "reason": "x" })),
        ),
    ] {
        let (st, _) = send(&state, method, path, Some(&squire_token), body).await;
        assert_eq!(st, StatusCode::FORBIDDEN, "squire on {path} must be 403");
    }
}
