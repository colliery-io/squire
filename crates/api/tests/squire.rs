//! End-to-end-ish tests for the three Squire-role endpoints (SQUIRE-T-0015), driven through the
//! axum `Router` via `tower::ServiceExt::oneshot` — no socket bound. A real temp-dir SQLite
//! tenant is provisioned and seeded (a Knight + a Squire, a daily quest assigned to the Squire,
//! an item); the `DevIdentity` is seeded with a Squire and a Knight token.

use std::collections::BTreeSet;
use std::sync::Arc;

use api::identity::{DevIdentity, Principal};
use api::{router, AppState};

use axum::body::Body;
use axum::http::{Request, StatusCode};

use domain_core::contract::{
    Assignment, AuthToken, Cadence, Change, Completion, Date, HouseholdHandle, ItemId, Quest,
    QuestId, RedeemableItem, Availability, Role, Schedule, StateView, SubmitClaimResp,
    RequestRedemptionResp, ClaimState, RedemptionState, User, UserId,
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
const SQUIRE_ID: u128 = 2;
const QUEST_ID: u128 = 100;
const ITEM_ID: u128 = 200;

/// Build an [`AppState`] over a fresh isolated temp-dir SQLite tenant seeded with a household:
/// a Knight + a Squire, a daily quest assigned to the Squire, and one item. The dev identity is
/// seeded with a Squire token and a Knight token, both bound to `house1`. Returns the state, the
/// `TempDir` (kept alive for the test), and `today` (the clock's date, for building claims).
fn test_state() -> (Arc<AppState>, tempfile::TempDir, Date) {
    let dir = tempfile::tempdir().expect("tempdir");
    let provisioner = Provisioner::new(Backend::Sqlite { dir: dir.path().to_path_buf() });
    let mut store = provisioner
        .open(HANDLE, SystemClock)
        .expect("open tenant store");

    let today = store.clock().today();

    // Seed identity + definitions directly via Put* changes (by = None: a system seed).
    let knight = User { id: UserId(KNIGHT_ID), role: Role::Knight, display_name: "Knight".into(), active: true };
    let squire = User { id: UserId(SQUIRE_ID), role: Role::Squire, display_name: "Squire".into(), active: true };

    let mut assignees = BTreeSet::new();
    assignees.insert(UserId(SQUIRE_ID));
    let quest = Quest {
        id: QuestId(QUEST_ID),
        title: "Tidy room".into(),
        description: None,
        category: None,
        reward: 5,
        cadence: Cadence::Recurring(Schedule::Daily),
        assignment: Assignment::Squires(assignees),
        completion: Completion::EachAssignee,
        auto_approve: false, // stays Pending after a claim
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
                Change::PutUser(squire),
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
        Principal { household: HouseholdHandle(HANDLE.into()), user: UserId(SQUIRE_ID), role: Role::Squire },
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

#[tokio::test]
async fn get_state_returns_squires_view() {
    let (state, _dir, _today) = test_state();
    let resp = router(state)
        .oneshot(req("GET", "/state", Some(SQUIRE_TOKEN), None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let view: StateView = json_body(resp).await;
    assert_eq!(view.squire, UserId(SQUIRE_ID));
    assert_eq!(view.balance, 0);
    // The daily quest the Squire is assigned for shows up as Available today.
    assert_eq!(view.quests_today.len(), 1);
    let card = &view.quests_today[0];
    assert_eq!(card.quest_id, QuestId(QUEST_ID));
    assert!(matches!(card.status, domain_core::contract::QuestStatus::Available));
    // The reward card is present.
    assert_eq!(view.rewards.len(), 1);
    assert_eq!(view.rewards[0].item_id, ItemId(ITEM_ID));
    // No claims or requests yet.
    assert!(view.my_claims.is_empty());
    assert!(view.my_requests.is_empty());
}

#[tokio::test]
async fn submit_claim_is_pending_and_idempotent() {
    let (state, _dir, today) = test_state();
    let body = serde_json::json!({
        "claim_id": 9000u128,
        "quest_id": QUEST_ID,
        "on": today.0,
    })
    .to_string();

    let resp = router(state.clone())
        .oneshot(req("POST", "/claims", Some(SQUIRE_TOKEN), Some(body.clone())))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let r: SubmitClaimResp = json_body(resp).await;
    assert!(matches!(r.state, ClaimState::Pending));

    // Re-POST the same claim_id → still one claim, same Pending state (idempotent).
    let resp2 = router(state.clone())
        .oneshot(req("POST", "/claims", Some(SQUIRE_TOKEN), Some(body)))
        .await
        .unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);
    let r2: SubmitClaimResp = json_body(resp2).await;
    assert!(matches!(r2.state, ClaimState::Pending));

    // The state view shows exactly one claim.
    let view: StateView = json_body(
        router(state)
            .oneshot(req("GET", "/state", Some(SQUIRE_TOKEN), None))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(view.my_claims.len(), 1);
    assert!(matches!(view.my_claims[0].state, ClaimState::Pending));
}

#[tokio::test]
async fn redemption_request_is_pending_and_shows_in_state() {
    let (state, _dir, _today) = test_state();
    let body = serde_json::json!({ "request_id": 8000u128, "item_id": ITEM_ID }).to_string();

    let resp = router(state.clone())
        .oneshot(req("POST", "/redemption-requests", Some(SQUIRE_TOKEN), Some(body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let r: RequestRedemptionResp = json_body(resp).await;
    assert!(matches!(r.state, RedemptionState::Pending));

    let view: StateView = json_body(
        router(state)
            .oneshot(req("GET", "/state", Some(SQUIRE_TOKEN), None))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(view.my_requests.len(), 1);
    assert!(matches!(view.my_requests[0].state, RedemptionState::Pending));
    assert_eq!(view.my_requests[0].cost, 3);
}

#[tokio::test]
async fn state_without_token_is_401() {
    let (state, _dir, _today) = test_state();
    let resp = router(state)
        .oneshot(req("GET", "/state", None, None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn knight_on_squire_routes_is_403() {
    let (state, _dir, today) = test_state();

    // GET /state
    let resp = router(state.clone())
        .oneshot(req("GET", "/state", Some(KNIGHT_TOKEN), None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // POST /claims
    let claim_body = serde_json::json!({ "claim_id": 1u128, "quest_id": QUEST_ID, "on": today.0 }).to_string();
    let resp = router(state.clone())
        .oneshot(req("POST", "/claims", Some(KNIGHT_TOKEN), Some(claim_body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // POST /redemption-requests
    let red_body = serde_json::json!({ "request_id": 1u128, "item_id": ITEM_ID }).to_string();
    let resp = router(state)
        .oneshot(req("POST", "/redemption-requests", Some(KNIGHT_TOKEN), Some(red_body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}
