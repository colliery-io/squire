//! **Consolidated end-to-end HTTP integration + trust-boundary + idempotency suite** for the
//! Local API component (SQUIRE-T-0018), driven entirely through the axum `Router` via
//! `tower::ServiceExt::oneshot` — no socket bound, no domain-internal shortcuts.
//!
//! Everything crosses the wire: callers `POST /register` / `POST /login` to obtain real
//! `(household, token)` pairs, then present `Authorization: Bearer <token>` + `X-Household:
//! <handle>` on every authenticated call with JSON bodies. The only thing seeded directly into
//! the shared `Store` is quest/item *authoring* (a `PutQuest` / `PutItem`), because authoring is
//! deliberately NOT on the wire (there is no define/archive route — proven below).
//!
//! This is the crux trust-boundary proof for the whole Local API:
//! * **AC-1** — the full claim→review→credit flow over HTTP (register → add Squire → login →
//!   claim → household-review → approve → state shows the credit).
//! * **Trust boundary** — a Squire token is 403 on every privileged route; a Squire can only ever
//!   read its OWN `/state` (identity is token-derived, never a parameter); there is no authoring
//!   route (404); missing / garbage / wrong-tenant credentials are rejected.
//! * **AC-3** — idempotency over HTTP: replaying a `claim_id` / `request_id` / `command_id`
//!   applies exactly once.
//! * **AC-6** — a redemption that was affordable at request time but is drained before approval
//!   fails the approval with the insufficient-points 409.

use std::sync::{Arc, Mutex};

use identity::DevIdentity;
use api::{router, AppState};

use axum::body::Body;
use axum::http::{Request, StatusCode};

use domain_core::contract::{
    Assignment, Availability, Cadence, Change, Completion, Date, ItemId, Quest, QuestId,
    RedeemableItem, RegisterHouseholdResp, Schedule,
};
use domain_core::contract::{Clock, Repository};
use store::tenant::{Backend, Provisioner};
use store::SystemClock;

use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt; // for `oneshot`

// Authored (NOT on the wire) — seeded into the shared store directly.
const DAILY_QUEST_ID: u128 = 100; // auto_approve = false → the review flow
const ITEM_ID: u128 = 200; // an affordable reward (cost 3)

// ─── fixture: a real app over a fresh isolated SQLite tenant ─────────────────────────────────

/// A fresh `AppState` over an isolated temp-dir SQLite tenant with a `DevIdentity` sharing that
/// store. No users are seeded — registration over the wire bootstraps the first Knight; squires
/// are added via `POST /members`. Returns the state, the live store handle (so the test can author
/// quests/items directly — authoring is not a wire endpoint), the `TempDir`, and `today`.
fn app() -> (
    Arc<AppState>,
    Arc<Mutex<store::Store<SystemClock>>>,
    tempfile::TempDir,
    Date,
) {
    let dir = tempfile::tempdir().expect("tempdir");
    let provisioner = Provisioner::new(Backend::Sqlite { dir: dir.path().to_path_buf() });
    let store = provisioner.open("seed", SystemClock).expect("open tenant store");
    let today = store.clock().today();

    let store = Arc::new(Mutex::new(store));
    let identity = DevIdentity::new(store.clone());
    let state = AppState::new(store.clone(), Arc::new(identity));
    (state, store, dir, today)
}

/// Author a daily quest assigned to ALL squires (auto_approve = false → stays Pending until a
/// Knight reviews) and one affordable item, written straight into the shared store. Authoring is
/// intentionally off the wire, so this is a direct `apply`, not an HTTP call.
fn seed_quest_and_item(store: &Arc<Mutex<store::Store<SystemClock>>>) {
    let quest = Quest {
        id: QuestId(DAILY_QUEST_ID),
        title: "Tidy room".into(),
        description: None,
        category: None,
        reward: 5,
        cadence: Cadence::Recurring(Schedule::Daily),
        assignment: Assignment::AllSquires, // every Squire in the household, by role
        completion: Completion::EachAssignee,
        auto_approve: false, // → the Knight review flow (AC-1)
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
        .lock()
        .expect("store mutex poisoned")
        .apply(None, &[Change::PutQuest(quest), Change::PutItem(item)])
        .expect("seed quest + item");
}

// ─── tiny wire helpers (the task's requested `post_json` / `get`) ────────────────────────────

/// Build a request, optionally authenticated with `Authorization: Bearer <token>` + `X-Household`.
fn build(
    method: &str,
    path: &str,
    auth: Option<(&str, &str)>,
    body: Option<String>,
) -> Request<Body> {
    let mut b = Request::builder().method(method).uri(path);
    if let Some((token, handle)) = auth {
        b = b
            .header("authorization", format!("Bearer {token}"))
            .header("x-household", handle);
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

async fn status_and_json(resp: axum::response::Response) -> (StatusCode, Value) {
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    // Some routes (404 / 401 / 403) carry no JSON body — fall back to Null.
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, value)
}

/// `POST <path>` as `(token, handle)` with a JSON body → `(status, body)`.
async fn post_json(
    state: &Arc<AppState>,
    path: &str,
    token: &str,
    handle: &str,
    body: Value,
) -> (StatusCode, Value) {
    let resp = router(state.clone())
        .oneshot(build("POST", path, Some((token, handle)), Some(body.to_string())))
        .await
        .unwrap();
    status_and_json(resp).await
}

/// `GET <path>` as `(token, handle)` → `(status, body)`.
async fn get(state: &Arc<AppState>, path: &str, token: &str, handle: &str) -> (StatusCode, Value) {
    let resp = router(state.clone())
        .oneshot(build("GET", path, Some((token, handle)), None))
        .await
        .unwrap();
    status_and_json(resp).await
}

// ─── over-the-wire register / login / add-member (how every token is obtained) ───────────────

/// `POST /register` (unauthenticated) → the household handle + the first Knight's token.
async fn register(state: &Arc<AppState>, name: &str, admin: &str, secret: &str) -> RegisterHouseholdResp {
    let body = json!({ "household_name": name, "admin_name": admin, "admin_secret": secret });
    let resp = router(state.clone())
        .oneshot(build("POST", "/register", None, Some(body.to_string())))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "register should be 200");
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).expect("RegisterHouseholdResp")
}

/// Knight adds a Squire (`POST /members`) and then that Squire logs in (`POST /login`), returning
/// `(squire_user_id, squire_token)`. Every step is over the wire.
async fn add_squire_and_login(
    state: &Arc<AppState>,
    knight_token: &str,
    handle: &str,
    name: &str,
    secret: &str,
) -> (u128, String) {
    let (st, add) = post_json(
        state,
        "/members",
        knight_token,
        handle,
        json!({ "role": "Squire", "display_name": name, "initial_secret": secret }),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "add member should be 200");
    let user = add["user"].as_u64().expect("user id") as u128;

    // /login is unauthenticated — exchange (household, user, secret) for a token.
    let resp = router(state.clone())
        .oneshot(build(
            "POST",
            "/login",
            None,
            Some(json!({ "household": handle, "user": user, "secret": secret }).to_string()),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "login should be 200");
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let login: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(login["role"], "Squire");
    let token = login["token"].as_str().expect("token").to_string();
    (user, token)
}

/// Give a squire a positive starting balance via a Knight `POST /admin/adjust`.
async fn fund(state: &Arc<AppState>, knight: &str, handle: &str, squire: u128, amount: i64, cmd: u128) {
    let (st, _) = post_json(
        state,
        "/admin/adjust",
        knight,
        handle,
        json!({ "command_id": cmd, "squire": squire, "amount": amount, "reason": "fund" }),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "fund adjust should be 200");
}

// ═════════════════════════════════════════════════════════════════════════════════════════════
// AC-1 — the full claim → review → credit flow, entirely over HTTP.
// ═════════════════════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn ac1_full_flow_over_http() {
    let (state, store, _dir, today) = app();
    seed_quest_and_item(&store);

    // 1. Register the household + first Knight over the wire.
    let reg = register(&state, "The Round Table", "Arthur", "excalibur").await;
    let handle = reg.household.0.clone();
    let knight = reg.token.0.clone();

    // 2. Knight adds a Squire, who then logs in (both over HTTP).
    let (squire_id, squire) = add_squire_and_login(&state, &knight, &handle, "Lancelot", "lake").await;

    // 3. Squire submits a completion claim → Pending (auto_approve = false).
    let claim_id = 9001u128;
    let (st, claim) = post_json(
        &state,
        "/claims",
        &squire,
        &handle,
        json!({ "claim_id": claim_id, "quest_id": DAILY_QUEST_ID, "on": today.0 }),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(claim["state"], "Pending", "fresh claim is Pending");

    // 4. Knight's household-review shows the claim in pending_claims, labelled with that squire.
    let (st, review) = get(&state, "/household-review", &knight, &handle).await;
    assert_eq!(st, StatusCode::OK);
    let pending = review["pending_claims"].as_array().unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0]["squire"], squire_id as u64);
    assert_eq!(pending[0]["quest_title"], "Tidy room");
    // The squire is also summarised at balance 0 before approval.
    let summary = review["squires"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["squire"] == squire_id as u64)
        .expect("squire summary");
    assert_eq!(summary["balance"], 0);

    // 5. Knight approves the claim.
    let (st, _) = post_json(
        &state,
        "/admin/review-claim",
        &knight,
        &handle,
        json!({ "claim_id": claim_id, "decision": "approve" }),
    )
    .await;
    assert_eq!(st, StatusCode::OK);

    // 6. The Squire's own /state shows the credited balance (reward = 5) and the Approved claim.
    let (st, view) = get(&state, "/state", &squire, &handle).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(view["squire"], squire_id as u64);
    assert_eq!(view["balance"], 5);
    let claims = view["my_claims"].as_array().unwrap();
    assert_eq!(claims.len(), 1);
    // ClaimState::Approved serialises as { "Approved": { "points": 5 } }.
    assert_eq!(claims[0]["state"]["Approved"]["points"], 5);
}

// ═════════════════════════════════════════════════════════════════════════════════════════════
// Trust boundary (the crux).
// ═════════════════════════════════════════════════════════════════════════════════════════════

/// A Squire token is 403 on every privileged route (the `RequireKnight` extractor rejects before
/// any handler body runs), and on `GET /household-review`.
#[tokio::test]
async fn squire_token_is_403_on_every_privileged_route() {
    let (state, store, _dir, today) = app();
    seed_quest_and_item(&store);
    let reg = register(&state, "House", "Admin", "secret").await;
    let handle = reg.household.0.clone();
    let knight = reg.token.0.clone();
    let (squire_id, squire) = add_squire_and_login(&state, &knight, &handle, "Sib", "s").await;

    // Each privileged POST with a *Squire* token → 403.
    let posts: Vec<(&str, Value)> = vec![
        ("/admin/review-claim", json!({ "claim_id": 1u128, "decision": "approve" })),
        ("/admin/review-redemption", json!({ "request_id": 1u128, "decision": "approve" })),
        ("/admin/redeem", json!({ "command_id": 1u128, "squire": squire_id, "item_id": ITEM_ID })),
        ("/admin/adjust", json!({ "command_id": 1u128, "squire": squire_id, "amount": 1, "reason": "x" })),
        ("/admin/mark-done", json!({ "claim_id": 1u128, "squire": squire_id, "quest_id": DAILY_QUEST_ID, "on": today.0 })),
    ];
    for (path, body) in posts {
        let (st, _) = post_json(&state, path, &squire, &handle, body).await;
        assert_eq!(st, StatusCode::FORBIDDEN, "squire on POST {path} should be 403");
    }

    // The cross-Squire triage read is Knight-only too.
    let (st, _) = get(&state, "/household-review", &squire, &handle).await;
    assert_eq!(st, StatusCode::FORBIDDEN, "squire on GET /household-review should be 403");
}

/// A Squire cannot read another Squire's data: identity is taken from the token, never a query
/// parameter, so each `GET /state` returns ONLY the caller's own squire/claims. Squire A's state
/// never contains squire B's claim.
#[tokio::test]
async fn squire_state_is_scoped_to_the_token_only() {
    let (state, store, _dir, today) = app();
    seed_quest_and_item(&store);
    let reg = register(&state, "House", "Admin", "secret").await;
    let handle = reg.household.0.clone();
    let knight = reg.token.0.clone();

    let (a_id, a_tok) = add_squire_and_login(&state, &knight, &handle, "Arthur", "a").await;
    let (b_id, b_tok) = add_squire_and_login(&state, &knight, &handle, "Bedivere", "b").await;

    // Squire B submits a claim. Squire A does not.
    let b_claim = 4242u128;
    let (st, _) = post_json(
        &state,
        "/claims",
        &b_tok,
        &handle,
        json!({ "claim_id": b_claim, "quest_id": DAILY_QUEST_ID, "on": today.0 }),
    )
    .await;
    assert_eq!(st, StatusCode::OK);

    // Squire A's /state is scoped to A: it is A's squire id and carries NONE of B's claims.
    let (st, a_view) = get(&state, "/state", &a_tok, &handle).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(a_view["squire"], a_id as u64);
    assert!(
        a_view["my_claims"].as_array().unwrap().is_empty(),
        "A's state must not contain B's claim"
    );

    // Squire B's /state is scoped to B and shows B's single claim.
    let (st, b_view) = get(&state, "/state", &b_tok, &handle).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(b_view["squire"], b_id as u64);
    let b_claims = b_view["my_claims"].as_array().unwrap();
    assert_eq!(b_claims.len(), 1);
    assert_eq!(b_claims[0]["claim_id"], b_claim as u64);
}

/// There is no authoring on the wire: no define-quest / archive route exists, so any such path is
/// a 404 (the router has no matching route — authoring stays a server-internal store write).
#[tokio::test]
async fn no_authoring_route_is_404() {
    let (state, store, _dir, _today) = app();
    seed_quest_and_item(&store);
    let reg = register(&state, "House", "Admin", "secret").await;
    let handle = reg.household.0.clone();
    let knight = reg.token.0.clone();

    // Even with a valid Knight token, there is simply no such route → 404 (not 401/403).
    for path in ["/admin/define-quest", "/admin/archive-quest", "/quests", "/admin/define-item"] {
        let (st, _) = post_json(&state, path, &knight, &handle, json!({ "anything": true })).await;
        assert_eq!(st, StatusCode::NOT_FOUND, "{path} must not exist on the wire");
    }
}

/// Missing, garbage, and wrong-tenant credentials are all rejected (never served).
#[tokio::test]
async fn missing_garbage_and_wrong_tenant_credentials_are_rejected() {
    let (state, store, _dir, _today) = app();
    seed_quest_and_item(&store);
    let reg = register(&state, "House", "Admin", "secret").await;
    let handle = reg.household.0.clone();
    let knight = reg.token.0.clone();
    let (_sq_id, squire) = add_squire_and_login(&state, &knight, &handle, "Sib", "s").await;

    // No Authorization header at all → 401.
    let resp = router(state.clone())
        .oneshot(build("GET", "/state", None, None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "no token → 401");

    // A garbage bearer token → 401.
    let (st, _) = get(&state, "/state", "not-a-real-token", &handle).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED, "garbage token → 401");

    // A valid Squire token but an unknown / wrong household handle → rejected (401 WrongTenant).
    let (st, _) = get(&state, "/state", &squire, "some-other-household").await;
    assert!(
        st == StatusCode::UNAUTHORIZED || st == StatusCode::FORBIDDEN,
        "valid token + wrong household must be rejected, got {st}"
    );
}

// ═════════════════════════════════════════════════════════════════════════════════════════════
// AC-3 — idempotency over HTTP (claim / request / adjust each apply exactly once).
// ═════════════════════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn claim_is_idempotent_over_http() {
    let (state, store, _dir, today) = app();
    seed_quest_and_item(&store);
    let reg = register(&state, "House", "Admin", "secret").await;
    let handle = reg.household.0.clone();
    let knight = reg.token.0.clone();
    let (_id, squire) = add_squire_and_login(&state, &knight, &handle, "Sib", "s").await;

    let body = json!({ "claim_id": 1234u128, "quest_id": DAILY_QUEST_ID, "on": today.0 });

    // Submit the same claim_id twice → 200 both times, same Pending state.
    let (st1, r1) = post_json(&state, "/claims", &squire, &handle, body.clone()).await;
    let (st2, r2) = post_json(&state, "/claims", &squire, &handle, body).await;
    assert_eq!(st1, StatusCode::OK);
    assert_eq!(st2, StatusCode::OK);
    assert_eq!(r1["state"], "Pending");
    assert_eq!(r2["state"], r1["state"], "replay returns the same state");

    // Exactly ONE claim exists (verified via the Squire's own /state).
    let (_st, view) = get(&state, "/state", &squire, &handle).await;
    assert_eq!(view["my_claims"].as_array().unwrap().len(), 1, "exactly one claim");
}

#[tokio::test]
async fn redemption_request_is_idempotent_over_http() {
    let (state, store, _dir, _today) = app();
    seed_quest_and_item(&store);
    let reg = register(&state, "House", "Admin", "secret").await;
    let handle = reg.household.0.clone();
    let knight = reg.token.0.clone();
    let (_id, squire) = add_squire_and_login(&state, &knight, &handle, "Sib", "s").await;

    let body = json!({ "request_id": 5678u128, "item_id": ITEM_ID });

    let (st1, r1) = post_json(&state, "/redemption-requests", &squire, &handle, body.clone()).await;
    let (st2, r2) = post_json(&state, "/redemption-requests", &squire, &handle, body).await;
    assert_eq!(st1, StatusCode::OK);
    assert_eq!(st2, StatusCode::OK);
    assert_eq!(r1["state"], "Pending");
    assert_eq!(r2["state"], r1["state"]);

    // Exactly ONE request exists (via the Squire's /state).
    let (_st, view) = get(&state, "/state", &squire, &handle).await;
    assert_eq!(view["my_requests"].as_array().unwrap().len(), 1, "exactly one request");
}

#[tokio::test]
async fn adjust_is_idempotent_over_http_no_double_credit() {
    let (state, store, _dir, _today) = app();
    seed_quest_and_item(&store);
    let reg = register(&state, "House", "Admin", "secret").await;
    let handle = reg.household.0.clone();
    let knight = reg.token.0.clone();
    let (squire_id, squire) = add_squire_and_login(&state, &knight, &handle, "Sib", "s").await;

    let body = json!({ "command_id": 7001u128, "squire": squire_id, "amount": 10, "reason": "bonus" });

    // Same command_id twice → 200 both times.
    let (st1, _) = post_json(&state, "/admin/adjust", &knight, &handle, body.clone()).await;
    let (st2, _) = post_json(&state, "/admin/adjust", &knight, &handle, body).await;
    assert_eq!(st1, StatusCode::OK);
    assert_eq!(st2, StatusCode::OK);

    // The squire's balance reflects exactly ONE adjustment (10, not 20) — read via that squire's /state.
    let (_st, view) = get(&state, "/state", &squire, &handle).await;
    assert_eq!(view["balance"], 10, "adjustment applied exactly once");
}

// ═════════════════════════════════════════════════════════════════════════════════════════════
// AC-6 — a redemption affordable at request time, drained before approval, fails at approval (409).
// ═════════════════════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn redemption_approval_fails_when_balance_drained_before_review() {
    let (state, store, _dir, _today) = app();
    seed_quest_and_item(&store);
    let reg = register(&state, "House", "Admin", "secret").await;
    let handle = reg.household.0.clone();
    let knight = reg.token.0.clone();
    let (squire_id, squire) = add_squire_and_login(&state, &knight, &handle, "Sib", "s").await;

    // Make the item (cost 3) affordable: fund the squire with 5 points.
    fund(&state, &knight, &handle, squire_id, 5, 8001).await;

    // The squire requests the redemption while it is affordable.
    let request_id = 9100u128;
    let (st, _) = post_json(
        &state,
        "/redemption-requests",
        &squire,
        &handle,
        json!({ "request_id": request_id, "item_id": ITEM_ID }),
    )
    .await;
    assert_eq!(st, StatusCode::OK);

    // Sanity: the squire could afford it at request time.
    let (_st, view) = get(&state, "/state", &squire, &handle).await;
    assert_eq!(view["balance"], 5);

    // Before the Knight approves, DRAIN the balance with a negative adjust (5 → 0).
    let (st, _) = post_json(
        &state,
        "/admin/adjust",
        &knight,
        &handle,
        json!({ "command_id": 8002u128, "squire": squire_id, "amount": -5, "reason": "drain" }),
    )
    .await;
    assert_eq!(st, StatusCode::OK);

    // Now approving the redemption fails affordability at commit time → 409 Conflict.
    let (st, _) = post_json(
        &state,
        "/admin/review-redemption",
        &knight,
        &handle,
        json!({ "request_id": request_id, "decision": "approve" }),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::CONFLICT,
        "approving a redemption the squire can no longer afford is a 409"
    );

    // The request is still pending (the failed approval did not resolve it).
    let (_st, review) = get(&state, "/household-review", &knight, &handle).await;
    let pending = review["pending_requests"].as_array().unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0]["request_id"], request_id as u64);
}
