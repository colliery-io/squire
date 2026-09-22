//! Scaffold tests for the Keep (SQUIRE-T-0025): the loopback bind, the embedded shell + health,
//! the engine-direct `commit` seam, and operator (Knight) login (cookie + bearer, Knight-only).
//! Driven via `tower::ServiceExt::oneshot` — no socket bound.

use std::sync::Arc;

use keep::{admin_addr, bind_target, router, KeepState};

use axum::body::Body;
use axum::http::{Request, StatusCode};

use domain_core::contract::{
    AddMemberReq, Assignment, Cadence, Command, Completion, HouseholdHandle, Quest, QuestId,
    RegisterHouseholdReq, Role, Schedule, UserId,
};
use identity::{Principal, TokenSigner};
use store::tenant::Backend;

use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

/// A Keep over a fresh temp-dir SQLite tenant, plus the registered admin Knight `(id, secret)` and
/// the `TempDir` (kept alive). Registration goes through the real identity flow.
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

    let secret = "excalibur".to_string();
    let reg = state
        .identity
        .register(RegisterHouseholdReq {
            household_name: "Keep".into(),
            admin_name: "Arthur".into(),
            admin_secret: secret.clone(),
        })
        .expect("register admin knight");
    (state.clone(), reg.admin.0, secret, dir)
}

async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

// ─── loopback bind (the A-0008 DEFAULT — defense in depth; authz is the primary control) ─────

#[test]
fn admin_addr_is_loopback_only() {
    let addr = admin_addr(4920);
    assert!(
        addr.ip().is_loopback(),
        "the Keep must bind loopback only, got {addr}"
    );
    assert!(!addr.ip().is_unspecified(), "must never be 0.0.0.0 / LAN");
}

/// `KEEP_BIND` (SQUIRE-T-0129) is opt-in: absent or blank still yields the loopback default, so a
/// bare-metal install is unchanged; a named interface (IP or hostname) is bound verbatim.
#[test]
fn bind_target_defaults_to_loopback_and_honours_override() {
    assert_eq!(bind_target(None, 4920), "127.0.0.1:4920");
    assert_eq!(bind_target(Some("  "), 4920), "127.0.0.1:4920");
    assert_eq!(bind_target(Some("squire"), 4920), "squire:4920");
    assert_eq!(bind_target(Some("172.18.0.2"), 4920), "172.18.0.2:4920");
    assert_eq!(bind_target(Some("::1"), 4920), "[::1]:4920");
    assert_eq!(bind_target(Some("[::1]"), 4920), "[::1]:4920");
}

// ─── embedded shell + health ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn health_is_ok() {
    let (state, _id, _s, _dir) = keep();
    let resp = router(state)
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&bytes[..], b"ok");
}

#[tokio::test]
async fn shell_and_assets_are_served_from_the_binary() {
    let (state, _id, _s, _dir) = keep();

    // The embedded app shell at `/`.
    let resp = router(state.clone())
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(ct.starts_with("text/html"), "shell is html, got {ct}");
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    assert!(
        String::from_utf8_lossy(&bytes).contains("The Keep"),
        "shell renders the Keep"
    );

    // An embedded static asset.
    let resp = router(state.clone())
        .oneshot(
            Request::builder()
                .uri("/static/keep.css")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // A missing asset is a 404 (not a panic).
    let resp = router(state)
        .oneshot(
            Request::builder()
                .uri("/static/nope.css")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ─── the engine-direct commit seam ───────────────────────────────────────────────────────────

#[test]
fn commit_applies_a_change_through_the_single_writer() {
    let (state, admin, _s, _dir) = keep();

    // An authoring command straight through the engine-direct seam (no HTTP, no network api).
    let quest = Quest {
        id: QuestId(100),
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
    };
    let changes = state
        .commit(Some(UserId(admin)), Command::DefineQuest(quest))
        .expect("commit DefineQuest");
    assert!(!changes.is_empty(), "a fresh define emits changes");

    // The write is visible in the single-writer store, and audited to the acting Knight.
    let snap = state.snapshot();
    assert!(
        snap.quests.iter().any(|q| q.id == QuestId(100)),
        "quest is persisted"
    );
    let mut guard = state.store.lock().unwrap();
    let audit = store::quest_audit(&mut guard.connection(), QuestId(100))
        .expect("quest audit")
        .expect("quest row exists");
    assert_eq!(
        audit.created_by,
        Some(UserId(admin)),
        "authoring is audited to the Knight"
    );
}

// ─── operator login (cookie + bearer, Knight-only) ───────────────────────────────────────────

/// `POST /login` form-encoded.
fn login_req(user: u128, secret: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/login")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(format!("user={user}&secret={secret}")))
        .unwrap()
}

/// The Keep's operator login is throttled too (SQUIRE-T-0130): past the allowance of wrong
/// secrets, even the right one gets a 429 and no session cookie.
#[tokio::test]
async fn operator_login_locks_after_repeated_failures() {
    let (state, admin, secret, _dir) = keep();

    for _ in 0..identity::throttle::FREE_FAILURES {
        let resp = router(state.clone())
            .oneshot(login_req(admin, "wrong"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    let resp = router(state)
        .oneshot(login_req(admin, &secret))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(
        resp.headers().get("set-cookie").is_none(),
        "a locked login must not mint a session"
    );
}

#[tokio::test]
async fn operator_login_round_trips_and_gates_whoami() {
    let (state, admin, secret, _dir) = keep();

    // Unauthenticated /api/whoami → 401.
    let resp = router(state.clone())
        .oneshot(
            Request::builder()
                .uri("/api/whoami")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "no session → 401");

    // Login as the admin Knight → 200, sets the session cookie, echoes the Knight.
    let resp = router(state.clone())
        .oneshot(login_req(admin, &secret))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let cookie = resp
        .headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .expect("login sets a cookie")
        .to_string();
    assert!(
        cookie.starts_with("keep_session="),
        "session cookie set: {cookie}"
    );
    assert!(cookie.contains("HttpOnly"), "session cookie is HttpOnly");
    let who = body_json(resp).await;
    assert_eq!(who["user"].as_u64().unwrap() as u128, admin);
    assert_eq!(who["role"], "Knight");
    assert_eq!(who["display_name"], "Arthur");

    // The cookie authenticates /api/whoami.
    let session = cookie.split(';').next().unwrap().to_string(); // `keep_session=<tok>`
    let resp = router(state.clone())
        .oneshot(
            Request::builder()
                .uri("/api/whoami")
                .header("cookie", session)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "session cookie authenticates"
    );
    assert_eq!(
        body_json(resp).await["user"].as_u64().unwrap() as u128,
        admin
    );
}

/// `POST /login` form-encoded with a raw `user` value (display name or id as typed).
fn login_req_str(user: &str, secret: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/login")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(format!("user={user}&secret={secret}")))
        .unwrap()
}

#[tokio::test]
async fn login_accepts_display_name() {
    let (state, admin, secret, _dir) = keep();

    // The fixture admin is "Arthur"; a name resolves case-insensitively to the same Knight.
    let resp = router(state.clone())
        .oneshot(login_req_str("arthur", &secret))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "display-name login works");
    let who = body_json(resp).await;
    assert_eq!(who["user"].as_u64().unwrap() as u128, admin);
    assert_eq!(who["display_name"], "Arthur");

    // An unknown name fails like any bad credential — 401, not 400.
    let resp = router(state)
        .oneshot(login_req_str("mordred", &secret))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_rejects_bad_secret_and_non_knight() {
    let (state, admin, _secret, _dir) = keep();

    // Wrong secret → 401.
    let resp = router(state.clone())
        .oneshot(login_req(admin, "wrong"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Add a Squire (real flow) and prove a Squire cannot operate the Keep → 403.
    let caller = Principal {
        household: HouseholdHandle("keep".into()),
        user: UserId(admin),
        role: Role::Knight,
    };
    let added = state
        .identity
        .add_member(
            &caller,
            AddMemberReq {
                role: Role::Squire,
                display_name: "Gareth".into(),
                initial_secret: "g".into(),
            },
        )
        .expect("add squire");
    let resp = router(state)
        .oneshot(login_req(added.user.0, "g"))
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "a Squire cannot operate the Keep"
    );
}

// ─── change my secret (SQUIRE-T-0131) ────────────────────────────────────────────────────────

fn change_secret_req(cookie: Option<&str>, current: &str, new: &str) -> Request<Body> {
    let mut b = Request::builder()
        .method("POST")
        .uri("/api/me/secret")
        .header("content-type", "application/json");
    if let Some(c) = cookie {
        b = b.header("cookie", c);
    }
    b.body(Body::from(
        serde_json::json!({ "current_secret": current, "new_secret": new }).to_string(),
    ))
    .unwrap()
}

#[tokio::test]
async fn operator_changes_their_own_secret_from_the_keep() {
    let (state, admin, secret, _dir) = keep();

    // No session → 401: this is a privileged route like any other.
    let resp = router(state.clone())
        .oneshot(change_secret_req(None, &secret, "a-brand-new-secret"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let resp = router(state.clone())
        .oneshot(login_req(admin, &secret))
        .await
        .unwrap();
    let cookie = resp
        .headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        .expect("session cookie")
        .to_string();

    // Wrong current secret → 401; too-short new one → 400; neither changes anything.
    let resp = router(state.clone())
        .oneshot(change_secret_req(
            Some(&cookie),
            "wrong",
            "a-brand-new-secret",
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let resp = router(state.clone())
        .oneshot(change_secret_req(Some(&cookie), &secret, "short"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let resp = router(state.clone())
        .oneshot(change_secret_req(
            Some(&cookie),
            &secret,
            "a-brand-new-secret",
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // The old secret no longer signs in; the new one does.
    let resp = router(state.clone())
        .oneshot(login_req(admin, &secret))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let resp = router(state)
        .oneshot(login_req(admin, "a-brand-new-secret"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
