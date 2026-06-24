//! **Production-identity integration, isolation, no-bypass, MVP-seed & dual-backend** suite for
//! the Local API (SQUIRE-T-0023). Everything is driven through the axum `Router` via
//! `tower::ServiceExt::oneshot` on a **`ProdIdentity`** (real Argon2id credentials + HMAC tokens
//! + a tenant registry), not the in-memory `DevIdentity`.
//!
//! Each scenario runs on every available backend (see [`each_backend_async`]): SQLite always; the
//! compose Postgres under `--features postgres` + `DATABASE_URL` (serialized + schema-cleaned),
//! proving the same flow on both.
//!
//! Coverage:
//! * **AC-1** — register → add-squire → login → claim → review → approve → state, over HTTP, on the
//!   production identity (REQ-1.5/1.6).
//! * **MVP seed** — a household of 2 Knights + 1 Squire built ONLY through the real register /
//!   add-member endpoints (no fixture back-door, REQ-1.10); each member logs in and acts per role.
//! * **Audit** — "who added member X (and when)" is answerable from the `users` audit columns
//!   (REQ-1.12).
//! * **No bypass** — every protected call needs a valid token; missing / garbage / wrong-tenant
//!   credentials are rejected (NFR-2.2).
//! * **Isolation** — two households are fully isolated: a token minted for A reaches only A; B has
//!   no record of A's members, and A's member cannot log into B (no global directory, NFR-2.1).

use std::sync::Arc;

use api::{router, AppState};

use axum::body::Body;
use axum::http::{Request, StatusCode};

use domain_core::contract::{
    Assignment, Availability, Cadence, Change, Completion, Date, HouseholdHandle, ItemId, Quest,
    QuestId, RedeemableItem, RegisterHouseholdResp, Schedule, UserId,
};
use domain_core::contract::{Clock, Repository};
use identity::TokenSigner;
use store::tenant::Backend;

use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt; // for `oneshot`

// Authored (NOT on the wire) — seeded into the shared store directly.
const DAILY_QUEST_ID: u128 = 100; // auto_approve = false → the review flow
const ITEM_ID: u128 = 200; // an affordable reward (cost 3)

/// The shared HMAC signing key every test's [`TokenSigner`] uses. Two signers built from the same
/// key produce cross-verifiable tokens — exactly the realistic threat the isolation test probes:
/// a *cryptographically valid* token from household A presented to household B.
const SIGNING_KEY: &[u8] = b"squire-prod-integration-test-key";

const TTL_MS: i64 = 60 * 60 * 1000; // 1h — comfortably longer than any test.

// ─── backend harness ─────────────────────────────────────────────────────────────────────────

/// Run `f` once per available backend: SQLite always (a fresh `TempDir`), Postgres under
/// `--features postgres` + `DATABASE_URL` (serialized behind a process-wide lock and with the
/// `handles`' schemas pre/post-cleaned). Mirrors the store / identity `tenant.rs` idiom, adapted
/// to async (`f` returns a future awaited inline).
#[cfg_attr(not(feature = "postgres"), allow(unused_variables))]
async fn each_backend_async<F, Fut>(handles: &[&str], f: F)
where
    F: Fn(Backend) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    // SQLite — always. Base dir is a TempDir that drops (and deletes) at scope end.
    {
        let dir = tempfile::tempdir().expect("tempdir");
        f(Backend::Sqlite {
            dir: dir.path().to_path_buf(),
        })
        .await;
    }

    // Postgres — opt-in, serialized behind a Mutex (the tests share one database).
    #[cfg(feature = "postgres")]
    {
        static PG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

        if let Ok(url) = std::env::var("DATABASE_URL") {
            let _guard = PG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            let cleaner = store::tenant::Provisioner::new(Backend::Postgres {
                base_url: url.clone(),
            });
            for h in handles {
                cleaner.deprovision(h).expect("pre-clean schema");
            }
            f(Backend::Postgres {
                base_url: url.clone(),
            })
            .await;
            for h in handles {
                cleaner.deprovision(h).expect("post-clean schema");
            }
        } else {
            eprintln!("skipping postgres prod-integration: DATABASE_URL not set");
        }
    }
}

/// A fresh production `AppState` bound to `handle` over `backend`, plus the shared store handle (so
/// a test can author quests/items directly — authoring is intentionally NOT a wire endpoint) and
/// `today`. The identity is a real [`ProdIdentity`]; no users exist until registration over HTTP.
fn prod_app(backend: Backend, handle: &str) -> (Arc<AppState>, Date) {
    let state = AppState::local_prod(
        backend,
        HouseholdHandle(handle.to_string()),
        TokenSigner::new(SIGNING_KEY),
        TTL_MS,
    )
    .expect("provision + wire prod AppState");
    let today = state
        .store
        .lock()
        .expect("store mutex poisoned")
        .clock()
        .today();
    (state, today)
}

/// Author a daily quest assigned to ALL squires (auto_approve = false → the Knight review flow) and
/// one affordable item, written straight into the shared store (authoring is off the wire).
fn seed_quest_and_item(state: &Arc<AppState>) {
    let quest = Quest {
        id: QuestId(DAILY_QUEST_ID),
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
    state
        .store
        .lock()
        .expect("store mutex poisoned")
        .apply(None, &[Change::PutQuest(quest), Change::PutItem(item)])
        .expect("seed quest + item");
}

// ─── tiny wire helpers ─────────────────────────────────────────────────────────────────────────

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
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, value)
}

async fn post_json(
    state: &Arc<AppState>,
    path: &str,
    token: &str,
    handle: &str,
    body: Value,
) -> (StatusCode, Value) {
    let resp = router(state.clone())
        .oneshot(build(
            "POST",
            path,
            Some((token, handle)),
            Some(body.to_string()),
        ))
        .await
        .unwrap();
    status_and_json(resp).await
}

async fn get(state: &Arc<AppState>, path: &str, token: &str, handle: &str) -> (StatusCode, Value) {
    let resp = router(state.clone())
        .oneshot(build("GET", path, Some((token, handle)), None))
        .await
        .unwrap();
    status_and_json(resp).await
}

// ─── over-the-wire register / login / add-member ─────────────────────────────────────────────

/// `POST /register` (unauthenticated) → the household handle + the first Knight's token.
async fn register(
    state: &Arc<AppState>,
    name: &str,
    admin: &str,
    secret: &str,
) -> RegisterHouseholdResp {
    let body = json!({ "household_name": name, "admin_name": admin, "admin_secret": secret });
    let resp = router(state.clone())
        .oneshot(build("POST", "/register", None, Some(body.to_string())))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "register should be 200");
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).expect("RegisterHouseholdResp")
}

/// `POST /members` (Knight-only) → the new member's user id.
async fn add_member(
    state: &Arc<AppState>,
    knight: &str,
    handle: &str,
    role: &str,
    name: &str,
    secret: &str,
) -> u128 {
    let (st, add) = post_json(
        state,
        "/members",
        knight,
        handle,
        json!({ "role": role, "display_name": name, "initial_secret": secret }),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "add member should be 200");
    add["user"].as_u64().expect("user id") as u128
}

/// `POST /login` (unauthenticated) → a fresh token for `(handle, user, secret)`, asserting `role`.
async fn login(
    state: &Arc<AppState>,
    handle: &str,
    user: u128,
    secret: &str,
    role: &str,
) -> String {
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
    assert_eq!(login["role"], role, "login role");
    login["token"].as_str().expect("token").to_string()
}

/// Read the `users` audit columns for `user` directly from the tenant store (the audit trail is
/// intentionally absent from the domain `Snapshot`).
fn created_by(state: &Arc<AppState>, user: u128) -> Option<UserId> {
    let mut guard = state.store.lock().expect("store mutex poisoned");
    let cols = store::user_audit(&mut guard.connection(), UserId(user))
        .expect("user_audit query")
        .expect("user row exists");
    cols.created_by
}

// ═════════════════════════════════════════════════════════════════════════════════════════════
// AC-1 — the full claim → review → credit flow over HTTP, on the PRODUCTION identity.
// ═════════════════════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn ac1_full_flow_over_http_on_prod() {
    each_backend_async(&["round_table"], |backend| async move {
        let (state, today) = prod_app(backend, "round_table");
        seed_quest_and_item(&state);

        let reg = register(&state, "Round Table", "Arthur", "excalibur").await;
        let handle = reg.household.0.clone();
        let knight = reg.token.0.clone();

        let squire_id = add_member(&state, &knight, &handle, "Squire", "Lancelot", "lake").await;
        let squire = login(&state, &handle, squire_id, "lake", "Squire").await;

        // Squire claims → Pending (auto_approve = false).
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
        assert_eq!(claim["state"]["state"], "Pending");

        // Knight approves.
        let (st, _) = post_json(
            &state,
            "/admin/review-claim",
            &knight,
            &handle,
            json!({ "claim_id": claim_id, "decision": { "verdict": "approve" } }),
        )
        .await;
        assert_eq!(st, StatusCode::OK);

        // The Squire's own /state shows the credit (reward = 5).
        let (st, view) = get(&state, "/state", &squire, &handle).await;
        assert_eq!(st, StatusCode::OK);
        assert_eq!(view["squire"], squire_id as u64);
        assert_eq!(view["balance"], 5);
        // Flat tagged object: { "state": "Approved", "points": 5 } (SQUIRE-T-0033).
        assert_eq!(view["my_claims"][0]["state"]["state"], "Approved");
        assert_eq!(view["my_claims"][0]["state"]["points"], 5);
    })
    .await;
}

// ═════════════════════════════════════════════════════════════════════════════════════════════
// MVP seed — 2 Knights + 1 Squire, built ONLY through register + add-member (no fixture). REQ-1.10.
// ═════════════════════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn mvp_seed_two_knights_one_squire_through_real_flow() {
    each_backend_async(&["keep"], |backend| async move {
        let (state, today) = prod_app(backend, "keep");
        seed_quest_and_item(&state);

        // Register seeds the FIRST Knight (the admin). System seed → no `created_by`.
        let reg = register(&state, "Keep", "Arthur", "k1secret").await;
        let handle = reg.household.0.clone();
        let knight1 = reg.token.0.clone();
        let knight1_id = reg.admin.0;

        // Knight #1 adds a SECOND Knight and a Squire — through the real Knight-only endpoint.
        let knight2_id =
            add_member(&state, &knight1, &handle, "Knight", "Lancelot", "k2secret").await;
        let squire_id = add_member(&state, &knight1, &handle, "Squire", "Gareth", "sqsecret").await;

        // Every member can log in and is the role they were created as.
        let knight2 = login(&state, &handle, knight2_id, "k2secret", "Knight").await;
        let squire = login(&state, &handle, squire_id, "sqsecret", "Squire").await;

        // The household has exactly 3 members: 2 Knights + 1 Squire (no fixture back-door).
        {
            let snap = state.store.lock().expect("store mutex poisoned").snapshot();
            assert_eq!(snap.users.len(), 3, "exactly the 3 seeded members");
            let knights = snap
                .users
                .iter()
                .filter(|u| u.role == domain_core::contract::Role::Knight)
                .count();
            let squires = snap
                .users
                .iter()
                .filter(|u| u.role == domain_core::contract::Role::Squire)
                .count();
            assert_eq!((knights, squires), (2, 1), "2 Knights + 1 Squire");
        }

        // The Squire acts (claim) and the SECOND Knight approves — both seeded members are usable.
        let claim_id = 7777u128;
        let (st, _) = post_json(
            &state,
            "/claims",
            &squire,
            &handle,
            json!({ "claim_id": claim_id, "quest_id": DAILY_QUEST_ID, "on": today.0 }),
        )
        .await;
        assert_eq!(st, StatusCode::OK);
        let (st, _) = post_json(
            &state,
            "/admin/review-claim",
            &knight2,
            &handle,
            json!({ "claim_id": claim_id, "decision": { "verdict": "approve" } }),
        )
        .await;
        assert_eq!(st, StatusCode::OK, "the second Knight can approve too");

        // Audit (REQ-1.12): who added member X? The admin Knight was a system seed; the two added
        // members both record Knight #1 as `created_by`.
        assert_eq!(
            created_by(&state, knight1_id),
            None,
            "the admin Knight is a system seed"
        );
        assert_eq!(
            created_by(&state, knight2_id),
            Some(UserId(knight1_id)),
            "Knight #1 added Knight #2"
        );
        assert_eq!(
            created_by(&state, squire_id),
            Some(UserId(knight1_id)),
            "Knight #1 added the Squire"
        );
    })
    .await;
}

// ═════════════════════════════════════════════════════════════════════════════════════════════
// No account bypass — every protected call needs a valid token (NFR-2.2).
// ═════════════════════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn no_account_bypass_on_prod() {
    each_backend_async(&["vault"], |backend| async move {
        let (state, _today) = prod_app(backend, "vault");
        seed_quest_and_item(&state);
        let reg = register(&state, "Vault", "Admin", "secret").await;
        let handle = reg.household.0.clone();
        let knight = reg.token.0.clone();
        let squire_id = add_member(&state, &knight, &handle, "Squire", "Sib", "s").await;
        let squire = login(&state, &handle, squire_id, "s", "Squire").await;

        // No Authorization header at all → 401.
        let resp = router(state.clone())
            .oneshot(build("GET", "/state", None, None))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "no token → 401");

        // A garbage / forged bearer token → 401 (the HMAC signature fails).
        let (st, _) = get(&state, "/state", "not-a-real-token", &handle).await;
        assert_eq!(st, StatusCode::UNAUTHORIZED, "garbage token → 401");

        // A signature-valid token but presented with a foreign household handle → 401.
        let (st, _) = get(&state, "/state", &squire, "some-other-household").await;
        assert_eq!(
            st,
            StatusCode::UNAUTHORIZED,
            "valid token + wrong household → 401"
        );
    })
    .await;
}

// ═════════════════════════════════════════════════════════════════════════════════════════════
// Isolation — two households are fully isolated (NFR-2.1: no cross-tenant reach, no global dir).
// ═════════════════════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn two_households_are_fully_isolated() {
    each_backend_async(&["house_a", "house_b"], |backend| async move {
        // Two separate tenants (own file / own schema), each its own production AppState. The
        // signers share a key, so A's token is *cryptographically valid* everywhere — isolation
        // must come from tenant binding, not from the token failing to verify.
        let (app_a, _) = prod_app(backend.clone(), "house_a");
        let (app_b, _) = prod_app(backend, "house_b");

        let reg_a = register(&app_a, "House A", "Alice", "asecret").await;
        let reg_b = register(&app_b, "House B", "Bob", "bsecret").await;
        let a_handle = reg_a.household.0.clone();
        let b_handle = reg_b.household.0.clone();
        let a_token = reg_a.token.0.clone();
        let a_admin = reg_a.admin.0;

        // Sanity: A's token works against A.
        let (st, _) = get(&app_a, "/household-review", &a_token, &a_handle).await;
        assert_eq!(st, StatusCode::OK, "A's token works on A");

        // A's token reaches NOTHING on B — with B's handle (token/handle mismatch) ...
        let (st, _) = get(&app_b, "/household-review", &a_token, &b_handle).await;
        assert_eq!(
            st,
            StatusCode::UNAUTHORIZED,
            "A's token + B handle on B → 401"
        );
        // ... and with A's own handle (B serves only B; a foreign-tenant handle is refused).
        let (st, _) = get(&app_b, "/household-review", &a_token, &a_handle).await;
        assert_eq!(
            st,
            StatusCode::UNAUTHORIZED,
            "A's token + A handle on B → 401"
        );

        // No global directory: A's admin cannot log into B (B holds no credential for that id), and
        // B's user list contains none of A's members.
        let resp = router(app_b.clone())
            .oneshot(build(
                "POST",
                "/login",
                None,
                Some(
                    json!({ "household": b_handle, "user": a_admin, "secret": "asecret" })
                        .to_string(),
                ),
            ))
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "A's admin cannot log into B"
        );

        // B's directory holds ONLY its own admin (Bob) — none of A's members. (Ids are minted
        // per-tenant, so both admins happen to be `UserId(1)` in their *separate* tenants; identity
        // here is the member data, which is wholly B's.)
        let b_users = app_b
            .store
            .lock()
            .expect("store mutex poisoned")
            .snapshot()
            .users;
        assert_eq!(b_users.len(), 1, "B has only its own admin");
        assert_eq!(
            b_users[0].display_name, "Bob",
            "B's sole member is Bob, not anyone from A"
        );
    })
    .await;
}

// ═════════════════════════════════════════════════════════════════════════════════════════════
// Concurrency — the api services requests concurrently; control-plane and feature writes share ONE
// store, so they serialize on one lock/connection (AR-1) instead of racing as two connections.
// ═════════════════════════════════════════════════════════════════════════════════════════════

/// SQLite-only (this is where two uncoordinated connections would collide with `SQLITE_BUSY`): fire
/// many concurrent `/admin/adjust` (control-adjacent feature writes) AND `/members` (control-plane
/// writes via `ProdIdentity`) on separate tasks. With the shared store every write succeeds and
/// applies exactly once — no lost updates, no busy errors.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_control_plane_and_feature_writes_serialize() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (state, _today) = prod_app(
        Backend::Sqlite {
            dir: dir.path().to_path_buf(),
        },
        "concurrent",
    );
    seed_quest_and_item(&state);
    let reg = register(&state, "Concurrent", "Admin", "secret").await;
    let handle = reg.household.0.clone();
    let knight = reg.token.0.clone();
    let squire_id = add_member(&state, &knight, &handle, "Squire", "Sib", "s").await;

    const ADJUSTS: u128 = 12; // each +1, distinct command_id → total credit must equal ADJUSTS
    const ADDS: usize = 8; // concurrent new squires via the control plane

    let mut tasks = Vec::new();
    for i in 0..ADJUSTS {
        let (s, h, k) = (state.clone(), handle.clone(), knight.clone());
        tasks.push(tokio::spawn(async move {
            let (st, _) = post_json(
                &s,
                "/admin/adjust",
                &k,
                &h,
                json!({ "command_id": 10_000 + i, "squire": squire_id, "amount": 1, "reason": "bump" }),
            )
            .await;
            assert_eq!(st, StatusCode::OK, "concurrent adjust must succeed (no SQLITE_BUSY)");
        }));
    }
    for i in 0..ADDS {
        let (s, h, k) = (state.clone(), handle.clone(), knight.clone());
        tasks.push(tokio::spawn(async move {
            let (st, _) = post_json(
                &s,
                "/members",
                &k,
                &h,
                json!({ "role": "Squire", "display_name": format!("Sq{i}"), "initial_secret": "x" }),
            )
            .await;
            assert_eq!(st, StatusCode::OK, "concurrent add_member must succeed (no SQLITE_BUSY)");
        }));
    }
    for t in tasks {
        t.await.expect("task panicked");
    }

    // Exactly ADJUSTS credited (no lost updates) and exactly 1 + ADDS squires created (no races).
    let (st, review) = get(&state, "/household-review", &knight, &handle).await;
    assert_eq!(st, StatusCode::OK);
    let squires = review["squires"].as_array().unwrap();
    assert_eq!(
        squires.len(),
        1 + ADDS,
        "every concurrent add_member created exactly one squire"
    );
    let balance = squires
        .iter()
        .find(|s| s["squire"] == squire_id as u64)
        .expect("the funded squire")["balance"]
        .as_i64()
        .unwrap();
    assert_eq!(
        balance, ADJUSTS as i64,
        "every concurrent adjust applied exactly once"
    );
}
