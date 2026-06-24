//! Repository (`snapshot` + atomic `apply`) and `Clock` tests for SQUIRE-T-0010.
//!
//! Run on BOTH backends via the same `each_backend` harness shape as the T-0009 mapping
//! tests: SQLite (temp file) always; Postgres under `--features postgres` + `DATABASE_URL`,
//! reset with `store::pg::provision_clean` and serialized behind a `Mutex`.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use domain_core::contract::{
    Achievement, AchievementId, Assignment, Availability, Cadence, Category, ClaimId, CommandId,
    Completion, Criterion, Currency, Date, Event, ItemId, Points, Quest, QuestId, RedeemableItem,
    Repository, Role, Schedule, Scope, Snapshot, StreakBasis, Timestamp, User, UserId,
};

use store::{AnyConnection, FixedClock, Store, SystemClock};

// ─── equality via debug-string (contract types don't all derive PartialEq) ─────

macro_rules! assert_dbg_eq {
    ($a:expr, $b:expr) => {{
        assert_eq!(format!("{:?}", $a), format!("{:?}", $b));
    }};
}

/// Whole-snapshot equality as a single debug string (events already ordered by seq).
fn snap_dbg(s: &Snapshot) -> String {
    format!(
        "users={:?}\nquests={:?}\nitems={:?}\nachievements={:?}\nevents={:?}",
        s.users, s.quests, s.items, s.achievements, s.events
    )
}

// ─── backend harness ───────────────────────────────────────────────────────────

/// Run `test` against each available backend, handing it a freshly-migrated, empty
/// `AnyConnection`. SQLite always; Postgres only under `--features postgres` + `DATABASE_URL`.
fn each_backend(test: impl Fn(AnyConnection)) {
    // SQLite (temp file) — always.
    {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("repo-test.sqlite");
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
            eprintln!("skipping postgres repository test: DATABASE_URL not set");
        }
    }
}

/// Build a `Store` with a deterministic clock so audit timestamps are predictable.
fn fixed_store(conn: AnyConnection, now_millis: i64) -> Store<FixedClock> {
    Store::new(conn, FixedClock::at(Timestamp(now_millis)))
}

// ─── sample data ────────────────────────────────────────────────────────────────

fn sample_users() -> Vec<User> {
    vec![
        User {
            id: UserId(1),
            role: Role::Knight,
            display_name: "Robb".into(),
            active: true,
        },
        User {
            id: UserId(2),
            role: Role::Squire,
            display_name: "Arya".into(),
            active: true,
        },
    ]
}

fn sample_quest() -> Quest {
    Quest {
        id: QuestId(10),
        title: "Sweep".into(),
        description: Some("the keep".into()),
        category: Some(Category("chores".into())),
        reward: 5 as Points,
        cash: 0,
        cadence: Cadence::Recurring(Schedule::Weekly {
            days: std::collections::BTreeSet::from([
                domain_core::contract::Weekday::Mon,
                domain_core::contract::Weekday::Wed,
            ]),
        }),
        assignment: Assignment::AllSquires,
        completion: Completion::EachAssignee,
        auto_approve: false,
        repeatable_within_day: false,
        active: true,
        icon: Some("broom".into()),
    }
}

fn sample_item() -> RedeemableItem {
    RedeemableItem {
        id: ItemId(20),
        name: "Ice cream".into(),
        description: None,
        cost: 50 as Points,
        gate: Some(AchievementId(30)),
        availability: Availability::Repeatable,
        active: true,
        icon: None,
    }
}

fn sample_achievement() -> Achievement {
    Achievement {
        id: AchievementId(30),
        name: "Diligent".into(),
        description: Some("7-day streak".into()),
        criterion: Criterion::Streak {
            scope: Scope::Any,
            length: 7,
            basis: StreakBasis::CalendarDays,
        },
        bonus_points: 25 as Points,
        active: true,
    }
}

/// A representative slice of event variants, in intended seq order.
fn sample_events() -> Vec<Event> {
    vec![
        Event::CompletionClaimed {
            claim_id: ClaimId(100),
            squire: UserId(2),
            quest_id: QuestId(10),
            on: Date(3),
            at: Timestamp(1_000),
        },
        Event::CompletionApproved {
            claim_id: ClaimId(100),
            squire: UserId(2),
            actor: Some(UserId(1)),
            points: 5 as Points,
            at: Timestamp(2_000),
        },
        Event::ItemRedeemed {
            request_id: None,
            command_id: Some(CommandId(200)),
            squire: UserId(2),
            actor: Some(UserId(1)),
            item_id: ItemId(20),
            cost: 50 as Points,
            at: Timestamp(3_000),
        },
        Event::Adjusted {
            command_id: CommandId(201),
            squire: UserId(2),
            actor: Some(UserId(1)),
            currency: Currency::Coins,
            amount: -10,
            reason: "spilled".into(),
            at: Timestamp(4_000),
        },
        Event::AchievementUnlocked {
            squire: UserId(2),
            id: AchievementId(30),
            bonus: 25 as Points,
            at: Timestamp(5_000),
        },
    ]
}

/// The full seed batch: defs/users first (as the engine would never depend on order here,
/// but we keep a natural ordering), then the events.
fn seed_batch() -> Vec<domain_core::contract::Change> {
    use domain_core::contract::Change;
    let mut changes: Vec<Change> = Vec::new();
    for u in sample_users() {
        changes.push(Change::PutUser(u));
    }
    changes.push(Change::PutQuest(sample_quest()));
    changes.push(Change::PutItem(sample_item()));
    changes.push(Change::PutAchievement(sample_achievement()));
    for ev in sample_events() {
        changes.push(Change::Append(ev));
    }
    changes
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[test]
fn snapshot_round_trip() {
    each_backend(|conn| {
        let mut store = fixed_store(conn, 1_700_000_000_000);
        store
            .apply(Some(UserId(1)), &seed_batch())
            .expect("apply seed");

        let snap = store.snapshot();

        // Users / quest / item / achievement all present, fields intact.
        let mut users = sample_users();
        users.sort_by_key(|u| u.id.0);
        let mut got_users = snap.users.clone();
        got_users.sort_by_key(|u| u.id.0);
        for (a, b) in users.iter().zip(got_users.iter()) {
            assert_dbg_eq!(a, b);
        }
        assert_eq!(snap.quests.len(), 1);
        assert_dbg_eq!(sample_quest(), snap.quests[0]);
        assert_eq!(snap.items.len(), 1);
        assert_dbg_eq!(sample_item(), snap.items[0]);
        assert_eq!(snap.achievements.len(), 1);
        assert_dbg_eq!(sample_achievement(), snap.achievements[0]);

        // Events reflected in seq order, all fields intact.
        let expected = sample_events();
        assert_eq!(snap.events.len(), expected.len());
        for (a, b) in expected.iter().zip(snap.events.iter()) {
            assert_dbg_eq!(a, b);
        }
    });
}

#[test]
fn atomic_rollback_leaves_store_unchanged() {
    use domain_core::contract::Change;
    each_backend(|conn| {
        let mut store = fixed_store(conn, 1_700_000_000_000);
        store
            .apply(Some(UserId(1)), &seed_batch())
            .expect("apply seed");

        let before = snap_dbg(&store.snapshot());

        // A batch that does a valid write THEN fails on a `SetQuestActive` for a missing id.
        let bad: Vec<Change> = vec![
            Change::Append(Event::CompletionClaimed {
                claim_id: ClaimId(999),
                squire: UserId(2),
                quest_id: QuestId(10),
                on: Date(9),
                at: Timestamp(9_999),
            }),
            Change::PutQuest({
                let mut q = sample_quest();
                q.title = "MUTATED".into();
                q
            }),
            Change::SetQuestActive(QuestId(424242), false), // missing → RepoError
        ];
        let err = store
            .apply(Some(UserId(1)), &bad)
            .expect_err("batch must fail");
        assert!(matches!(
            err,
            domain_core::contract::RepoError::Conflict | domain_core::contract::RepoError::Io(_)
        ));

        // Byte-for-byte unchanged: the new event, the mutated quest, all rolled back.
        let after = snap_dbg(&store.snapshot());
        assert_eq!(
            before, after,
            "rolled-back batch must leave the store unchanged"
        );
    });
}

#[test]
fn audit_stamping_preserves_created_on_update() {
    use domain_core::contract::Change;
    each_backend(|conn| {
        let mut store = fixed_store(conn, 1_111);

        // First PutQuest by user u=1 at now=1_111.
        store
            .apply(Some(UserId(1)), &[Change::PutQuest(sample_quest())])
            .expect("first put");
        let a1 = store::quest_audit(&mut store.connection(), QuestId(10))
            .expect("read audit")
            .expect("row exists");
        assert_eq!(a1.created_by, Some(UserId(1)));
        assert_eq!(a1.updated_by, Some(UserId(1)));
        assert_eq!(a1.created_at, Timestamp(1_111));
        assert_eq!(a1.updated_at, Timestamp(1_111));

        // Second PutQuest (same id) by user v=2 at a later now.
        *store.clock_mut() = FixedClock::at(Timestamp(2_222));
        store
            .apply(
                Some(UserId(2)),
                &[Change::PutQuest({
                    let mut q = sample_quest();
                    q.title = "Sweep harder".into();
                    q
                })],
            )
            .expect("second put");
        let a2 = store::quest_audit(&mut store.connection(), QuestId(10))
            .expect("read audit")
            .expect("row exists");
        // created_* preserved; updated_* moved to v / later time.
        assert_eq!(
            a2.created_by,
            Some(UserId(1)),
            "created_by must be preserved"
        );
        assert_eq!(
            a2.created_at,
            Timestamp(1_111),
            "created_at must be preserved"
        );
        assert_eq!(a2.updated_by, Some(UserId(2)));
        assert_eq!(a2.updated_at, Timestamp(2_222));

        // SetQuestActive by w=3: updated_by=w, created_* still unchanged.
        *store.clock_mut() = FixedClock::at(Timestamp(3_333));
        store
            .apply(
                Some(UserId(3)),
                &[Change::SetQuestActive(QuestId(10), false)],
            )
            .expect("set active");
        let a3 = store::quest_audit(&mut store.connection(), QuestId(10))
            .expect("read audit")
            .expect("row exists");
        assert_eq!(a3.created_by, Some(UserId(1)), "created_by still preserved");
        assert_eq!(a3.created_at, Timestamp(1_111));
        assert_eq!(a3.updated_by, Some(UserId(3)));
        assert_eq!(a3.updated_at, Timestamp(3_333));

        // by = None → NULL audit user.
        *store.clock_mut() = FixedClock::at(Timestamp(4_444));
        store
            .apply(
                None,
                &[Change::PutUser(User {
                    id: UserId(77),
                    role: Role::Squire,
                    display_name: "System".into(),
                    active: true,
                })],
            )
            .expect("system put");
        let au = store::user_audit(&mut store.connection(), UserId(77))
            .expect("read audit")
            .expect("row exists");
        assert_eq!(au.created_by, None);
        assert_eq!(au.updated_by, None);
    });
}

#[test]
fn append_only_seq_is_monotonic_and_immutable() {
    use domain_core::contract::Change;
    each_backend(|conn| {
        let mut store = fixed_store(conn, 1);

        // First batch of 3 events → seq 1,2,3.
        let first: Vec<Change> = sample_events()
            .into_iter()
            .take(3)
            .map(Change::Append)
            .collect();
        store.apply(None, &first).expect("first append");
        let after_first: Vec<String> = store
            .snapshot()
            .events
            .iter()
            .map(|e| format!("{e:?}"))
            .collect();
        assert_eq!(after_first.len(), 3);

        // Second batch of 2 → seq 4,5; earlier events untouched.
        let second: Vec<Change> = sample_events()
            .into_iter()
            .skip(3)
            .map(Change::Append)
            .collect();
        store.apply(None, &second).expect("second append");
        let after_second = store.snapshot().events;
        assert_eq!(after_second.len(), 5);
        // The first three debug-strings are unchanged (rows never modified).
        for (i, prev) in after_first.iter().enumerate() {
            assert_eq!(prev, &format!("{:?}", after_second[i]));
        }

        // Verify seq column itself is strictly increasing 1..=5.
        let seqs = raw_event_seqs(&mut store.connection());
        assert_eq!(seqs, vec![1, 2, 3, 4, 5]);
    });
}

/// Mirror of the domain's `weekday_of` (Date(0) == Monday). `common` is `pub(crate)` in
/// domain-core, so we re-derive the same pure mapping here to assert alignment.
fn weekday_of(d: Date) -> domain_core::contract::Weekday {
    use domain_core::contract::Weekday::*;
    match ((d.0 % 7) + 7) % 7 {
        0 => Mon,
        1 => Tue,
        2 => Wed,
        3 => Thu,
        4 => Fri,
        5 => Sat,
        _ => Sun,
    }
}

#[test]
fn clock_today_is_monday_aligned() {
    use domain_core::contract::{Clock, Weekday};

    // today() must satisfy the domain's Date(0) == Monday convention.
    // 2024-01-01 was a Monday. Its unix-day count (UTC) is 19723; the Monday-aligned Date
    // is 19723 + 3 = 19726, whose weekday must be Mon.
    let date = FixedClock::at(Timestamp(19_723i64 * 86_400_000)).today();
    assert_eq!(date, Date(19_726));
    assert_eq!(weekday_of(date), Weekday::Mon);

    // 2024-01-03 was a Wednesday.
    let wed = FixedClock::at(Timestamp(19_725i64 * 86_400_000)).today();
    assert_eq!(weekday_of(wed), Weekday::Wed);

    // SystemClock flows now()/today() through the same conversion; today() is consistent
    // with now() to the day.
    let sys = SystemClock;
    let n = sys.now().0;
    let expected_day = (n.div_euclid(86_400_000) + 3) as i32;
    assert_eq!(sys.today(), Date(expected_day));
}

/// Credentials live IN the tenant store (SQUIRE-S-0007 / REQ-1.6) and are written/read directly
/// (bypassing `apply`). `set_credential` upserts by `user_id`; `credential` reads the hash back.
/// Runs on both backends so the upsert dispatch is proven on SQLite and Postgres alike.
#[test]
fn credentials_set_and_read_back() {
    each_backend(|conn| {
        let store = Store::new(conn, SystemClock);

        // Absent before any write.
        assert_eq!(store.credential(UserId(1)), None);

        // Set, then read back verbatim.
        store
            .set_credential(UserId(1), "$argon2id$hash-one")
            .expect("set credential");
        assert_eq!(
            store.credential(UserId(1)).as_deref(),
            Some("$argon2id$hash-one")
        );

        // Upsert by user_id: a second set replaces the hash (no duplicate row).
        store
            .set_credential(UserId(1), "$argon2id$hash-two")
            .expect("replace credential");
        assert_eq!(
            store.credential(UserId(1)).as_deref(),
            Some("$argon2id$hash-two")
        );

        // A different user is independent.
        store
            .set_credential(UserId(2), "$argon2id$other")
            .expect("set other");
        assert_eq!(
            store.credential(UserId(2)).as_deref(),
            Some("$argon2id$other")
        );
        assert_eq!(
            store.credential(UserId(1)).as_deref(),
            Some("$argon2id$hash-two")
        );
    });
}

// ─── small helpers ─────────────────────────────────────────────────────────────

/// Read the raw `seq` column values in ascending order.
fn raw_event_seqs(conn: &mut AnyConnection) -> Vec<i64> {
    use store::schema::events;
    events::table
        .select(events::seq)
        .order(events::seq.asc())
        .load(conn)
        .expect("load seqs")
}
