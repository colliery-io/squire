//! `serve_demo` — a runnable LAN api server that seeds a **deterministic** demo household and
//! serves the real Squire/Knight/control surfaces (SQUIRE-T-0035).
//!
//! This is demo/bootstrap convenience, NOT production wiring: it recreates a fixed SQLite dir on
//! every run for a clean, reproducible seed, registers household `demo` (admin Knight `UserId(1)`,
//! secret `demo`), adds a Squire (`UserId(2)`, secret `demo`), and authors a couple of quests +
//! a reward directly into the store (authoring is deliberately NOT on the wire). It then serves on
//! `0.0.0.0:PORT` so an emulator can `login` → token → `GET /state` end-to-end.
//!
//! Run: `cargo run -p api --bin serve_demo` (override the port with `DEMO_PORT`).

use std::net::SocketAddr;

use api::AppState;
use domain_core::contract::{
    Achievement, AchievementId, AddMemberReq, Assignment, Availability, Cadence, Change, Completion,
    Criterion, HouseholdHandle, ItemId, Quest, QuestId, RedeemableItem, RegisterHouseholdReq,
    Repository, Role, Schedule, Scope, StreakBasis, UserId,
};
use identity::{Principal, TokenSigner};
use store::tenant::Backend;

/// Fixed on-disk location for the demo tenant. Recreated each run so the seed is deterministic.
const DEMO_DIR: &str = "/tmp/squire-demo-api";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Recreate the demo dir for a clean, deterministic seed every run (ignore "not found").
    let _ = std::fs::remove_dir_all(DEMO_DIR);

    let port: u16 = std::env::var("DEMO_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let handle = HouseholdHandle("demo".into());

    // ── Wire a production AppState over a fixed-dir SQLite tenant ───────────────────────────────
    let state = AppState::local_prod(
        Backend::Sqlite { dir: DEMO_DIR.into() },
        handle.clone(),
        TokenSigner::new(b"squire-demo-signing-key"),
        24 * 60 * 60 * 1000, // 24h token TTL
    )?;

    // ── Seed members via the SAME public control-plane APIs the tests use ───────────────────────
    // Register the household + first Knight → admin is UserId(1).
    state
        .identity
        .register(RegisterHouseholdReq {
            household_name: "demo".into(),
            admin_name: "Arthur".into(),
            admin_secret: "demo".into(),
        })
        .expect("register demo household + admin Knight");

    // The Knight principal authorizes adding the Squire (UserId(2)).
    let knight = Principal {
        household: handle.clone(),
        user: UserId(1),
        role: Role::Knight,
    };
    state
        .identity
        .add_member(
            &knight,
            AddMemberReq {
                role: Role::Squire,
                display_name: "Gawain".into(),
                initial_secret: "demo".into(),
            },
        )
        .expect("add Squire (UserId 2)");

    // ── Author quests + a reward by DIRECT store apply (authoring is NOT on the wire) ────────────
    // One auto_approve quest (a claim immediately credits points → the child sees a balance),
    // one manual-review quest, and one repeatable reward at a modest cost.
    let auto_quest = Quest {
        id: QuestId(100),
        title: "Make your bed".into(),
        description: Some("Tidy your bed each morning.".into()),
        category: None,
        reward: 5,
        cadence: Cadence::Recurring(Schedule::Daily),
        assignment: Assignment::AllSquires,
        completion: Completion::EachAssignee,
        auto_approve: true, // → credited immediately on claim
        repeatable_within_day: false,
        active: true,
        icon: None,
    };
    let review_quest = Quest {
        id: QuestId(101),
        title: "Tidy your room".into(),
        description: Some("Clean up and put everything away.".into()),
        category: None,
        reward: 10,
        cadence: Cadence::Recurring(Schedule::Daily),
        assignment: Assignment::AllSquires,
        completion: Completion::EachAssignee,
        auto_approve: false, // → Knight review flow
        repeatable_within_day: false,
        active: true,
        icon: None,
    };
    let reward = RedeemableItem {
        id: ItemId(200),
        name: "Ice cream".into(),
        description: Some("One scoop of your choice.".into()),
        cost: 3,
        gate: None,
        availability: Availability::Repeatable,
        active: true,
        icon: None,
    };
    // A STREAK achievement: 3 scheduled days in a row of "Tidy your room" (quest 101) → +50 bonus
    // points on unlock. Achievements aren't "points for a total" — they can be streaks that award
    // currency AND gate rewards (operator feedback / SQUIRE-T-0036).
    let streak_achievement = Achievement {
        id: AchievementId(300),
        name: "Room Master".into(),
        description: Some("Tidy your room 3 scheduled days in a row.".into()),
        criterion: Criterion::Streak {
            scope: Scope::Quest(QuestId(101)),
            length: 3,
            basis: StreakBasis::ScheduledOccurrences,
        },
        bonus_points: 50,
        active: true,
    };
    // A GATED reward: locked (NeedsAchievement) until the "Room Master" streak is earned.
    let gated_reward = RedeemableItem {
        id: ItemId(201),
        name: "Movie night".into(),
        description: Some("Unlocked by the Room Master streak.".into()),
        cost: 15,
        gate: Some(AchievementId(300)),
        availability: Availability::Repeatable,
        active: true,
        icon: None,
    };
    state
        .store
        .lock()
        .unwrap()
        .apply(
            Some(UserId(1)),
            &[
                Change::PutQuest(auto_quest),
                Change::PutQuest(review_quest),
                Change::PutAchievement(streak_achievement),
                Change::PutItem(reward),
                Change::PutItem(gated_reward),
            ],
        )
        .expect("seed quests + achievement + rewards");

    // ── Banner ──────────────────────────────────────────────────────────────────────────────────
    println!("════════════════════════════════════════════════════════════════════");
    println!("  Squire demo api server (SQUIRE-T-0035)");
    println!("  Base URL:   http://0.0.0.0:{port}");
    println!("  Emulator:   the Android emulator reaches the host at http://10.0.2.2:{port}");
    println!("  Household:  demo");
    println!("  Squire login: (household=demo, user=2, secret=demo)");
    println!("  Knight login: (household=demo, user=1, secret=demo)");
    println!("  Seeded: 2 quests, 'Ice cream' (3pts), 'Movie night' (gated on the");
    println!("          'Room Master' streak: 3 days of 'Tidy your room' → +50pts).");
    println!("════════════════════════════════════════════════════════════════════");

    // ── Serve ───────────────────────────────────────────────────────────────────────────────────
    api::serve(state, SocketAddr::from(([0, 0, 0, 0], port))).await
}
