//! End-to-end-ish tests for the Knight-role privileged surface (SQUIRE-T-0016), driven through
//! the axum `Router` via `tower::ServiceExt::oneshot` — no socket bound. A temp-dir SQLite tenant
//! is seeded with a Knight + two Squires, a daily quest assigned to both, and one item; the
//! `DevIdentity` is seeded with a Knight token and a Squire token (both bound to `house1`).
//!
//! Coverage: the `/household-review` triage read, the quick-action endpoints (review-claim,
//! adjust, redeem) including idempotent replays and the `AlreadyReviewed` double-action 409, and
//! the trust boundary (a Squire token on any `/admin/...` or `/household-review` is a 403; a
//! missing token is a 401).

use std::collections::BTreeSet;
use std::sync::Arc;

use api::identity::{DevIdentity, Principal};
use api::{router, AppState};

use axum::body::Body;
use axum::http::{Request, StatusCode};

use domain_core::contract::{
    Assignment, AuthToken, Availability, Cadence, Change, Completion, Date, HouseholdHandle,
    HouseholdReview, ItemId, Quest, QuestId, RedeemableItem, Role, Schedule, User, UserId,
};
use domain_core::contract::{Clock, Repository};
use store::tenant::{Backend, Provisioner};
use store::SystemClock;

use http_body_util::BodyExt;
use tower::ServiceExt; // for `oneshot`

const HANDLE: &str = "house1";
const KNIGHT_TOKEN: &str = "tok-knight";
const SQUIRE_TOKEN: &str = "tok-squire";

const KNIGHT_ID: u128 = 1;
const SQUIRE_A_ID: u128 = 2;
const SQUIRE_B_ID: u128 = 3;
const QUEST_ID: u128 = 100;
const ITEM_ID: u128 = 200;

/// Build an [`AppState`] over a fresh temp-dir SQLite tenant seeded with a Knight + two Squires, a
/// daily quest assigned to both, and one item. The dev identity is seeded with a Knight token and
/// a Squire (squire A) token, both bound to `house1`. Returns the state, the `TempDir` (kept alive
/// for the test), and `today`.
fn test_state() -> (Arc<AppState>, tempfile::TempDir, Date) {
    let dir = tempfile::tempdir().expect("tempdir");
    let provisioner = Provisioner::new(Backend::Sqlite { dir: dir.path().to_path_buf() });
    let mut store = provisioner.open(HANDLE, SystemClock).expect("open tenant store");

    let today = store.clock().today();

    let knight = User { id: UserId(KNIGHT_ID), role: Role::Knight, display_name: "Knight".into(), active: true };
    let squire_a = User { id: UserId(SQUIRE_A_ID), role: Role::Squire, display_name: "Arthur".into(), active: true };
    let squire_b = User { id: UserId(SQUIRE_B_ID), role: Role::Squire, display_name: "Bedivere".into(), active: true };

    let mut assignees = BTreeSet::new();
    assignees.insert(UserId(SQUIRE_A_ID));
    assignees.insert(UserId(SQUIRE_B_ID));
    let quest = Quest {
        id: QuestId(QUEST_ID),
        title: "Tidy room".into(),
        description: None,
        category: None,
        reward: 5,
        cadence: Cadence::Recurring(Schedule::Daily),
        assignment: Assignment::Squires(assignees),
        completion: Completion::EachAssignee,
        auto_approve: false,
        repeatable_within_day: false,
        active: true,
        icon: None,
    };
    let item = RedeemableItem {
        id: ItemId(ITEM_ID),
        name: "Ice cream".into(),
        description: None,
        cost: 3,
        gate: None,
        availability: Availability::Repeatable,
        active: true,
        icon: None,
    };

    store
        .apply(
            None,
            &[
                Change::PutUser(knight),
                Change::PutUser(squire_a),
                Change::PutUser(squire_b),
                Change::PutQuest(quest),
                Change::PutItem(item),
            ],
        )
        .expect("seed");

    let identity = DevIdentity::new();
    identity.seed(
        AuthToken(KNIGHT_TOKEN.into()),
        Principal { household: HouseholdHandle(HANDLE.into()), user: UserId(KNIGHT_ID), role: Role::Knight },
    );
    identity.seed(
        AuthToken(SQUIRE_TOKEN.into()),
        Principal { household: HouseholdHandle(HANDLE.into()), user: UserId(SQUIRE_A_ID), role: Role::Squire },
    );

    let state = AppState::new(store, Arc::new(identity));
    (state, dir, today)
}

fn req(method: &str, path: &str, token: Option<&str>, body: Option<String>) -> Request<Body> {
    let mut b = Request::builder().method(method).uri(path);
    if let Some(t) = token {
        b = b
            .header("authorization", format!("Bearer {t}"))
            .header("x-household", HANDLE);
    }
    let body = match body {
        Some(json) => {
            b = b.header("content-type", "application/json");
            Body::from(json)
        }
        None => Body::empty(),
    };
    b.body(body).expect("request")
}

async fn json_body<T: serde::de::DeserializeOwned>(resp: axum::response::Response) -> T {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).expect("deserialize body")
}

/// Squire A submits a completion claim via the Squire route; returns the `claim_id` used.
async fn squire_a_claims(state: Arc<AppState>, today: Date, claim_id: u128) {
    let body = serde_json::json!({ "claim_id": claim_id, "quest_id": QUEST_ID, "on": today.0 }).to_string();
    let resp = router(state)
        .oneshot(req("POST", "/claims", Some(SQUIRE_TOKEN), Some(body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn household_review_lists_pending_claim_and_squires() {
    let (state, _dir, today) = test_state();
    squire_a_claims(state.clone(), today, 9001).await;

    let resp = router(state)
        .oneshot(req("GET", "/household-review", Some(KNIGHT_TOKEN), None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let review: HouseholdReview = json_body(resp).await;

    // Both squires are listed (active), both at balance 0.
    assert_eq!(review.squires.len(), 2);
    assert!(review.squires.iter().all(|s| s.balance == 0));
    assert!(review.squires.iter().any(|s| s.squire == UserId(SQUIRE_A_ID)));
    assert!(review.squires.iter().any(|s| s.squire == UserId(SQUIRE_B_ID)));

    // The pending claim is shown, labelled with the right squire and quest title.
    assert_eq!(review.pending_claims.len(), 1);
    let pc = &review.pending_claims[0];
    assert_eq!(pc.squire, UserId(SQUIRE_A_ID));
    assert_eq!(pc.quest_title, "Tidy room");
    assert!(review.pending_requests.is_empty());
}

#[tokio::test]
async fn review_claim_approve_credits_balance_and_clears_pending() {
    let (state, _dir, today) = test_state();
    let claim_id = 9002u128;
    squire_a_claims(state.clone(), today, claim_id).await;

    // Knight approves the claim.
    let body = serde_json::json!({ "claim_id": claim_id, "decision": "approve" }).to_string();
    let resp = router(state.clone())
        .oneshot(req("POST", "/admin/review-claim", Some(KNIGHT_TOKEN), Some(body.clone())))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Balance credited (quest reward = 5); claim no longer pending.
    let review: HouseholdReview = json_body(
        router(state.clone())
            .oneshot(req("GET", "/household-review", Some(KNIGHT_TOKEN), None))
            .await
            .unwrap(),
    )
    .await;
    let a = review.squires.iter().find(|s| s.squire == UserId(SQUIRE_A_ID)).unwrap();
    assert_eq!(a.balance, 5);
    assert!(review.pending_claims.is_empty());

    // A SECOND approve of the same claim is a genuine double-action → 409 AlreadyReviewed.
    let resp2 = router(state)
        .oneshot(req("POST", "/admin/review-claim", Some(KNIGHT_TOKEN), Some(body)))
        .await
        .unwrap();
    assert_eq!(resp2.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn adjust_empty_reason_400_then_idempotent_credit() {
    let (state, _dir, _today) = test_state();

    // Empty reason → 400, before the engine.
    let bad = serde_json::json!({
        "command_id": 5000u128, "squire": SQUIRE_A_ID, "amount": 10, "reason": "   "
    })
    .to_string();
    let resp = router(state.clone())
        .oneshot(req("POST", "/admin/adjust", Some(KNIGHT_TOKEN), Some(bad)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Valid reason → ok, balance changes.
    let good = serde_json::json!({
        "command_id": 5001u128, "squire": SQUIRE_A_ID, "amount": 10, "reason": "bonus"
    })
    .to_string();
    let resp = router(state.clone())
        .oneshot(req("POST", "/admin/adjust", Some(KNIGHT_TOKEN), Some(good.clone())))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let balance_now = |review: &HouseholdReview| {
        review.squires.iter().find(|s| s.squire == UserId(SQUIRE_A_ID)).unwrap().balance
    };

    let review: HouseholdReview = json_body(
        router(state.clone())
            .oneshot(req("GET", "/household-review", Some(KNIGHT_TOKEN), None))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(balance_now(&review), 10);

    // Replay the SAME command_id → still ok, still one adjustment (idempotent, no double-credit).
    let resp = router(state.clone())
        .oneshot(req("POST", "/admin/adjust", Some(KNIGHT_TOKEN), Some(good)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let review: HouseholdReview = json_body(
        router(state)
            .oneshot(req("GET", "/household-review", Some(KNIGHT_TOKEN), None))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(balance_now(&review), 10);
}

#[tokio::test]
async fn redeem_is_idempotent_no_double_spend() {
    let (state, _dir, _today) = test_state();

    // Give squire A enough to afford the item (cost 3).
    let fund = serde_json::json!({
        "command_id": 6000u128, "squire": SQUIRE_A_ID, "amount": 10, "reason": "fund"
    })
    .to_string();
    let resp = router(state.clone())
        .oneshot(req("POST", "/admin/adjust", Some(KNIGHT_TOKEN), Some(fund)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Direct redeem.
    let redeem = serde_json::json!({
        "command_id": 6001u128, "squire": SQUIRE_A_ID, "item_id": ITEM_ID
    })
    .to_string();
    let resp = router(state.clone())
        .oneshot(req("POST", "/admin/redeem", Some(KNIGHT_TOKEN), Some(redeem.clone())))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let balance_now = |review: &HouseholdReview| {
        review.squires.iter().find(|s| s.squire == UserId(SQUIRE_A_ID)).unwrap().balance
    };
    let review: HouseholdReview = json_body(
        router(state.clone())
            .oneshot(req("GET", "/household-review", Some(KNIGHT_TOKEN), None))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(balance_now(&review), 7); // 10 - 3

    // Replay same command_id → ok, no double-spend.
    let resp = router(state.clone())
        .oneshot(req("POST", "/admin/redeem", Some(KNIGHT_TOKEN), Some(redeem)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let review: HouseholdReview = json_body(
        router(state)
            .oneshot(req("GET", "/household-review", Some(KNIGHT_TOKEN), None))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(balance_now(&review), 7); // unchanged
}

#[tokio::test]
async fn squire_token_on_admin_and_review_is_403() {
    let (state, _dir, today) = test_state();

    let cases: Vec<(&str, &str, Option<String>)> = vec![
        ("GET", "/household-review", None),
        ("POST", "/admin/review-claim", Some(serde_json::json!({ "claim_id": 1u128, "decision": "approve" }).to_string())),
        ("POST", "/admin/review-redemption", Some(serde_json::json!({ "request_id": 1u128, "decision": "approve" }).to_string())),
        ("POST", "/admin/redeem", Some(serde_json::json!({ "command_id": 1u128, "squire": SQUIRE_A_ID, "item_id": ITEM_ID }).to_string())),
        ("POST", "/admin/adjust", Some(serde_json::json!({ "command_id": 1u128, "squire": SQUIRE_A_ID, "amount": 1, "reason": "x" }).to_string())),
        ("POST", "/admin/mark-done", Some(serde_json::json!({ "claim_id": 1u128, "squire": SQUIRE_A_ID, "quest_id": QUEST_ID, "on": today.0 }).to_string())),
    ];

    for (method, path, body) in cases {
        let resp = router(state.clone())
            .oneshot(req(method, path, Some(SQUIRE_TOKEN), body))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN, "squire on {method} {path} should be 403");
    }
}

#[tokio::test]
async fn missing_token_on_admin_is_401() {
    let (state, _dir, _today) = test_state();
    let resp = router(state)
        .oneshot(req("GET", "/household-review", None, None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn mark_done_submits_and_approves() {
    let (state, _dir, today) = test_state();

    let body = serde_json::json!({
        "claim_id": 7000u128, "squire": SQUIRE_B_ID, "quest_id": QUEST_ID, "on": today.0
    })
    .to_string();
    let resp = router(state.clone())
        .oneshot(req("POST", "/admin/mark-done", Some(KNIGHT_TOKEN), Some(body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Squire B is credited the reward, and there is no pending claim.
    let review: HouseholdReview = json_body(
        router(state)
            .oneshot(req("GET", "/household-review", Some(KNIGHT_TOKEN), None))
            .await
            .unwrap(),
    )
    .await;
    let b = review.squires.iter().find(|s| s.squire == UserId(SQUIRE_B_ID)).unwrap();
    assert_eq!(b.balance, 5);
    assert!(review.pending_claims.is_empty());
}
