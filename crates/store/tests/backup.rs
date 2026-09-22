//! Durability + per-tenant single-file export/import (SQUIRE-T-0012).
//!
//! Two guarantees, both proven on BOTH backends via the same parametrized harness shape as
//! the T-0010 repository tests:
//!
//! 1. **Durability across reopen** — data written to a SQLite *file* (not `:memory:`) survives
//!    closing and reopening the connection. For Postgres, reconnecting to the same schema sees
//!    the same data.
//! 2. **Export → import round-trip** — a full backend-portable dump of store A round-trips into
//!    a FRESH, equal store B, preserving the domain `Snapshot` AND the audit columns + `seq`.
//!
//! SQLite always; Postgres under `--features postgres` + `DATABASE_URL`, serialized behind a
//! `Mutex` (shared `public` schema), reset with `store::pg::provision_clean`.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use domain_core::contract::{
    Achievement, AchievementId, Assignment, Availability, Cadence, Category, ClaimId, CommandId,
    Completion, Criterion, Currency, Date, Event, ItemId, Points, Quest, QuestId, RedeemableItem,
    Repository, Role, Schedule, Scope, Snapshot, StreakBasis, Timestamp, User, UserId,
};

use store::{AnyConnection, FixedClock, Store};

// ─── whole-snapshot equality via debug-string ─────────────────────────────────

fn snap_dbg(s: &Snapshot) -> String {
    format!(
        "users={:?}\nquests={:?}\nitems={:?}\nachievements={:?}\nevents={:?}",
        s.users, s.quests, s.items, s.achievements, s.events
    )
}

// ─── sample data (mirrors repository.rs) ──────────────────────────────────────

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
        due_time: None,
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

/// Defs/users first, then events. Non-trivial audit comes from `apply(Some(uid), …)`.
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

// ─── Postgres harness plumbing ────────────────────────────────────────────────

#[cfg(feature = "postgres")]
use std::sync::Mutex;
#[cfg(feature = "postgres")]
static PG_LOCK: Mutex<()> = Mutex::new(());

/// Reset + migrate the shared Postgres `public` schema and hand back a fresh connection.
#[cfg(feature = "postgres")]
fn fresh_pg(url: &str) -> AnyConnection {
    store::pg::provision_clean(url).expect("reset + migrate postgres");
    AnyConnection::Pg(diesel::pg::PgConnection::establish(url).expect("pg connect"))
}

// ─── test 1: durability across reopen ─────────────────────────────────────────

#[test]
fn durability_sqlite_reopen_same_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("durable.sqlite");
    let url = path.to_str().expect("utf-8 path").to_string();

    // Open at a FILE path (not :memory:), apply a batch, then DROP the store (close conn).
    let before = {
        let mut store = store::SqliteStore::open(&url).expect("open A");
        store
            .apply(Some(UserId(1)), &seed_batch())
            .expect("apply seed");
        snap_dbg(&store.snapshot())
        // `store` dropped here → connection closed.
    };

    // Reopen the SAME file: data must be intact.
    let store = store::SqliteStore::open(&url).expect("reopen");
    let after = snap_dbg(&store.snapshot());
    assert_eq!(
        before, after,
        "data must survive close + reopen of the same SQLite file"
    );
    assert!(!store.snapshot().events.is_empty(), "events must persist");
}

/// Postgres durability: write through one connection, then reconnect to the SAME schema and
/// confirm persistence.
#[cfg(feature = "postgres")]
#[test]
fn durability_postgres_reconnect_same_schema() {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping postgres durability test: DATABASE_URL not set");
        return;
    };
    let _guard = PG_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let before = {
        let mut store = Store::new(fresh_pg(&url), FixedClock::at(Timestamp(1_700_000_000_000)));
        store
            .apply(Some(UserId(1)), &seed_batch())
            .expect("apply seed");
        snap_dbg(&store.snapshot())
        // store dropped → connection closed; data committed to the schema.
    };

    // Reconnect WITHOUT resetting the schema.
    let conn = AnyConnection::Pg(diesel::pg::PgConnection::establish(&url).expect("pg reconnect"));
    let store = Store::new(conn, FixedClock::at(Timestamp(0)));
    let after = snap_dbg(&store.snapshot());
    assert_eq!(
        before, after,
        "data must survive reconnect to the same Postgres schema"
    );
}

// ─── test 2 + 3: export → import round-trip, one-file ─────────────────────────

/// Drive the export/import round-trip with two fresh, migrated connections (A populated, B
/// empty). Returns nothing; asserts inline.
fn round_trip(conn_a: AnyConnection, mut conn_b: AnyConnection) {
    let dir = tempfile::tempdir().expect("tempdir");
    let dump_path = dir.path().join("household.json");

    // Populate store A with non-trivial audit (apply with Some(uid)).
    let mut store_a = Store::new(conn_a, FixedClock::at(Timestamp(1_700_000_000_000)));
    store_a
        .apply(Some(UserId(1)), &seed_batch())
        .expect("apply seed into A");

    // Export A to ONE file.
    store_a.export(&dump_path).expect("export A");

    // One-file: the export is a single file that exists and is non-empty.
    let meta = std::fs::metadata(&dump_path).expect("dump file exists");
    assert!(meta.is_file(), "export must be a single file");
    assert!(meta.len() > 0, "export file must be non-empty");

    // Import into a FRESH migrated store B.
    store::backup::import(&mut conn_b, &dump_path).expect("import into B");
    let mut store_b = Store::new(conn_b, FixedClock::at(Timestamp(0)));

    // Whole-snapshot equality (debug-string compare).
    let snap_a = snap_dbg(&store_a.snapshot());
    let snap_b = snap_dbg(&store_b.snapshot());
    assert_eq!(
        snap_a, snap_b,
        "snapshot(A) must equal snapshot(B) after round-trip"
    );

    // Spot-check: audit columns survived (created_by / updated_by) for the quest.
    let qa = store::quest_audit(&mut store_a.connection(), QuestId(10))
        .expect("read A quest audit")
        .expect("quest exists in A");
    let qb = store::quest_audit(&mut store_b.connection(), QuestId(10))
        .expect("read B quest audit")
        .expect("quest exists in B");
    assert_eq!(
        qa.created_by, qb.created_by,
        "created_by must survive round-trip"
    );
    assert_eq!(
        qa.updated_by, qb.updated_by,
        "updated_by must survive round-trip"
    );
    assert_eq!(
        qa.created_at, qb.created_at,
        "created_at must survive round-trip"
    );
    assert_eq!(
        qa.updated_at, qb.updated_at,
        "updated_at must survive round-trip"
    );
    assert_eq!(
        qa.created_by,
        Some(UserId(1)),
        "non-trivial audit was captured"
    );

    // Spot-check: `seq` survived verbatim (events ordered, seq preserved).
    let seqs_a = raw_event_seqs(&mut store_a.connection());
    let seqs_b = raw_event_seqs(&mut store_b.connection());
    assert_eq!(
        seqs_a, seqs_b,
        "event `seq` must survive round-trip verbatim"
    );
    assert_eq!(
        seqs_a,
        vec![1, 2, 3, 4, 5],
        "five events with their original seq"
    );
}

#[test]
fn export_import_round_trip_sqlite() {
    let a = fresh_sqlite();
    let b = fresh_sqlite();
    round_trip(a, b);
}

#[cfg(feature = "postgres")]
#[test]
fn export_import_round_trip_postgres() {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping postgres round-trip test: DATABASE_URL not set");
        return;
    };
    let _guard = PG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // A and B share the same Postgres schema, so they cannot coexist: provision → populate →
    // export A; then reset the schema for a truly FRESH B and import. We snapshot A's debug
    // string before resetting and compare B against it.
    let dump_path_dir = tempfile::tempdir().expect("tempdir");
    let dump_path = dump_path_dir.path().join("household.json");

    // Populate + export A.
    let snap_a = {
        let mut store_a = Store::new(fresh_pg(&url), FixedClock::at(Timestamp(1_700_000_000_000)));
        store_a
            .apply(Some(UserId(1)), &seed_batch())
            .expect("apply seed into A (pg)");
        store_a.export(&dump_path).expect("export A (pg)");
        let s = snap_dbg(&store_a.snapshot());
        let qa = store::quest_audit(&mut store_a.connection(), QuestId(10))
            .expect("audit A")
            .expect("quest A");
        let seqs_a = raw_event_seqs(&mut store_a.connection());
        (s, qa, seqs_a)
    };

    // One-file check.
    let meta = std::fs::metadata(&dump_path).expect("dump exists");
    assert!(
        meta.is_file() && meta.len() > 0,
        "export must be one non-empty file"
    );

    // Fresh B (resets the shared schema), import.
    let mut store_b = {
        let mut conn_b = fresh_pg(&url);
        store::backup::import(&mut conn_b, &dump_path).expect("import into B (pg)");
        Store::new(conn_b, FixedClock::at(Timestamp(0)))
    };

    let (snap_a_str, qa, seqs_a) = snap_a;
    assert_eq!(
        snap_a_str,
        snap_dbg(&store_b.snapshot()),
        "snapshot(A)==snapshot(B) on pg"
    );

    let qb = store::quest_audit(&mut store_b.connection(), QuestId(10))
        .expect("audit B")
        .expect("quest B");
    assert_eq!(qa.created_by, qb.created_by);
    assert_eq!(qa.updated_by, qb.updated_by);
    assert_eq!(qa.created_at, qb.created_at);
    assert_eq!(qa.updated_at, qb.updated_at);
    assert_eq!(qb.created_by, Some(UserId(1)));

    let seqs_b = raw_event_seqs(&mut store_b.connection());
    assert_eq!(seqs_a, seqs_b, "seq survives on pg");
}

// ─── helpers ──────────────────────────────────────────────────────────────────

/// A fresh, migrated SQLite temp-file connection.
fn fresh_sqlite() -> AnyConnection {
    // Leak a tempdir so the file outlives this call; the OS cleans /tmp. Using a unique name
    // per call keeps A and B distinct files.
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("rt.sqlite");
    let url = path.to_str().expect("utf-8 path").to_string();
    let mut sqlite = SqliteConnection::establish(&url).expect("sqlite");
    store::run_migrations(&mut sqlite).expect("sqlite migrations");
    // Keep the tempdir alive for the duration of the test process.
    std::mem::forget(dir);
    AnyConnection::Sqlite(sqlite)
}

/// Read the raw `seq` column values in ascending order.
fn raw_event_seqs(conn: &mut AnyConnection) -> Vec<i64> {
    use store::schema::events;
    events::table
        .select(events::seq)
        .order(events::seq.asc())
        .load(conn)
        .expect("load seqs")
}
