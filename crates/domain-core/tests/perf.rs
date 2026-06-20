//! T-0007 projection performance check (NFR-1.1.2): a full projection sweep over a realistic
//! single-household history (a few thousand events) must complete well under the interactive
//! budget. We build the history deterministically by pushing `Change::Append` events directly
//! (bypassing the engine for speed — the engine's correctness is covered elsewhere), then time
//! one full `StateView`-shaped sweep across all squires, quests, and items.
//!
//! Threshold: < 100 ms (NFR-1.1.2). See the comment at the bottom on the hot-path optimisation.

use domain_core::testkit::*;
use domain_core::*;
use std::time::Instant;

const SQUIRES: [u128; 3] = [1, 2, 3];
const N_QUESTS: u128 = 20;
const N_DAYS: i32 = 40;

fn squire(id: u128) -> User {
    User { id: UserId(id), role: Role::Squire, display_name: format!("S{id}"), active: true }
}
fn knight(id: u128) -> User {
    User { id: UserId(id), role: Role::Knight, display_name: format!("K{id}"), active: true }
}

fn daily_quest(id: u128, reward: Points, category: Option<&str>) -> Quest {
    Quest {
        id: QuestId(id),
        title: format!("Q{id}"),
        description: None,
        category: category.map(|c| Category(c.into())),
        reward,
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
    RedeemableItem { id: ItemId(id), name: format!("I{id}"), description: None, cost, gate: None, availability: Availability::Repeatable, active: true, icon: None }
}

/// Build the history: 3 squires × ~20 quests/day × ~40 days of approved completions.
/// Each completion is a `CompletionClaimed` + `CompletionApproved` pair → ~4800 events.
fn build_history() -> InMemoryRepository {
    let mut r = InMemoryRepository::new();
    let mut seed: Vec<Change> = Vec::new();
    seed.push(Change::PutUser(knight(99)));
    for s in SQUIRES {
        seed.push(Change::PutUser(squire(s)));
    }
    for q in 1..=N_QUESTS {
        let cat = if q % 2 == 0 { Some("chores") } else { Some("school") };
        seed.push(Change::PutQuest(daily_quest(q, 5, cat)));
    }
    for i in 1..=5u128 {
        seed.push(Change::PutItem(item(i, 10)));
    }
    r.seed(&seed);

    // Approved completions. Monotone claim ids and timestamps.
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
                    actor: Some(UserId(99)),
                    points: 5,
                    at: Timestamp(t + 1),
                }));
                claim_id += 1;
                t += 2;
            }
        }
    }
    r.seed(&events);
    r
}

#[test]
fn projection_sweep_under_budget() {
    let repo = build_history();
    let snap = repo.snapshot();
    let n_events = snap.events.len();
    assert!(n_events >= 4000, "expected a few thousand events, got {n_events}");

    let asof = Date(N_DAYS - 1);
    let scopes: Vec<(String, Scope, StreakBasis)> = vec![
        ("chores".into(), Scope::Category(Category("chores".into())), StreakBasis::CalendarDays),
        ("any".into(), Scope::Any, StreakBasis::CalendarDays),
    ];

    let start = Instant::now();
    let mut sink = 0i64; // prevent the optimiser dropping the work
    for s in SQUIRES {
        let sid = UserId(s);
        // Per-squire balance.
        sink += Proj::balance(&snap, sid);
        // Quests due today.
        sink += Proj::quests_due(&snap, sid, asof).len() as i64;
        // A couple of streak scopes (quest-scoped + the category/any scopes).
        sink += Proj::current_streak(&snap, sid, &Scope::Quest(QuestId(1)), StreakBasis::ScheduledOccurrences, asof) as i64;
        for (_, scope, basis) in &scopes {
            sink += Proj::current_streak(&snap, sid, scope, *basis, asof) as i64;
        }
        // Approximate StateView assembly: a quest_status card per quest and a reward_view per item.
        for q in &snap.quests {
            sink += match quest_status(&snap, sid, q, asof) {
                QuestStatus::CompletedToday => 1,
                _ => 0,
            };
        }
        for it in &snap.items {
            let (affordable, _lock, _last) = reward_view(&snap, sid, it);
            sink += affordable as i64;
        }
    }
    let elapsed = start.elapsed();
    // Keep `sink` observable so the loop isn't elided.
    assert!(sink >= 0 || sink < 0);
    println!(
        "perf sweep: {n_events} events, {} squires, {} quests → {:?}",
        SQUIRES.len(),
        snap.quests.len(),
        elapsed
    );

    // NFR-1.1.2 is a production (release) budget. After the hot-path optimisation (a one-pass
    // `ClaimIndex` built once per projection call, replacing the per-event `claim_meta` re-scans
    // that made `quests_due` / streak walks / `quest_status` O(n²)), a release build does this
    // ~4800-event sweep in ~13 ms — comfortably under the 100 ms target. Debug builds run several
    // times slower (no inlining, overflow checks), so the strict 100 ms assertion is enforced
    // only when optimisations are on; debug gets a generous bound that still catches an algorithmic
    // regression (pre-optimisation this sweep took ~2 s in debug).
    #[cfg(not(debug_assertions))]
    assert!(
        elapsed.as_millis() < 100,
        "projection sweep took {:?} (>100 ms NFR-1.1.2 budget) over {n_events} events",
        elapsed
    );
    #[cfg(debug_assertions)]
    assert!(
        elapsed.as_millis() < 1000,
        "projection sweep took {:?} in debug (>1 s) over {n_events} events — likely an O(n²) regression",
        elapsed
    );
}
