//! `squire-home` — the **demo** home server: a throwaway harness that wipes a fixed `/tmp` dir and
//! re-seeds a deterministic "demo" household every launch (admin Knight `UserId(1)`/`demo`, Squire
//! `UserId(2)`/`demo`, sample quests + a streak achievement + a gated reward). For the emulator
//! one-tap flow and manual demos. Run: `cargo run -p squire-home` (ports via `API_PORT`/`KEEP_PORT`).
//!
//! The **persistent** production server is the sibling binary `squire-serve` (durable data dir,
//! register-or-load, stable signing key — SQUIRE-T-0048). Shared wiring lives in this crate's `lib.rs`.

use std::path::Path;

use domain_core::contract::{
    Achievement, AchievementId, AddMemberReq, Assignment, Availability, Cadence, Change, ClaimId,
    Clock, Completion, Criterion, Event, HouseholdHandle, ItemId, Quest, QuestId, RedeemableItem,
    RegisterHouseholdReq, Repository, Role, Schedule, Scope, StreakBasis, UserId,
};
use identity::Principal;

use squire_home::{ensure_timezone, env_u16, init_tracing, open_household, serve, TOKEN_TTL_MS};

/// Fixed on-disk location for the demo tenant, recreated each run for a deterministic seed.
const DIR: &str = "/tmp/squire-home";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Structured logging (SQUIRE-T-0127): subscriber up front so the api access log renders.
    init_tracing();
    // DEMO: wipe so every run starts from the same deterministic seed.
    let _ = std::fs::remove_dir_all(DIR);

    let api_port = env_u16("API_PORT", 8080);
    let keep_port = env_u16("KEEP_PORT", 4920);
    let handle = HouseholdHandle("demo".into());

    // One shared store + identity (a fixed demo signing key — fine for a throwaway tenant).
    let (store, identity) = open_household(
        Path::new(DIR),
        &handle,
        b"squire-home-signing-key",
        TOKEN_TTL_MS,
    )?;

    // ── Seed the demo household via the same public APIs the tests use ────────────────────────────
    identity
        .register(RegisterHouseholdReq {
            household_name: "demo".into(),
            admin_name: "Arthur".into(),
            admin_secret: "demo".into(),
        })
        .expect("register demo household + admin Knight (UserId 1)");
    let knight = Principal {
        household: handle.clone(),
        user: UserId(1),
        role: Role::Knight,
    };
    identity
        .add_member(
            &knight,
            AddMemberReq {
                role: Role::Squire,
                display_name: "Gawain".into(),
                initial_secret: "demo".into(),
            },
        )
        .expect("add Squire (UserId 2)");

    // Author quests + a streak achievement + rewards by DIRECT store apply (authoring is off the wire).
    let daily = |id, title: &str, reward, auto| Quest {
        id: QuestId(id),
        title: title.into(),
        description: None,
        category: None,
        reward,
        cash: 0,
        cadence: Cadence::Recurring(Schedule::Daily),
        assignment: Assignment::AllSquires,
        completion: Completion::EachAssignee,
        auto_approve: auto,
        repeatable_within_day: false,
        active: true,
        icon: None,
    };
    let streak = Achievement {
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
    let ice_cream = RedeemableItem {
        id: ItemId(200),
        name: "Ice cream".into(),
        description: None,
        cost: 3,
        gate: None,
        availability: Availability::Repeatable,
        active: true,
        icon: None,
    };
    let movie_night = RedeemableItem {
        id: ItemId(201),
        name: "Movie night".into(),
        description: Some("Unlocked by the Room Master streak.".into()),
        cost: 15,
        gate: Some(AchievementId(300)),
        availability: Availability::Repeatable,
        active: true,
        icon: None,
    };
    // A seeded pending claim so the Review tab always has something to triage in the demo
    // (Gawain claimed "Tidy your room" today, awaiting the Knight's seal) — SQUIRE-T-0080.
    let (today, now) = {
        let g = store.lock().unwrap();
        (g.clock().today(), g.clock().now())
    };
    store
        .lock()
        .unwrap()
        .apply(
            Some(UserId(1)),
            &[
                Change::PutQuest(daily(100, "Make your bed", 5, true)),
                Change::PutQuest(daily(101, "Tidy your room", 10, false)),
                Change::PutQuest(daily(102, "Walk the dog", 8, false)),
                Change::PutAchievement(streak),
                Change::PutItem(ice_cream),
                Change::PutItem(movie_night),
                // Two pending claims so the Review tab demo shows triage (and the E2E can reject one
                // and approve another independently) — SQUIRE-T-0080.
                Change::Append(Event::CompletionClaimed {
                    claim_id: ClaimId(9001),
                    squire: UserId(2),
                    quest_id: QuestId(101),
                    on: today,
                    at: now,
                }),
                Change::Append(Event::CompletionClaimed {
                    claim_id: ClaimId(9002),
                    squire: UserId(2),
                    quest_id: QuestId(102),
                    on: today,
                    at: now,
                }),
            ],
        )
        .expect("seed quests + achievement + rewards + a pending claim");

    // Onboarding (ADR A-0011): seed the household timezone (SQUIRE_TZ or the detected host zone).
    let timezone = ensure_timezone(&store);

    println!("════════════════════════════════════════════════════════════════════");
    println!("  Squire DEMO server (squire-home) — wiped + re-seeded each run");
    println!("  Timezone:                {timezone}");
    println!("  Keep (parent, loopback): http://127.0.0.1:{keep_port}   login: Knight 1 / demo");
    println!("  LAN api (phones):        http://0.0.0.0:{api_port}   (emulator: http://10.0.2.2:{api_port})");
    println!("  Squire (child) login:    household=demo, user=2, secret=demo");
    println!("  Persistent server:       cargo run -p squire-home --bin squire-serve");
    println!("════════════════════════════════════════════════════════════════════");

    serve(store, identity, handle, api_port, keep_port).await
}
