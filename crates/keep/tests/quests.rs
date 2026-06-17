//! Quest-authoring tests for the Keep (SQUIRE-T-0026): create/edit/archive over the engine-direct
//! API, last-editor audit, forward-only reward edits, assignment + completion round-trip, and the
//! auth/validation guards. Driven via `oneshot`.

use std::collections::BTreeSet;
use std::sync::Arc;

use keep::{router, KeepState};

use axum::body::Body;
use axum::http::{Request, StatusCode};

use domain_core::contract::{
    AddMemberReq, Assignment, Cadence, ClaimId, Clock, Command, Completion, Decision, Projections,
    Quest, QuestId, RegisterHouseholdReq, Role, Schedule, UserId,
};
use domain_core::Proj;
use identity::{Principal, TokenSigner};
use store::tenant::Backend;

use http_body_util::BodyExt;
use serde_json::{to_value, Value};
use tower::ServiceExt;

const HANDLE: &str = "keep";

/// A Keep + the admin Knight's bearer token (from registration). Authoring API calls authenticate
/// with `Authorization: Bearer <token>`.
fn keep() -> (Arc<KeepState>, u128, String, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let backend = Backend::Sqlite { dir: dir.path().to_path_buf() };
    let state = KeepState::local(
        backend,
        domain_core::contract::HouseholdHandle(HANDLE.into()),
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
    (state.clone(), reg.admin.0, reg.token.0, dir)
}

/// Add a Squire through the real identity flow; returns its `UserId`.
fn add_squire(state: &Arc<KeepState>, admin: u128) -> UserId {
    let caller = Principal {
        household: domain_core::contract::HouseholdHandle(HANDLE.into()),
        user: UserId(admin),
        role: Role::Knight,
    };
    state
        .identity
        .add_member(
            &caller,
            AddMemberReq { role: Role::Squire, display_name: "Gareth".into(), initial_secret: "g".into() },
        )
        .expect("add squire")
        .user
}

/// A daily quest, AllSquires / EachAssignee, with the given id + reward.
fn daily_quest(id: u128, reward: u32) -> Quest {
    Quest {
        id: QuestId(id),
        title: "Tidy room".into(),
        description: None,
        category: None,
        reward,
        cadence: Cadence::Recurring(Schedule::Daily),
        assignment: Assignment::AllSquires,
        completion: Completion::EachAssignee,
        auto_approve: false,
        repeatable_within_day: false,
        active: true,
        icon: None,
    }
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
    let value = if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap_or(Value::Null) };
    (status, value)
}

/// Approve a fresh claim for `squire` on `quest`/`on` via the engine-direct seam (used to exercise
/// reward snapshotting). The acting Knight is `admin`.
fn claim_and_approve(state: &Arc<KeepState>, admin: u128, squire: UserId, quest: u128, on: domain_core::contract::Date, claim: u128) {
    state
        .commit(None, Command::SubmitClaim { claim_id: ClaimId(claim), squire, quest_id: QuestId(quest), on })
        .expect("submit");
    state
        .commit(Some(UserId(admin)), Command::ReviewClaim { actor: UserId(admin), claim_id: ClaimId(claim), decision: Decision::Approve })
        .expect("approve");
}

fn balance(state: &Arc<KeepState>, squire: UserId) -> i64 {
    Proj::balance(&state.snapshot(), squire)
}

// ─── create / list / audit ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_lists_and_audits_to_the_knight() {
    let (state, admin, token, _dir) = keep();

    let (st, body) = send(&state, "POST", "/api/quests", Some(&token), Some(to_value(daily_quest(100, 5)).unwrap())).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(body["id"], "100");

    let (st, list) = send(&state, "GET", "/api/quests", Some(&token), None).await;
    assert_eq!(st, StatusCode::OK);
    let row = list.as_array().unwrap().iter().find(|r| r["quest"]["id"] == 100).expect("quest in list");
    assert_eq!(row["quest"]["active"], true);
    assert_eq!(row["quest"]["reward"], 5);
    assert_eq!(row["audit"]["created_by"], admin.to_string(), "authoring audited to the Knight");
}

#[tokio::test]
async fn unauthenticated_create_is_401() {
    let (state, _admin, _token, _dir) = keep();
    let (st, _) = send(&state, "POST", "/api/quests", None, Some(to_value(daily_quest(1, 5)).unwrap())).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn invalid_definition_is_400() {
    let (state, _admin, token, _dir) = keep();
    // Assignment::Squires(empty) is an invalid definition → 400 (not a 500).
    let mut q = daily_quest(7, 5);
    q.assignment = Assignment::Squires(BTreeSet::new());
    let (st, _) = send(&state, "POST", "/api/quests", Some(&token), Some(to_value(q).unwrap())).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
}

// ─── forward-only reward edits ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn reward_edits_are_forward_only() {
    let (state, admin, token, _dir) = keep();
    let squire = add_squire(&state, admin);
    let today = state.clock.today();

    // Define reward 5, approve a claim → +5.
    let (st, _) = send(&state, "POST", "/api/quests", Some(&token), Some(to_value(daily_quest(100, 5)).unwrap())).await;
    assert_eq!(st, StatusCode::OK);
    claim_and_approve(&state, admin, squire, 100, today, 9001);
    assert_eq!(balance(&state, squire), 5);

    // Edit the reward to 10 (same id → upsert). The PAST payout is untouched (still 5).
    let (st, _) = send(&state, "POST", "/api/quests", Some(&token), Some(to_value(daily_quest(100, 10)).unwrap())).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(balance(&state, squire), 5, "editing reward must not rewrite a past approval");

    // A NEW approval (next day) snapshots the new reward → +10 → 15 total.
    claim_and_approve(&state, admin, squire, 100, domain_core::contract::Date(today.0 + 1), 9002);
    assert_eq!(balance(&state, squire), 15, "future approvals use the new reward");
}

// ─── archive (never delete) ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn archive_deactivates_but_keeps_history() {
    let (state, admin, token, _dir) = keep();
    let squire = add_squire(&state, admin);
    let today = state.clock.today();
    send(&state, "POST", "/api/quests", Some(&token), Some(to_value(daily_quest(100, 5)).unwrap())).await;
    claim_and_approve(&state, admin, squire, 100, today, 9001);
    assert_eq!(balance(&state, squire), 5);

    // Archive → 204; the quest is now inactive but the historical credit stands.
    let (st, _) = send(&state, "POST", "/api/quests/100/archive", Some(&token), None).await;
    assert_eq!(st, StatusCode::NO_CONTENT);
    let (_st, list) = send(&state, "GET", "/api/quests", Some(&token), None).await;
    let row = list.as_array().unwrap().iter().find(|r| r["quest"]["id"] == 100).unwrap();
    assert_eq!(row["quest"]["active"], false, "archived, not deleted");
    assert_eq!(balance(&state, squire), 5, "history survives archive");
}

#[tokio::test]
async fn archive_missing_quest_is_404() {
    let (state, _admin, token, _dir) = keep();
    let (st, _) = send(&state, "POST", "/api/quests/424242/archive", Some(&token), None).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
}

// ─── assignment + completion round-trip (incl. Race) ─────────────────────────────────────────

#[tokio::test]
async fn race_quest_assignment_and_completion_round_trip() {
    let (state, admin, token, _dir) = keep();
    let squire = add_squire(&state, admin);

    let mut q = daily_quest(200, 8);
    q.completion = Completion::Race;
    q.assignment = Assignment::Squires(BTreeSet::from([squire]));
    let (st, _) = send(&state, "POST", "/api/quests", Some(&token), Some(to_value(q).unwrap())).await;
    assert_eq!(st, StatusCode::OK);

    let (_st, list) = send(&state, "GET", "/api/quests", Some(&token), None).await;
    let row = list.as_array().unwrap().iter().find(|r| r["quest"]["id"] == 200).unwrap();
    assert_eq!(row["quest"]["completion"], "Race");
    let assigned = row["quest"]["assignment"]["Squires"].as_array().expect("explicit squire subset");
    assert_eq!(assigned.len(), 1, "the one assigned squire round-trips");
}
