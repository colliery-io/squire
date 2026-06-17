//! `squire-home` — the **home server**: one process that hosts BOTH the Keep (the loopback admin
//! UI, ADR SQUIRE-A-0008) AND the LAN api (the phone clients, SQUIRE-S-0003) over **one shared,
//! single-writer store + identity**, for the same household.
//!
//! This is the coherent topology the product actually deploys: the parent's Keep and the child's
//! phone talk to the *same* data — what the child submits on the phone shows up in the parent's
//! Keep review queue. (The earlier `serve_demo` only ran the api, on a separate household from a
//! standalone Keep, so the two looked disconnected.)
//!
//! Demo seed is deterministic (a fixed SQLite dir recreated each run): household `demo`, admin
//! Knight `UserId(1)`/`demo`, Squire `UserId(2)`/`demo`, a few quests + a streak achievement + a
//! gated reward. Run: `cargo run -p squire-home` (ports via `API_PORT` / `KEEP_PORT`).

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use api::AppState;
use keep::KeepState;

use domain_core::contract::{
    Achievement, AchievementId, AddMemberReq, Assignment, Availability, Cadence, Change, Completion,
    Criterion, HouseholdHandle, ItemId, Quest, QuestId, RedeemableItem, RegisterHouseholdReq,
    Repository, Role, Schedule, Scope, StreakBasis, UserId,
};
use identity::{Identity, Principal, ProdIdentity, SharedStore, TokenSigner};
use store::tenant::{Backend, Provisioner};
use store::SystemClock;

/// Fixed on-disk location for the demo tenant, recreated each run for a deterministic seed.
const DIR: &str = "/tmp/squire-home";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _ = std::fs::remove_dir_all(DIR);

    let api_port: u16 = env_u16("API_PORT", 8080);
    let keep_port: u16 = env_u16("KEEP_PORT", 4920);
    let handle = HouseholdHandle("demo".into());

    // ── ONE shared store + identity (single writer across BOTH surfaces) ─────────────────────────
    let provisioner = Provisioner::new(Backend::Sqlite { dir: DIR.into() });
    provisioner.provision(&handle.0)?;
    let store: SharedStore = Arc::new(Mutex::new(provisioner.open(&handle.0, SystemClock)?));
    let identity: Arc<dyn Identity> = Arc::new(ProdIdentity::shared_local(
        store.clone(),
        TokenSigner::new(b"squire-home-signing-key"),
        handle.clone(),
        24 * 60 * 60 * 1000, // 24h tokens
    ));

    // ── Seed via the same public APIs the tests use ──────────────────────────────────────────────
    identity
        .register(RegisterHouseholdReq {
            household_name: "demo".into(),
            admin_name: "Arthur".into(),
            admin_secret: "demo".into(),
        })
        .expect("register demo household + admin Knight (UserId 1)");
    let knight = Principal { household: handle.clone(), user: UserId(1), role: Role::Knight };
    identity
        .add_member(
            &knight,
            AddMemberReq { role: Role::Squire, display_name: "Gawain".into(), initial_secret: "demo".into() },
        )
        .expect("add Squire (UserId 2)");

    // Author quests + a streak achievement + rewards by DIRECT store apply (authoring is off the wire).
    let daily = |id, title: &str, reward, auto| Quest {
        id: QuestId(id),
        title: title.into(),
        description: None,
        category: None,
        reward,
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
    store
        .lock()
        .unwrap()
        .apply(
            Some(UserId(1)),
            &[
                Change::PutQuest(daily(100, "Make your bed", 5, true)),
                Change::PutQuest(daily(101, "Tidy your room", 10, false)),
                Change::PutAchievement(streak),
                Change::PutItem(ice_cream),
                Change::PutItem(movie_night),
            ],
        )
        .expect("seed quests + achievement + rewards");

    // ── Two surfaces over the ONE store/identity ─────────────────────────────────────────────────
    let app = AppState::new(store.clone(), identity.clone()); // LAN api (phones)
    let keep_state = KeepState::from_parts(store.clone(), identity.clone(), handle.clone()); // loopback admin

    println!("════════════════════════════════════════════════════════════════════");
    println!("  Squire HOME server — one household, two surfaces, one shared store");
    println!("  Keep (parent, loopback): http://127.0.0.1:{keep_port}   login: Knight 1 / demo");
    println!("  LAN api (phones):        http://0.0.0.0:{api_port}   (emulator: http://10.0.2.2:{api_port})");
    println!("  Squire (child) login:    household=demo, user=2, secret=demo");
    println!("  Same data: a phone claim shows up in the Keep's Review queue.");
    println!("════════════════════════════════════════════════════════════════════");

    // Serve both concurrently; if either listener fails, the process exits.
    tokio::try_join!(
        api::serve(app, SocketAddr::from(([0, 0, 0, 0], api_port))),
        keep::serve(keep_state, keep_port),
    )?;
    Ok(())
}

fn env_u16(key: &str, default: u16) -> u16 {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}
