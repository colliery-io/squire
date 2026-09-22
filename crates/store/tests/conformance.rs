//! SQUIRE-T-0013 — conformance / parity proof for the Diesel-backed `store`.
//!
//! The whole point: the real Diesel [`Store`] is a *faithful drop-in* for the pure-core
//! [`InMemoryRepository`]. We drive a representative end-to-end household scenario THROUGH the
//! real [`DomainEngine`] against BOTH repositories — the SAME command sequence, the SAME
//! [`FixedClock`] — and after every step assert:
//!   * `store.snapshot()` debug-equals `inmem.snapshot()` (same users / quests / items /
//!     achievements / events, in the same order), and
//!   * the five projections (`balance`, `quests_due`, `current_streak`, `is_unlocked`,
//!     `can_redeem`) computed over the rehydrated `store` snapshot equal those over the
//!     in-memory truth, for every relevant Squire.
//!
//! This proves the store persists exactly the `Change`s the engine emits and that projections
//! over the reloaded snapshot match the in-memory reference. Runs on BOTH backends: SQLite
//! always; Postgres under `--features postgres` + `DATABASE_URL` (reset via
//! `store::pg::provision_clean`, serialized behind a `Mutex`).
//!
//! Also exercises the raw per-quest / per-item event-log queries (NFR-11).

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use domain_core::contract::{
    Achievement, AchievementId, Assignment, Availability, Cadence, Category, ClaimId, Command,
    CommandId, Completion, Criterion, Currency, Date, Decision, Engine, Event, ItemId, Points,
    Projections, Quest, QuestId, RedeemableItem, Repository, RequestId, Role, Schedule, Scope,
    Snapshot, StreakBasis, Timestamp, User, UserId,
};
use domain_core::testkit::InMemoryRepository;
use domain_core::{DomainEngine, Proj};

use store::{AnyConnection, FixedClock, Store};

// ─── backend harness ───────────────────────────────────────────────────────────

/// Run `test` against each available backend with a freshly-migrated, empty `AnyConnection`.
/// SQLite (temp file) always; Postgres only under `--features postgres` + `DATABASE_URL`.
fn each_backend(test: impl Fn(AnyConnection)) {
    // SQLite (temp file) — always.
    {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("conformance.sqlite");
        let url = path.to_str().expect("utf-8 path");
        let mut sqlite = SqliteConnection::establish(url).expect("sqlite");
        store::run_migrations(&mut sqlite).expect("sqlite migrations");
        test(AnyConnection::Sqlite(sqlite));
    }

    // Postgres — opt-in, serialized (one shared `public` schema).
    #[cfg(feature = "postgres")]
    {
        use std::sync::Mutex;
        static PG_LOCK: Mutex<()> = Mutex::new(());

        if let Ok(url) = std::env::var("DATABASE_URL") {
            let _guard = PG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            store::pg::provision_clean(&url).expect("reset + migrate postgres");
            let conn =
                AnyConnection::Pg(diesel::pg::PgConnection::establish(&url).expect("pg connect"));
            test(conn);
        } else {
            eprintln!("skipping postgres conformance test: DATABASE_URL not set");
        }
    }
}

// ─── the clock both repos share ──────────────────────────────────────────────────

/// One fixed clock for both repos so audit timestamps and any clock-driven logic match. The
/// scenario only spans a handful of "days"; `today` is Date(2) (the last day we author/claim on)
/// and `now` is an arbitrary fixed instant.
fn the_clock() -> FixedClock {
    FixedClock::new(Date(2), Timestamp(1_700_000_000_000))
}

// ─── fixtures ────────────────────────────────────────────────────────────────────

const KNIGHT: u128 = 1;
const ALICE: u128 = 2;
const BORIS: u128 = 3;

const QUEST_DAILY: u128 = 10; // EachAssignee, manual-approve, daily — drives the streak
const QUEST_RACE: u128 = 11; // Race — two squires, one wins
const ITEM_GATED: u128 = 20; // gated by the streak achievement
const ACH_STREAK: u128 = 30; // 2-day scheduled streak on QUEST_DAILY → unlock + bonus + gate lift

fn user(id: u128, role: Role) -> User {
    User {
        id: UserId(id),
        role,
        display_name: format!("U{id}"),
        active: true,
    }
}

fn daily_quest() -> Quest {
    Quest {
        id: QuestId(QUEST_DAILY),
        title: "Sweep".into(),
        description: Some("daily".into()),
        category: Some(Category("chores".into())),
        reward: 10 as Points,
        cash: 0,
        cadence: Cadence::Recurring(Schedule::Daily),
        assignment: Assignment::AllSquires,
        completion: Completion::EachAssignee,
        auto_approve: false,
        repeatable_within_day: false,
        active: true,
        icon: None,
        due_time: None,
    }
}

fn race_quest() -> Quest {
    Quest {
        id: QuestId(QUEST_RACE),
        title: "First to the well".into(),
        description: None,
        category: None,
        reward: 20 as Points,
        cash: 0,
        cadence: Cadence::Recurring(Schedule::Daily),
        assignment: Assignment::AllSquires,
        completion: Completion::Race,
        auto_approve: false,
        repeatable_within_day: false,
        active: true,
        icon: None,
        due_time: None,
    }
}

fn streak_achievement() -> Achievement {
    Achievement {
        id: AchievementId(ACH_STREAK),
        name: "Two in a row".into(),
        description: None,
        criterion: Criterion::Streak {
            scope: Scope::Quest(QuestId(QUEST_DAILY)),
            length: 2,
            basis: StreakBasis::ScheduledOccurrences,
        },
        bonus_points: 25 as Points,
        active: true,
    }
}

fn gated_item() -> RedeemableItem {
    RedeemableItem {
        id: ItemId(ITEM_GATED),
        name: "Ice cream".into(),
        description: None,
        cost: 15 as Points,
        gate: Some(AchievementId(ACH_STREAK)),
        availability: Availability::Repeatable,
        active: true,
        icon: None,
    }
}

/// The representative end-to-end command sequence, in order. Each tuple is `(by, Command)`:
/// `by` is the user the change is applied under (audit stamping). Authoring + review run as the
/// Knight; squire commands run as the squire (the API fills `squire` from the token, and the
/// single writer stamps `by`).
fn scenario() -> Vec<(Option<UserId>, Command)> {
    let k = Some(UserId(KNIGHT));
    let a = Some(UserId(ALICE));
    let b = Some(UserId(BORIS));
    vec![
        // ── authoring (through the engine) ──
        (k, Command::DefineQuest(daily_quest())),
        (k, Command::DefineQuest(race_quest())),
        (k, Command::DefineAchievement(streak_achievement())),
        (k, Command::DefineItem(gated_item())),
        // ── claim → approve → balance (Alice, daily quest, day 0) ──
        (
            a,
            Command::SubmitClaim {
                claim_id: ClaimId(100),
                squire: UserId(ALICE),
                quest_id: QuestId(QUEST_DAILY),
                on: Date(0),
            },
        ),
        (
            k,
            Command::ReviewClaim {
                actor: UserId(KNIGHT),
                claim_id: ClaimId(100),
                decision: Decision::Approve,
            },
        ),
        // ── second scheduled day → streak length 2 → AchievementUnlocked (+bonus + gate lift) ──
        (
            a,
            Command::SubmitClaim {
                claim_id: ClaimId(101),
                squire: UserId(ALICE),
                quest_id: QuestId(QUEST_DAILY),
                on: Date(1),
            },
        ),
        (
            k,
            Command::ReviewClaim {
                actor: UserId(KNIGHT),
                claim_id: ClaimId(101),
                decision: Decision::Approve,
            },
        ),
        // ── an AdjustPoints (Knight tops Alice up) ──
        (
            k,
            Command::AdjustPoints {
                command_id: CommandId(300),
                actor: UserId(KNIGHT),
                squire: UserId(ALICE),
                currency: Currency::Coins,
                amount: 5,
                reason: "bonus chores".into(),
            },
        ),
        // ── redemption: request → approve (Alice redeems the now-unlocked gated item) ──
        (
            a,
            Command::RequestRedemption {
                request_id: RequestId(400),
                squire: UserId(ALICE),
                item_id: ItemId(ITEM_GATED),
            },
        ),
        (
            k,
            Command::ReviewRedemption {
                actor: UserId(KNIGHT),
                request_id: RequestId(400),
                decision: Decision::Approve,
            },
        ),
        // ── a Race quest: both squires claim on day 2, Alice wins ──
        (
            a,
            Command::SubmitClaim {
                claim_id: ClaimId(500),
                squire: UserId(ALICE),
                quest_id: QuestId(QUEST_RACE),
                on: Date(2),
            },
        ),
        (
            b,
            Command::SubmitClaim {
                claim_id: ClaimId(501),
                squire: UserId(BORIS),
                quest_id: QuestId(QUEST_RACE),
                on: Date(2),
            },
        ),
        (
            k,
            Command::ReviewClaim {
                actor: UserId(KNIGHT),
                claim_id: ClaimId(500),
                decision: Decision::Approve,
            },
        ), // Alice wins
    ]
}

// ─── equality helpers ────────────────────────────────────────────────────────────

/// Whole-snapshot equality as one debug string (events already ordered by seq, defs by id).
/// We sort the definition vecs by id so the two repos' iteration orders don't matter — only the
/// content does. Events are compared in `seq` order (the store) vs append order (in-memory),
/// which must agree.
fn snap_dbg(s: &Snapshot) -> String {
    let mut users = s.users.clone();
    users.sort_by_key(|u| u.id.0);
    let mut quests = s.quests.clone();
    quests.sort_by_key(|q| q.id.0);
    let mut items = s.items.clone();
    items.sort_by_key(|i| i.id.0);
    let mut achievements = s.achievements.clone();
    achievements.sort_by_key(|a| a.id.0);
    format!(
        "users={:?}\nquests={:?}\nitems={:?}\nachievements={:?}\nevents={:?}",
        users, quests, items, achievements, s.events
    )
}

/// Assert the five projections agree across the two snapshots for `squire`. `today` drives
/// `quests_due` / `current_streak`; we probe a couple of streak scopes and `can_redeem` over
/// every item in the snapshot.
fn assert_projections_agree(
    store_snap: &Snapshot,
    inmem_snap: &Snapshot,
    squire: UserId,
    today: Date,
) {
    assert_eq!(
        Proj::balance(store_snap, squire),
        Proj::balance(inmem_snap, squire),
        "balance disagrees for {squire:?}"
    );
    assert_eq!(
        Proj::quests_due(store_snap, squire, today),
        Proj::quests_due(inmem_snap, squire, today),
        "quests_due disagrees for {squire:?}"
    );

    // Quest-scoped scheduled streak + an Any calendar streak.
    let scopes: [(Scope, StreakBasis); 2] = [
        (
            Scope::Quest(QuestId(QUEST_DAILY)),
            StreakBasis::ScheduledOccurrences,
        ),
        (Scope::Any, StreakBasis::CalendarDays),
    ];
    for (scope, basis) in &scopes {
        assert_eq!(
            Proj::current_streak(store_snap, squire, scope, *basis, today),
            Proj::current_streak(inmem_snap, squire, scope, *basis, today),
            "current_streak disagrees for {squire:?} scope {scope:?}"
        );
    }

    // is_unlocked for the streak achievement.
    assert_eq!(
        Proj::is_unlocked(store_snap, squire, AchievementId(ACH_STREAK)),
        Proj::is_unlocked(inmem_snap, squire, AchievementId(ACH_STREAK)),
        "is_unlocked disagrees for {squire:?}"
    );

    // can_redeem over every item — compare via debug (Blocked has no PartialEq across all arms).
    for it in &store_snap.items {
        let s = format!("{:?}", Proj::can_redeem(store_snap, squire, it.id, today));
        let m = format!("{:?}", Proj::can_redeem(inmem_snap, squire, it.id, today));
        assert_eq!(s, m, "can_redeem disagrees for {squire:?} item {:?}", it.id);
    }
}

/// Drive one `(by, cmd)` step through the engine against `repo`, returning the engine result.
/// The SAME clock is used for both repos so any emitted timestamps match byte-for-byte.
fn step<R: Repository>(repo: &mut R, by: Option<UserId>, cmd: Command, clock: &FixedClock) {
    let snap = repo.snapshot();
    let changes = DomainEngine
        .handle(&snap, cmd, clock)
        .expect("engine handle failed");
    repo.apply(by, &changes).expect("apply failed");
}

// ─── the parity test ───────────────────────────────────────────────────────────

#[test]
fn store_is_faithful_drop_in_for_in_memory() {
    each_backend(|conn| {
        let clock = the_clock();
        let mut store = Store::new(conn, clock);
        let mut inmem = InMemoryRepository::new();

        // Seed identities directly (the engine never authors users — Change::PutUser only).
        // Apply to BOTH repos identically so they start from the same state.
        use domain_core::contract::Change;
        let seed_users = vec![
            Change::PutUser(user(KNIGHT, Role::Knight)),
            Change::PutUser(user(ALICE, Role::Squire)),
            Change::PutUser(user(BORIS, Role::Squire)),
        ];
        store.apply(None, &seed_users).expect("seed users (store)");
        inmem.apply(None, &seed_users).expect("seed users (inmem)");

        // Baseline parity right after seeding.
        assert_eq!(
            snap_dbg(&store.snapshot()),
            snap_dbg(&inmem.snapshot()),
            "post-seed snapshot"
        );

        let today = clock.today;
        let squires = [UserId(ALICE), UserId(BORIS)];

        // Drive the scenario through the engine against BOTH repos, asserting parity after each
        // step — both the rehydrated snapshot AND every projection for both squires.
        for (i, (by, cmd)) in scenario().into_iter().enumerate() {
            step(&mut store, by, cmd.clone(), &clock);
            step(&mut inmem, by, cmd, &clock);

            let store_snap = store.snapshot();
            let inmem_snap = inmem.snapshot();
            assert_eq!(
                snap_dbg(&store_snap),
                snap_dbg(&inmem_snap),
                "snapshot diverged after step {i}"
            );
            for s in squires {
                assert_projections_agree(&store_snap, &inmem_snap, s, today);
            }
        }

        // Final sanity on the scenario itself (so a silently-empty engine result can't pass):
        let snap = store.snapshot();
        // Alice: 10 + 10 (two approvals) + 25 (streak bonus) + 5 (adjust) + 20 (race win)
        //        - 15 (gated item redemption) = 55.
        assert_eq!(
            Proj::balance(&snap, UserId(ALICE)),
            55,
            "Alice final balance"
        );
        assert!(
            Proj::is_unlocked(&snap, UserId(ALICE), AchievementId(ACH_STREAK)),
            "Alice unlocked"
        );
        assert!(
            !Proj::is_unlocked(&snap, UserId(BORIS), AchievementId(ACH_STREAK)),
            "Boris not unlocked"
        );
        // Boris lost the race and never completed the daily → balance 0.
        assert_eq!(
            Proj::balance(&snap, UserId(BORIS)),
            0,
            "Boris final balance"
        );

        // ── NFR-11: raw per-quest / per-item event-log queries over the same history ──
        assert_raw_logs(&store, &snap);
    });
}

/// NFR-11 ("explain how a balance/streak was reached"): the raw queries must return exactly the
/// events touching the quest / item, in seq order. We cross-check against the full snapshot.
fn assert_raw_logs(store: &Store<FixedClock>, snap: &Snapshot) {
    // Per-item: the gated item's redemption lifecycle (request + redeemed). No rejects here.
    let item_log = store.raw_log_for_item(ItemId(ITEM_GATED));
    let expected_item: Vec<&Event> = snap
        .events
        .iter()
        .filter(|e| event_item_id(e) == Some(ItemId(ITEM_GATED)))
        .collect();
    assert_eq!(
        item_log
            .iter()
            .map(|e| format!("{e:?}"))
            .collect::<Vec<_>>(),
        expected_item
            .iter()
            .map(|e| format!("{e:?}"))
            .collect::<Vec<_>>(),
        "raw_log_for_item mismatch"
    );
    // Concretely: a RedemptionRequested then an ItemRedeemed, in that order.
    assert_eq!(
        item_log.len(),
        2,
        "expected request + redeemed for the gated item"
    );
    assert!(matches!(item_log[0], Event::RedemptionRequested { .. }));
    assert!(matches!(item_log[1], Event::ItemRedeemed { .. }));

    // Per-quest: the daily quest's claims + their approvals (claim_id resolution).
    let quest_log = store.raw_log_for_quest(QuestId(QUEST_DAILY));
    // The claim_ids that belong to the daily quest (ClaimId isn't Ord, so use a Vec).
    let claim_ids: Vec<ClaimId> = snap
        .events
        .iter()
        .filter_map(|e| match e {
            Event::CompletionClaimed {
                claim_id, quest_id, ..
            } if *quest_id == QuestId(QUEST_DAILY) => Some(*claim_id),
            _ => None,
        })
        .collect();
    let expected_quest: Vec<&Event> = snap
        .events
        .iter()
        .filter(|e| match e {
            Event::CompletionClaimed { quest_id, .. } => *quest_id == QuestId(QUEST_DAILY),
            Event::CompletionApproved { claim_id, .. }
            | Event::CompletionRejected { claim_id, .. } => claim_ids.contains(claim_id),
            _ => false,
        })
        .collect();
    assert_eq!(
        quest_log
            .iter()
            .map(|e| format!("{e:?}"))
            .collect::<Vec<_>>(),
        expected_quest
            .iter()
            .map(|e| format!("{e:?}"))
            .collect::<Vec<_>>(),
        "raw_log_for_quest mismatch"
    );
    // Concretely: claim(100), approve(100), claim(101), approve(101) for the daily quest.
    assert_eq!(
        quest_log.len(),
        4,
        "two claim+approve pairs for the daily quest"
    );

    // The race quest's log: both claims + Alice's approval (501 was never reviewed).
    let race_log = store.raw_log_for_quest(QuestId(QUEST_RACE));
    assert_eq!(race_log.len(), 3, "two race claims + one approval");
}

/// The item a (redemption-family) event touches, if any. Only `RedemptionRequested` and
/// `ItemRedeemed` populate the `item_id` column — `RedemptionRejected` carries a `request_id`
/// only — so those two are exactly what `raw_log_for_item` filters on.
fn event_item_id(e: &Event) -> Option<ItemId> {
    match e {
        Event::RedemptionRequested { item_id, .. } => Some(*item_id),
        Event::ItemRedeemed { item_id, .. } => Some(*item_id),
        _ => None,
    }
}
