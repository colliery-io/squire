//! SQUIRE-T-0013 perf gate (NFR-2.2 / NFR-8): a household-scale store must rehydrate and run a
//! full projection sweep well within the interactive budget.
//!
//! We seed a realistic single-household store — ~3 squires, ~20 quests, a few thousand events —
//! by applying many auto-approve-shaped completions directly (a `CompletionClaimed` +
//! `CompletionApproved` pair per completion, bulk-inserted via one `apply` for speed; the
//! engine's correctness is covered by the conformance/domain-core suites). Then we time ONE
//! `store.snapshot()` (the full Diesel read + decode) plus a full projection sweep across every
//! squire (balance + quests_due + a couple of streaks + can_redeem over the items).
//!
//! Threshold (mirrors domain-core's `tests/perf.rs`): strictly < 100 ms under `--release`
//! (NFR-2.2 / NFR-8 production budget), and a generous < 1.5 s in debug that still catches an
//! algorithmic regression. Runs on the SQLite backend (the default `cargo test -p store`); the
//! perf gate is portable and not tied to Postgres.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use domain_core::contract::{
    Assignment, Availability, Cadence, Category, Change, ClaimId, Completion, Date, Event, ItemId,
    Points, Projections, Quest, QuestId, RedeemableItem, Repository, Role, Schedule, Scope,
    StreakBasis, Timestamp, User, UserId,
};
use domain_core::Proj;

use store::{AnyConnection, FixedClock, Store};

const SQUIRES: [u128; 3] = [1, 2, 3];
const N_QUESTS: u128 = 20;
const N_DAYS: i32 = 40;
const KNIGHT: u128 = 99;

fn user(id: u128, role: Role) -> User {
    User { id: UserId(id), role, display_name: format!("U{id}"), active: true }
}

fn daily_quest(id: u128, category: Option<&str>) -> Quest {
    Quest {
        id: QuestId(id),
        title: format!("Q{id}"),
        description: None,
        category: category.map(|c| Category(c.into())),
        reward: 5 as Points,
        cash: 0,
        cadence: Cadence::Recurring(Schedule::Daily),
        assignment: Assignment::AllSquires,
        completion: Completion::EachAssignee,
        auto_approve: false,
        repeatable_within_day: false,
        active: true,
        icon: None,
    }
}

fn item(id: u128, cost: Points) -> RedeemableItem {
    RedeemableItem {
        id: ItemId(id),
        name: format!("I{id}"),
        description: None,
        cost,
        gate: None,
        availability: Availability::Repeatable,
        active: true,
        icon: None,
    }
}

/// Seed the store: defs/users, then 3 squires × ~20 quests/day × ~40 days of approved
/// completions (a claim+approve pair each → ~4800 events), bulk-applied.
fn seed(store: &mut Store<FixedClock>) {
    let mut defs: Vec<Change> = Vec::new();
    defs.push(Change::PutUser(user(KNIGHT, Role::Knight)));
    for s in SQUIRES {
        defs.push(Change::PutUser(user(s, Role::Squire)));
    }
    for q in 1..=N_QUESTS {
        let cat = if q % 2 == 0 { Some("chores") } else { Some("school") };
        defs.push(Change::PutQuest(daily_quest(q, cat)));
    }
    for i in 1..=5u128 {
        defs.push(Change::PutItem(item(i, 10)));
    }
    store.apply(None, &defs).expect("apply defs");

    let mut events: Vec<Change> = Vec::new();
    let mut claim_id: u128 = 1;
    let mut t: i64 = 0;
    for day in 0..N_DAYS {
        for s in SQUIRES {
            for q in 1..=N_QUESTS {
                events.push(Change::Append(Event::CompletionClaimed {
                    claim_id: ClaimId(claim_id),
                    squire: UserId(s),
                    quest_id: QuestId(q),
                    on: Date(day),
                    at: Timestamp(t),
                }));
                events.push(Change::Append(Event::CompletionApproved {
                    claim_id: ClaimId(claim_id),
                    squire: UserId(s),
                    actor: Some(UserId(KNIGHT)),
                    points: 5 as Points,
                    at: Timestamp(t + 1),
                }));
                claim_id += 1;
                t += 2;
            }
        }
    }
    store.apply(None, &events).expect("apply events");
}

#[test]
fn snapshot_and_projection_sweep_under_budget() {
    // SQLite temp-file store (the default backend; the perf gate is backend-portable).
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("perf.sqlite");
    let url = path.to_str().expect("utf-8 path");
    let mut conn = SqliteConnection::establish(url).expect("sqlite");
    store::run_migrations(&mut conn).expect("migrations");
    let mut store = Store::new(AnyConnection::Sqlite(conn), FixedClock::at(Timestamp(0)));

    seed(&mut store);

    let asof = Date(N_DAYS - 1);
    let scopes: [(Scope, StreakBasis); 2] = [
        (Scope::Category(Category("chores".into())), StreakBasis::CalendarDays),
        (Scope::Any, StreakBasis::CalendarDays),
    ];

    // Time the WHOLE thing: snapshot (Diesel read + decode) + a full projection sweep.
    let start = std::time::Instant::now();
    let snap = store.snapshot();
    let n_events = snap.events.len();
    let mut sink = 0i64; // keep the work observable so the optimiser can't elide it
    for s in SQUIRES {
        let sid = UserId(s);
        sink += Proj::balance(&snap, sid);
        sink += Proj::quests_due(&snap, sid, asof).len() as i64;
        sink += Proj::current_streak(&snap, sid, &Scope::Quest(QuestId(1)), StreakBasis::ScheduledOccurrences, asof) as i64;
        for (scope, basis) in &scopes {
            sink += Proj::current_streak(&snap, sid, scope, *basis, asof) as i64;
        }
        for it in &snap.items {
            sink += Proj::can_redeem(&snap, sid, it.id, asof).is_ok() as i64;
        }
    }
    let elapsed = start.elapsed();
    std::hint::black_box(sink); // keep the work observable so the optimiser can't elide it

    assert!(n_events >= 4000, "expected a few thousand events, got {n_events}");
    println!(
        "store perf sweep: snapshot+projections over {n_events} events, {} squires, {} quests → {:?}",
        SQUIRES.len(),
        snap.quests.len(),
        elapsed
    );

    // NFR-2.2 / NFR-8 is a production (release) budget. Enforce the strict 100 ms bound only with
    // optimisations on; debug builds run several times slower (no inlining, overflow checks) so
    // they get a generous bound that still catches an algorithmic regression.
    #[cfg(not(debug_assertions))]
    assert!(
        elapsed.as_millis() < 100,
        "snapshot+sweep took {:?} (>100 ms NFR-2.2/NFR-8 budget) over {n_events} events",
        elapsed
    );
    #[cfg(debug_assertions)]
    assert!(
        elapsed.as_millis() < 1500,
        "snapshot+sweep took {:?} in debug (>1.5 s) over {n_events} events — likely a regression",
        elapsed
    );
}
