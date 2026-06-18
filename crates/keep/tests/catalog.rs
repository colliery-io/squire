//! Item + achievement authoring tests for the Keep (SQUIRE-T-0027): create/edit/archive over the
//! engine-direct API, last-editor audit, gate validation, and the log-derived `last_redeemed` /
//! out-of-stock surfacing. Driven via `oneshot`.

use std::sync::Arc;

use keep::{router, KeepState};

use axum::body::Body;
use axum::http::{Request, StatusCode};

use domain_core::contract::{
    Achievement, AchievementId, AddMemberReq, Availability, Category, Command, CommandId, Criterion,
    HouseholdHandle, ItemId, RedeemableItem, RegisterHouseholdReq, Role, Scope, StreakBasis, UserId,
};
use identity::{Principal, TokenSigner};
use store::tenant::Backend;

use http_body_util::BodyExt;
use serde_json::{to_value, Value};
use tower::ServiceExt;

const HANDLE: &str = "keep";

fn keep() -> (Arc<KeepState>, u128, String, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let backend = Backend::Sqlite { dir: dir.path().to_path_buf() };
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
    (state.clone(), reg.admin.0, reg.token.0, dir)
}

fn add_squire(state: &Arc<KeepState>, admin: u128) -> UserId {
    let caller = Principal { household: HouseholdHandle(HANDLE.into()), user: UserId(admin), role: Role::Knight };
    state
        .identity
        .add_member(
            &caller,
            AddMemberReq { role: Role::Squire, display_name: "Gareth".into(), initial_secret: "g".into() },
        )
        .expect("add squire")
        .user
}

fn item(id: u128, cost: u32, availability: Availability, gate: Option<u128>) -> RedeemableItem {
    RedeemableItem {
        id: ItemId(id),
        name: "Ice cream".into(),
        description: None,
        cost,
        gate: gate.map(AchievementId),
        availability,
        active: true,
        icon: None,
    }
}

fn points_achievement(id: u128, total: u32) -> Achievement {
    Achievement {
        id: AchievementId(id),
        name: "Saver".into(),
        description: None,
        criterion: Criterion::PointsEarned { total },
        bonus_points: 10,
        active: true,
    }
}

async fn send(state: &Arc<KeepState>, method: &str, path: &str, token: Option<&str>, body: Option<Value>) -> (StatusCode, Value) {
    let mut b = Request::builder().method(method).uri(path);
    if let Some(t) = token {
        b = b.header("authorization", format!("Bearer {t}"));
    }
    let req = match body {
        Some(v) => b.header("content-type", "application/json").body(Body::from(v.to_string())).unwrap(),
        None => b.body(Body::empty()).unwrap(),
    };
    let resp = router(state.clone()).oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let value = if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap_or(Value::Null) };
    (status, value)
}

// ─── items ───────────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_item_lists_and_audits() {
    let (state, admin, token, _dir) = keep();
    let (st, body) = send(&state, "POST", "/api/items", Some(&token), Some(to_value(item(200, 3, Availability::Repeatable, None)).unwrap())).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(body["id"], "200");

    let (_st, list) = send(&state, "GET", "/api/items", Some(&token), None).await;
    let row = list.as_array().unwrap().iter().find(|r| r["item"]["id"] == 200).unwrap();
    assert_eq!(row["item"]["cost"], 3);
    assert_eq!(row["audit"]["created_by"], admin.to_string());
    assert_eq!(row["last_redeemed"], Value::Null, "never redeemed yet");
    assert_eq!(row["out_of_stock"], false);
}

#[tokio::test]
async fn item_gate_to_missing_achievement_is_404() {
    let (state, _admin, token, _dir) = keep();
    let (st, _) = send(&state, "POST", "/api/items", Some(&token), Some(to_value(item(201, 1, Availability::Repeatable, Some(99999))).unwrap())).await;
    assert_eq!(st, StatusCode::NOT_FOUND, "a gate to a missing achievement is 404");
}

#[tokio::test]
async fn unauthenticated_item_create_is_401() {
    let (state, _admin, _token, _dir) = keep();
    let (st, _) = send(&state, "POST", "/api/items", None, Some(to_value(item(1, 1, Availability::Repeatable, None)).unwrap())).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn once_item_shows_out_of_stock_after_redemption() {
    let (state, admin, token, _dir) = keep();
    let squire = add_squire(&state, admin);

    // A free `Once` item (cost 0 → affordable without funding).
    send(&state, "POST", "/api/items", Some(&token), Some(to_value(item(300, 0, Availability::Once, None)).unwrap())).await;
    // A `Repeatable` item, also redeemed, for contrast.
    send(&state, "POST", "/api/items", Some(&token), Some(to_value(item(301, 0, Availability::Repeatable, None)).unwrap())).await;

    // Direct redeem both (engine-direct), emitting ItemRedeemed.
    state.commit(None, Command::RedeemItem { command_id: CommandId(1), actor: UserId(admin), squire, item_id: ItemId(300) }).expect("redeem once");
    state.commit(None, Command::RedeemItem { command_id: CommandId(2), actor: UserId(admin), squire, item_id: ItemId(301) }).expect("redeem repeatable");

    let (_st, list) = send(&state, "GET", "/api/items", Some(&token), None).await;
    let once = list.as_array().unwrap().iter().find(|r| r["item"]["id"] == 300).unwrap();
    let repeatable = list.as_array().unwrap().iter().find(|r| r["item"]["id"] == 301).unwrap();
    assert_eq!(once["out_of_stock"], true, "a redeemed Once item is out of stock");
    assert!(once["last_redeemed"].is_i64(), "last_redeemed is set");
    assert_eq!(repeatable["out_of_stock"], false, "a Repeatable item never goes out of stock");
    assert!(repeatable["last_redeemed"].is_i64(), "repeatable last_redeemed is set");
}

#[tokio::test]
async fn archive_item_deactivates() {
    let (state, _admin, token, _dir) = keep();
    send(&state, "POST", "/api/items", Some(&token), Some(to_value(item(200, 3, Availability::Repeatable, None)).unwrap())).await;
    let (st, _) = send(&state, "POST", "/api/items/200/archive", Some(&token), None).await;
    assert_eq!(st, StatusCode::NO_CONTENT);
    let (_st, list) = send(&state, "GET", "/api/items", Some(&token), None).await;
    let row = list.as_array().unwrap().iter().find(|r| r["item"]["id"] == 200).unwrap();
    assert_eq!(row["item"]["active"], false);

    let (st, _) = send(&state, "POST", "/api/items/424242/archive", Some(&token), None).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
}

// ─── achievements ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_achievement_lists_and_audits() {
    let (state, admin, token, _dir) = keep();
    let (st, body) = send(&state, "POST", "/api/achievements", Some(&token), Some(to_value(points_achievement(400, 100)).unwrap())).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(body["id"], "400");

    let (_st, list) = send(&state, "GET", "/api/achievements", Some(&token), None).await;
    let row = list.as_array().unwrap().iter().find(|r| r["achievement"]["id"] == 400).unwrap();
    assert_eq!(row["audit"]["created_by"], admin.to_string());
}

#[tokio::test]
async fn invalid_achievement_is_400() {
    let (state, _admin, token, _dir) = keep();
    // PointsEarned { total: 0 } is an invalid definition → 400.
    let (st, _) = send(&state, "POST", "/api/achievements", Some(&token), Some(to_value(points_achievement(401, 0)).unwrap())).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
}

/// A category-scoped `TotalCompletions` achievement (SQUIRE-T-0071).
fn category_total_achievement(id: u128, category: &str, count: u32) -> Achievement {
    Achievement {
        id: AchievementId(id),
        name: "Cat".into(),
        description: None,
        criterion: Criterion::TotalCompletions { scope: Scope::Category(Category(category.into())), count },
        bonus_points: 10,
        active: true,
    }
}

/// An `Any`-scoped `Streak` achievement.
fn streak_achievement(id: u128, length: u32) -> Achievement {
    Achievement {
        id: AchievementId(id),
        name: "Streaky".into(),
        description: None,
        criterion: Criterion::Streak { scope: Scope::Any, length, basis: StreakBasis::CalendarDays },
        bonus_points: 10,
        active: true,
    }
}

// Streak + TotalCompletions + Category scope create + list (only PointsEarned was covered before).
#[tokio::test]
async fn streak_and_category_achievements_create_and_list() {
    let (state, _admin, token, _dir) = keep();
    let (st, _) = send(&state, "POST", "/api/achievements", Some(&token), Some(to_value(category_total_achievement(700, "Bedroom", 20)).unwrap())).await;
    assert_eq!(st, StatusCode::OK, "category-scoped total-completions is valid");
    let (st, _) = send(&state, "POST", "/api/achievements", Some(&token), Some(to_value(streak_achievement(701, 7)).unwrap())).await;
    assert_eq!(st, StatusCode::OK, "any-scoped streak is valid");

    let (_st, list) = send(&state, "GET", "/api/achievements", Some(&token), None).await;
    let ids: Vec<_> = list.as_array().unwrap().iter().map(|r| r["achievement"]["id"].clone()).collect();
    assert!(ids.contains(&serde_json::json!(700)) && ids.contains(&serde_json::json!(701)), "both new achievements are listed");
}

// A blank Category scope is rejected (SQUIRE-T-0071 decision); a zero-length streak too.
#[tokio::test]
async fn invalid_scoped_achievements_are_400() {
    let (state, _admin, token, _dir) = keep();
    let (st, _) = send(&state, "POST", "/api/achievements", Some(&token), Some(to_value(category_total_achievement(702, "   ", 5)).unwrap())).await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "a blank category scope is rejected");
    let (st, _) = send(&state, "POST", "/api/achievements", Some(&token), Some(to_value(streak_achievement(703, 0)).unwrap())).await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "a zero-length streak is rejected");
}

// Every entry in the shipped starter reward library imports cleanly and lists (SQUIRE-T-0073).
// Guards the rewards-library.json shape against the wire format the same way the UI imports it.
#[tokio::test]
async fn rewards_library_entries_all_import() {
    let (state, _admin, token, _dir) = keep();
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/rewards-library.json"))
        .expect("read rewards-library.json");
    let doc: Value = serde_json::from_str(&text).expect("parse rewards-library.json");
    let rewards = doc["rewards"].as_array().expect("rewards array");
    assert!(rewards.len() >= 8, "a useful starter set");

    for (i, r) in rewards.iter().enumerate() {
        let avail = match r["availability"].as_str().unwrap() {
            "Once" => Availability::Once,
            _ => Availability::Repeatable,
        };
        let mut wire = to_value(item(800 + i as u128, r["cost"].as_u64().unwrap() as u32, avail, None)).unwrap();
        wire["name"] = r["name"].clone();
        wire["description"] = r["description"].clone();
        let (st, _) = send(&state, "POST", "/api/items", Some(&token), Some(wire)).await;
        assert_eq!(st, StatusCode::OK, "library reward '{}' imports", r["name"]);
    }

    let (_st, list) = send(&state, "GET", "/api/items", Some(&token), None).await;
    assert_eq!(list.as_array().unwrap().len(), rewards.len(), "all library rewards are listed");
}

#[tokio::test]
async fn item_gated_on_real_achievement_is_accepted_then_archived() {
    let (state, _admin, token, _dir) = keep();
    // Define the achievement, then an item gated on it → 200 (valid gate).
    send(&state, "POST", "/api/achievements", Some(&token), Some(to_value(points_achievement(500, 50)).unwrap())).await;
    let (st, _) = send(&state, "POST", "/api/items", Some(&token), Some(to_value(item(600, 5, Availability::Repeatable, Some(500))).unwrap())).await;
    assert_eq!(st, StatusCode::OK, "an item gated on an existing achievement is valid");

    // Archive the achievement (never delete); missing → 404.
    let (st, _) = send(&state, "POST", "/api/achievements/500/archive", Some(&token), None).await;
    assert_eq!(st, StatusCode::NO_CONTENT);
    let (st, _) = send(&state, "POST", "/api/achievements/424242/archive", Some(&token), None).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
}
