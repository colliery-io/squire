//! End-to-end-ish tests driven through the axum `Router` via `tower::ServiceExt::oneshot` —
//! no real socket is bound. Covers the `GET /health` route and the auth/role gates wired onto
//! a couple of throwaway test-only protected routes (the real feature routes land in
//! T-0015/16/17).

use std::sync::Arc;

use api::auth::{Auth, RequireKnight};
use api::identity::{DevIdentity, Principal};
use api::{router, AppState};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::get;
use axum::Router;

use domain_core::contract::{AuthToken, HouseholdHandle, Role, UserId};
use store::tenant::{Backend, Provisioner};
use store::SystemClock;

use http_body_util::BodyExt;
use tower::ServiceExt; // for `oneshot`

const HANDLE: &str = "house1";
const TOKEN: &str = "tok-knight";
const SQUIRE_TOKEN: &str = "tok-squire";

/// Build an [`AppState`] over a fresh, isolated temp-dir SQLite tenant, with a dev identity
/// seeded with a Knight token and a Squire token, both bound to `house1`.
fn test_state() -> (Arc<AppState>, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let provisioner = Provisioner::new(Backend::Sqlite { dir: dir.path().to_path_buf() });
    let store = provisioner
        .open(HANDLE, SystemClock)
        .expect("open tenant store");

    let identity = DevIdentity::new();
    identity.seed(
        AuthToken(TOKEN.into()),
        Principal {
            household: HouseholdHandle(HANDLE.into()),
            user: UserId(1),
            role: Role::Knight,
        },
    );
    identity.seed(
        AuthToken(SQUIRE_TOKEN.into()),
        Principal {
            household: HouseholdHandle(HANDLE.into()),
            user: UserId(2),
            role: Role::Squire,
        },
    );

    let state = AppState::new(store, Arc::new(identity));
    (state, dir)
}

/// The production router merged with two throwaway protected routes for exercising the
/// extractors. The protected routes are built as a `Router<Arc<AppState>>` and state is
/// applied once at the end (the production `router` already wraps its own state, so we merge
/// before `with_state`).
fn test_router(state: Arc<AppState>) -> Router {
    let protected: Router<Arc<AppState>> = Router::new()
        .route(
            "/protected",
            get(|Auth(p): Auth| async move { format!("user {}", p.user.0) }),
        )
        .route(
            "/knight-only",
            get(|RequireKnight(_): RequireKnight| async move { "knight" }),
        );
    router(state.clone()).merge(protected.with_state(state))
}

fn authed(method_path: &str, token: &str, handle: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder()
        .uri(method_path)
        .header("authorization", format!("Bearer {token}"));
    if let Some(h) = handle {
        builder = builder.header("x-household", h);
    }
    builder.body(Body::empty()).expect("request")
}

async fn status_of(state: &Arc<AppState>, req: Request<Body>) -> StatusCode {
    test_router(state.clone())
        .oneshot(req)
        .await
        .expect("oneshot")
        .status()
}

#[tokio::test]
async fn health_returns_ok() {
    let (state, _dir) = test_state();
    let resp = test_router(state)
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"ok");
}

#[tokio::test]
async fn protected_route_without_token_is_401() {
    let (state, _dir) = test_state();
    let req = Request::builder()
        .uri("/protected")
        .body(Body::empty())
        .unwrap();
    assert_eq!(status_of(&state, req).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn protected_route_with_valid_token_succeeds() {
    let (state, _dir) = test_state();
    let req = authed("/protected", TOKEN, Some(HANDLE));
    let resp = test_router(state.clone()).oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"user 1");
}

#[tokio::test]
async fn token_without_household_header_is_401() {
    let (state, _dir) = test_state();
    let req = authed("/protected", TOKEN, None);
    assert_eq!(status_of(&state, req).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn unknown_token_is_401() {
    let (state, _dir) = test_state();
    let req = authed("/protected", "garbage", Some(HANDLE));
    assert_eq!(status_of(&state, req).await, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn squire_on_knight_route_is_403() {
    let (state, _dir) = test_state();
    let req = authed("/knight-only", SQUIRE_TOKEN, Some(HANDLE));
    assert_eq!(status_of(&state, req).await, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn knight_on_knight_route_succeeds() {
    let (state, _dir) = test_state();
    let req = authed("/knight-only", TOKEN, Some(HANDLE));
    assert_eq!(status_of(&state, req).await, StatusCode::OK);
}
