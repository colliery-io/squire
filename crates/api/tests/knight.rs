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

use identity::{DevIdentity, Principal};
use api::{router, AppState};

use axum::body::Body;
use axum::http::{Request, StatusCode};

use domain_core::contract::{
    Assignment, AuthToken, Availability, Cadence, Change, Completion, Date, HouseholdHandle,
    HouseholdReview, ItemId, Quest, QuestId, RedeemableItem, Role, Schedule, StateView, User, UserId,
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
        cash: 0,
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

    // Share one store between the identity port and the request handlers.
    let store = Arc::new(std::sync::Mutex::new(store));
    let identity = DevIdentity::new(store.clone());
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
    let body = serde_json::json!({ "claim_id": claim_id, "decision": { "verdict": "approve" } }).to_string();
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
        ("POST", "/admin/review-claim", Some(serde_json::json!({ "claim_id": 1u128, "decision": { "verdict": "approve" } }).to_string())),
        ("POST", "/admin/review-redemption", Some(serde_json::json!({ "request_id": 1u128, "decision": { "verdict": "approve" } }).to_string())),
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

// ─── "assume Squire" read (SQUIRE-T-0053) ────────────────────────────────────────────────────────

#[tokio::test]
async fn knight_can_read_any_squires_state() {
    let (state, _dir, _today) = test_state();
    let resp = router(state)
        .oneshot(req("GET", &format!("/admin/squire/{SQUIRE_A_ID}/state"), Some(KNIGHT_TOKEN), None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let view: StateView = json_body(resp).await;
    assert_eq!(view.squire, UserId(SQUIRE_A_ID));
    // The seeded daily quest assigned to both squires is on this squire's "today" list.
    assert!(!view.quests_today.is_empty(), "the daily quest shows on the assumed squire's home");
}

#[tokio::test]
async fn assume_unknown_squire_is_404() {
    let (state, _dir, _today) = test_state();
    let resp = router(state)
        .oneshot(req("GET", "/admin/squire/9999/state", Some(KNIGHT_TOKEN), None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn assume_a_non_squire_id_is_404() {
    let (state, _dir, _today) = test_state();
    // The Knight's own id is not a Squire.
    let resp = router(state)
        .oneshot(req("GET", &format!("/admin/squire/{KNIGHT_ID}/state"), Some(KNIGHT_TOKEN), None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_squire_token_cannot_assume_a_squire() {
    let (state, _dir, _today) = test_state();
    let resp = router(state)
        .oneshot(req("GET", &format!("/admin/squire/{SQUIRE_B_ID}/state"), Some(SQUIRE_TOKEN), None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// ─── Reward (item) authoring from the phone (SQUIRE-T-0074) ──────────────────────────────────────

/// A reward created over `/admin/items` with **explicit JSON nulls** for every optional field (the
/// shape the generated SDK emits) is accepted — guards against the `#[serde(default)]`-on-`Vec` 422
/// that bit quest authoring (SQUIRE-T-0065). It then lists with a server-computed availability label.
#[tokio::test]
async fn create_item_with_explicit_nulls_lists_with_label() {
    let (state, _dir, _today) = test_state();
    let body = serde_json::json!({
        "id": null, "name": "Sticker pack", "description": null,
        "cost": 5, "availability": "Repeatable", "gate": null, "icon": null,
    })
    .to_string();
    let resp = router(state.clone())
        .oneshot(req("POST", "/admin/items", Some(KNIGHT_TOKEN), Some(body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "explicit nulls must not 422");

    let resp = router(state)
        .oneshot(req("GET", "/admin/items", Some(KNIGHT_TOKEN), None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let rows: serde_json::Value = json_body(resp).await;
    let row = rows.as_array().unwrap().iter().find(|r| r["name"] == "Sticker pack").unwrap();
    assert_eq!(row["cost"], 5);
    assert_eq!(row["summary"], "Repeatable");
    assert_eq!(row["active"], true);
}

/// A reward gated on a non-existent achievement is a 404 (engine `validate_item`).
#[tokio::test]
async fn create_item_gated_on_missing_achievement_is_404() {
    let (state, _dir, _today) = test_state();
    let body = serde_json::json!({
        "id": null, "name": "Locked treat", "description": null,
        "cost": 1, "availability": "Once", "gate": 999999, "icon": null,
    })
    .to_string();
    let resp = router(state)
        .oneshot(req("POST", "/admin/items", Some(KNIGHT_TOKEN), Some(body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// Archive the seeded reward (204); archiving a missing reward is a 404.
#[tokio::test]
async fn archive_item_then_missing_is_404() {
    let (state, _dir, _today) = test_state();
    let resp = router(state.clone())
        .oneshot(req("POST", &format!("/admin/items/{ITEM_ID}/archive"), Some(KNIGHT_TOKEN), None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = router(state)
        .oneshot(req("POST", "/admin/items/424242/archive", Some(KNIGHT_TOKEN), None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// A Squire token cannot author rewards (trust boundary — 403).
#[tokio::test]
async fn a_squire_token_cannot_author_items() {
    let (state, _dir, _today) = test_state();
    let body = serde_json::json!({
        "id": null, "name": "Nope", "description": null,
        "cost": 1, "availability": "Repeatable", "gate": null, "icon": null,
    })
    .to_string();
    let resp = router(state)
        .oneshot(req("POST", "/admin/items", Some(SQUIRE_TOKEN), Some(body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// ─── Member administration from the phone (SQUIRE-T-0075) ────────────────────────────────────────

/// `GET /admin/members` lists the seeded Knight + two Squires with role + active flags.
#[tokio::test]
async fn list_members_returns_seeded_household() {
    let (state, _dir, _today) = test_state();
    let resp = router(state)
        .oneshot(req("GET", "/admin/members", Some(KNIGHT_TOKEN), None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let rows: serde_json::Value = json_body(resp).await;
    let arr = rows.as_array().unwrap();
    assert_eq!(arr.len(), 3, "Knight + two Squires");
    let knight = arr.iter().find(|m| m["user"] == KNIGHT_ID as i64).unwrap();
    assert_eq!(knight["role"], "Knight");
    assert_eq!(knight["active"], true);
    assert!(arr.iter().filter(|m| m["role"] == "Squire").count() == 2);
}

/// Deactivating a Squire flips its active flag in the list; reactivating restores it.
#[tokio::test]
async fn set_member_active_toggles_then_lists() {
    let (state, _dir, _today) = test_state();
    let off = serde_json::json!({ "active": false }).to_string();
    let resp = router(state.clone())
        .oneshot(req("POST", &format!("/admin/members/{SQUIRE_A_ID}/active"), Some(KNIGHT_TOKEN), Some(off)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = router(state.clone())
        .oneshot(req("GET", "/admin/members", Some(KNIGHT_TOKEN), None))
        .await
        .unwrap();
    let rows: serde_json::Value = json_body(resp).await;
    let sq = rows.as_array().unwrap().iter().find(|m| m["user"] == SQUIRE_A_ID as i64).unwrap().clone();
    assert_eq!(sq["active"], false, "deactivated");

    let on = serde_json::json!({ "active": true }).to_string();
    let resp = router(state)
        .oneshot(req("POST", &format!("/admin/members/{SQUIRE_A_ID}/active"), Some(KNIGHT_TOKEN), Some(on)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

/// Setting active on a missing member is a 404.
#[tokio::test]
async fn set_member_active_missing_is_404() {
    let (state, _dir, _today) = test_state();
    let body = serde_json::json!({ "active": false }).to_string();
    let resp = router(state)
        .oneshot(req("POST", "/admin/members/999999/active", Some(KNIGHT_TOKEN), Some(body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// A Knight cannot deactivate their own account (no self-lockout) → 400.
#[tokio::test]
async fn a_knight_cannot_deactivate_self() {
    let (state, _dir, _today) = test_state();
    let body = serde_json::json!({ "active": false }).to_string();
    let resp = router(state)
        .oneshot(req("POST", &format!("/admin/members/{KNIGHT_ID}/active"), Some(KNIGHT_TOKEN), Some(body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

/// A Squire token cannot read or change members (trust boundary — 403 on each).
#[tokio::test]
async fn a_squire_token_cannot_administer_members() {
    let (state, _dir, _today) = test_state();
    let resp = router(state.clone())
        .oneshot(req("GET", "/admin/members", Some(SQUIRE_TOKEN), None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    let body = serde_json::json!({ "active": false }).to_string();
    let resp = router(state)
        .oneshot(req("POST", &format!("/admin/members/{SQUIRE_B_ID}/active"), Some(SQUIRE_TOKEN), Some(body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// ─── Reward request/redeem loop hardening (SQUIRE-T-0077) ─────────────────────────────────────────

/// Fund squire A by `amount` (Knight adjust). Panics unless 200.
async fn fund(state: &Arc<AppState>, command_id: u128, amount: i64) {
    let body = serde_json::json!({ "command_id": command_id, "squire": SQUIRE_A_ID, "amount": amount, "reason": "seed" }).to_string();
    let resp = router(state.clone()).oneshot(req("POST", "/admin/adjust", Some(KNIGHT_TOKEN), Some(body))).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

/// Squire A's `GET /state` as a JSON value (the child's view of its own requests/balance).
async fn squire_state(state: &Arc<AppState>) -> serde_json::Value {
    json_body(router(state.clone()).oneshot(req("GET", "/state", Some(SQUIRE_TOKEN), None)).await.unwrap()).await
}

/// Full child→Knight cycle: request → approve debits the cost; the child's state shows it Approved.
#[tokio::test]
async fn redemption_request_approve_debits_and_shows_approved() {
    let (state, _dir, _today) = test_state();
    fund(&state, 6000, 10).await;
    // Child requests the seeded item (cost 3).
    let request = serde_json::json!({ "request_id": 6100u128, "item_id": ITEM_ID }).to_string();
    let resp = router(state.clone()).oneshot(req("POST", "/redemption-requests", Some(SQUIRE_TOKEN), Some(request))).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    // Knight approves.
    let approve = serde_json::json!({ "request_id": 6100u128, "decision": { "verdict": "approve" } }).to_string();
    let resp = router(state.clone()).oneshot(req("POST", "/admin/review-redemption", Some(KNIGHT_TOKEN), Some(approve))).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let view = squire_state(&state).await;
    assert_eq!(view["balance"], 7, "approval debits the item cost");
    let r = &view["my_requests"][0];
    assert_eq!(r["state"]["state"], "Approved");
}

/// A Knight reject with a reason reaches the child: `GET /state` `my_requests` shows it Rejected
/// carrying the reason, and nothing is debited.
#[tokio::test]
async fn redemption_reject_with_reason_surfaces_to_child() {
    let (state, _dir, _today) = test_state();
    fund(&state, 6001, 10).await;
    let request = serde_json::json!({ "request_id": 6200u128, "item_id": ITEM_ID }).to_string();
    router(state.clone()).oneshot(req("POST", "/redemption-requests", Some(SQUIRE_TOKEN), Some(request))).await.unwrap();
    let reject = serde_json::json!({ "request_id": 6200u128, "decision": { "verdict": "reject", "reason": "Maybe next week" } }).to_string();
    let resp = router(state.clone()).oneshot(req("POST", "/admin/review-redemption", Some(KNIGHT_TOKEN), Some(reject))).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let view = squire_state(&state).await;
    assert_eq!(view["balance"], 10, "a rejected request debits nothing");
    let r = &view["my_requests"][0];
    assert_eq!(r["state"]["state"], "Rejected");
    assert_eq!(r["state"]["reason"], "Maybe next week");
}

/// A direct redeem the Squire can't afford is a 409 (blocked at commit).
#[tokio::test]
async fn direct_redeem_insufficient_funds_is_409() {
    let (state, _dir, _today) = test_state();
    // Squire A has balance 0; the seeded item costs 3.
    let body = serde_json::json!({ "command_id": 6300u128, "squire": SQUIRE_A_ID, "item_id": ITEM_ID }).to_string();
    let resp = router(state).oneshot(req("POST", "/admin/redeem", Some(KNIGHT_TOKEN), Some(body))).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

/// A `Once` item redeemed a second time is out of stock → 409 (authored via `/admin/items`).
#[tokio::test]
async fn direct_redeem_once_item_out_of_stock_is_409() {
    let (state, _dir, _today) = test_state();
    fund(&state, 6002, 10).await;
    // Author a Once item (cost 1) from the phone surface.
    let item = serde_json::json!({ "id": 700u64, "name": "Sticker", "description": null, "cost": 1, "availability": "Once", "gate": null, "icon": null }).to_string();
    let resp = router(state.clone()).oneshot(req("POST", "/admin/items", Some(KNIGHT_TOKEN), Some(item))).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    // First redeem succeeds; the second (new command_id) is out of stock.
    let r1 = serde_json::json!({ "command_id": 6310u128, "squire": SQUIRE_A_ID, "item_id": 700 }).to_string();
    let resp = router(state.clone()).oneshot(req("POST", "/admin/redeem", Some(KNIGHT_TOKEN), Some(r1))).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let r2 = serde_json::json!({ "command_id": 6311u128, "squire": SQUIRE_A_ID, "item_id": 700 }).to_string();
    let resp = router(state).oneshot(req("POST", "/admin/redeem", Some(KNIGHT_TOKEN), Some(r2))).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT, "a redeemed Once item is out of stock");
}

/// A direct redeem of an achievement-gated item the Squire hasn't unlocked is a 409.
#[tokio::test]
async fn direct_redeem_gated_item_is_409() {
    let (state, _dir, _today) = test_state();
    fund(&state, 6003, 10).await;
    // Author an achievement (5 total completions) and an item gated on it.
    let ach = serde_json::json!({ "id": 800u64, "name": "Busy bee", "criterion": "TotalCompletions", "scope": "Any", "scope_quest": null, "scope_category": null, "length": null, "basis": null, "count": 5, "total": null, "bonus": 0 }).to_string();
    let resp = router(state.clone()).oneshot(req("POST", "/admin/achievements", Some(KNIGHT_TOKEN), Some(ach))).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let item = serde_json::json!({ "id": 801u64, "name": "Gated treat", "description": null, "cost": 1, "availability": "Repeatable", "gate": 800, "icon": null }).to_string();
    let resp = router(state.clone()).oneshot(req("POST", "/admin/items", Some(KNIGHT_TOKEN), Some(item))).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    // Squire A has 0 completions → the gate is locked → 409 (not insufficient: balance 10 ≥ cost 1).
    let body = serde_json::json!({ "command_id": 6320u128, "squire": SQUIRE_A_ID, "item_id": 801 }).to_string();
    let resp = router(state).oneshot(req("POST", "/admin/redeem", Some(KNIGHT_TOKEN), Some(body))).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT, "a locked gate blocks the redeem");
}
