//! End-to-end-ish tests for the three control-plane endpoints (SQUIRE-T-0017): `POST /register`,
//! `POST /login`, and `POST /members`, driven through the axum `Router` via
//! `tower::ServiceExt::oneshot` — no socket bound.
//!
//! Unlike the T-0015/16 tests, nothing is pre-seeded: registration bootstraps the household and
//! its first Knight, and the dev identity manages members in the shared tenant store. Coverage:
//! * `/register` (no auth) → 200; the returned admin token works on a `RequireKnight` route and
//!   (being a Knight) is 403 on the Squire-only `/state`;
//! * `/members` (Knight) creating a Squire → `/login` for that Squire → its token works on `/state`;
//! * `/members` with a Squire token → 403;
//! * `/login` with a wrong secret → 401.

use std::sync::{Arc, Mutex};

use identity::DevIdentity;
use api::{router, AppState};

use axum::body::Body;
use axum::http::{Request, StatusCode};

use domain_core::contract::{
    AddMemberResp, LoginResp, RegisterHouseholdResp, Role,
};
use store::tenant::{Backend, Provisioner};
use store::SystemClock;

use http_body_util::BodyExt;
use tower::ServiceExt; // for `oneshot`

const HANDLE: &str = "house1";

/// Build an [`AppState`] over a fresh, empty temp-dir SQLite tenant with a dev identity sharing
/// that store. No users are seeded — registration creates the first Knight.
fn test_state() -> (Arc<AppState>, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let provisioner = Provisioner::new(Backend::Sqlite { dir: dir.path().to_path_buf() });
    let store = provisioner.open(HANDLE, SystemClock).expect("open tenant store");

    let store = Arc::new(Mutex::new(store));
    let identity = DevIdentity::new(store.clone());
    let state = AppState::new(store, Arc::new(identity));
    (state, dir)
}

/// Build a request, optionally authenticated with `(token, household)`, optionally with a JSON body.
fn req(method: &str, path: &str, auth: Option<(&str, &str)>, body: Option<String>) -> Request<Body> {
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

async fn json_body<T: serde::de::DeserializeOwned>(resp: axum::response::Response) -> T {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).expect("deserialize body")
}

/// Register a household and return the admin's `RegisterHouseholdResp`.
async fn register(state: Arc<AppState>) -> RegisterHouseholdResp {
    let body = serde_json::json!({
        "household_name": "The Round Table",
        "admin_name": "Arthur",
        "admin_secret": "excalibur",
    })
    .to_string();
    let resp = router(state)
        .oneshot(req("POST", "/register", None, Some(body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    json_body(resp).await
}

#[tokio::test]
async fn register_is_unauthenticated_and_mints_a_working_knight_token() {
    let (state, _dir) = test_state();
    let reg = register(state.clone()).await;

    // The minted admin token works on a RequireKnight route.
    let resp = router(state.clone())
        .oneshot(req("GET", "/household-review", Some((&reg.token.0, &reg.household.0)), None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // …and, being a Knight (not a Squire), is 403 on the Squire-only /state.
    let resp = router(state)
        .oneshot(req("GET", "/state", Some((&reg.token.0, &reg.household.0)), None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn knight_adds_squire_who_can_login_and_reach_squire_routes() {
    let (state, _dir) = test_state();
    let reg = register(state.clone()).await;

    // The admin Knight adds a Squire member.
    let add_body = serde_json::json!({
        "role": "Squire",
        "display_name": "Lancelot",
        "initial_secret": "lake",
    })
    .to_string();
    let resp = router(state.clone())
        .oneshot(req("POST", "/members", Some((&reg.token.0, &reg.household.0)), Some(add_body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let added: AddMemberResp = json_body(resp).await;

    // The new Squire logs in with (household, user, secret).
    let login_body = serde_json::json!({
        "household": reg.household.0,
        "user": added.user.0,
        "secret": "lake",
    })
    .to_string();
    let resp = router(state.clone())
        .oneshot(req("POST", "/login", None, Some(login_body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let login: LoginResp = json_body(resp).await;
    assert!(matches!(login.role, Role::Squire));

    // That Squire token works on the Squire-only /state.
    let resp = router(state)
        .oneshot(req("GET", "/state", Some((&login.token.0, &reg.household.0)), None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn members_with_squire_token_is_403() {
    let (state, _dir) = test_state();
    let reg = register(state.clone()).await;

    // Add a Squire, then log them in to get a Squire token.
    let add_body = serde_json::json!({
        "role": "Squire", "display_name": "Lancelot", "initial_secret": "lake",
    })
    .to_string();
    let added: AddMemberResp = json_body(
        router(state.clone())
            .oneshot(req("POST", "/members", Some((&reg.token.0, &reg.household.0)), Some(add_body)))
            .await
            .unwrap(),
    )
    .await;
    let login_body = serde_json::json!({
        "household": reg.household.0, "user": added.user.0, "secret": "lake",
    })
    .to_string();
    let login: LoginResp = json_body(
        router(state.clone())
            .oneshot(req("POST", "/login", None, Some(login_body)))
            .await
            .unwrap(),
    )
    .await;

    // The Squire token on /members is a 403 (RequireKnight extractor).
    let add_body2 = serde_json::json!({
        "role": "Squire", "display_name": "Galahad", "initial_secret": "grail",
    })
    .to_string();
    let resp = router(state)
        .oneshot(req("POST", "/members", Some((&login.token.0, &reg.household.0)), Some(add_body2)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn login_with_wrong_secret_is_401() {
    let (state, _dir) = test_state();
    let reg = register(state.clone()).await;

    let login_body = serde_json::json!({
        "household": reg.household.0, "user": reg.admin.0, "secret": "wrong",
    })
    .to_string();
    let resp = router(state)
        .oneshot(req("POST", "/login", None, Some(login_body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// A Knight renames a member in place (SQUIRE-T-0120/0126): the roster shows the new name on the
/// SAME id with the role preserved (it's a `PutUser` upsert), the old name is gone, and a blank
/// name is rejected.
#[tokio::test]
async fn knight_renames_a_member_in_place() {
    let (state, _dir) = test_state();
    let reg = register(state.clone()).await;
    let auth = (reg.token.0.as_str(), reg.household.0.as_str());

    // Add a Squire.
    let add_body =
        serde_json::json!({ "role": "Squire", "display_name": "Lancelot", "initial_secret": "lake" }).to_string();
    let resp = router(state.clone())
        .oneshot(req("POST", "/members", Some(auth), Some(add_body)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let added: AddMemberResp = json_body(resp).await;
    let name_path = format!("/admin/members/{}/name", added.user.0);

    // Blank name → 400 (no change).
    let resp = router(state.clone())
        .oneshot(req("POST", &name_path, Some(auth), Some(serde_json::json!({ "display_name": "  " }).to_string())))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Rename in place → 204.
    let resp = router(state.clone())
        .oneshot(req("POST", &name_path, Some(auth), Some(serde_json::json!({ "display_name": "Galahad" }).to_string())))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Renaming an unknown member → 404.
    let resp = router(state.clone())
        .oneshot(req("POST", "/admin/members/999999/name", Some(auth), Some(serde_json::json!({ "display_name": "Nobody" }).to_string())))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // The roster shows the new name on the same id, role preserved; old name gone; no duplicate.
    let resp = router(state)
        .oneshot(req("GET", "/admin/members", Some(auth), None))
        .await
        .unwrap();
    let members: serde_json::Value = json_body(resp).await;
    let arr = members.as_array().expect("members array");
    assert_eq!(arr.len(), 2, "still just the Knight + the (renamed) Squire — no duplicate");
    let galahad = arr.iter().find(|m| m["display_name"] == "Galahad").expect("the renamed member");
    assert_eq!(galahad["role"], "Squire", "the upsert preserves the role");
    assert!(arr.iter().all(|m| m["display_name"] != "Lancelot"), "the old name is gone (renamed, not duplicated)");
}
