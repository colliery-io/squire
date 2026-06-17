//! Mapping tests for SQUIRE-T-0009.
//!
//! Two layers:
//!   1. **Pure round-trip** (proptest): for randomized values of every domain type and
//!      every `Event` variant, `decode(encode(x)) == x`. Most contract types do NOT derive
//!      `PartialEq`, so equality is asserted via debug-string (`format!("{:?}", _)`)
//!      comparison — lossless for these `Debug`-deriving types and avoids adding derives
//!      to the contract.
//!   2. **DB round-trip on BOTH backends**: insert encoded rows, then select + decode, and
//!      assert equality. SQLite always runs (temp file); Postgres runs only under
//!      `--features postgres` with `DATABASE_URL` set.

use std::collections::BTreeSet;

use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use domain_core::contract::{
    Achievement, AchievementId, Assignment, Availability, Cadence, Category, ClaimId, CommandId,
    Completion, Criterion, Date, Event, ItemId, Points, Quest, QuestId, RedeemableItem, RequestId,
    Role, Scope, Schedule, StreakBasis, Timestamp, User, UserId, Weekday,
};
use store::rows::{AchievementRow, Audit, EventRow, ItemRow, QuestRow, UserRow};
use store::schema::{achievements, events, items, quests, users};
use store::AnyConnection;

use proptest::prelude::*;

// ─── equality via debug-string ──────────────────────────────────────────────
//
// The contract types derive `Debug` but not (all) `PartialEq`. Comparing `{:?}` strings
// is a lossless equality check for round-tripping and keeps the contract untouched.

macro_rules! assert_dbg_eq {
    ($a:expr, $b:expr) => {{
        assert_eq!(format!("{:?}", $a), format!("{:?}", $b));
    }};
}

const AUDIT: Audit = Audit {
    created_by: Some(UserId(7)),
    created_at: Timestamp(1_000),
    updated_by: None,
    updated_at: Timestamp(2_000),
};

// ─── proptest strategies ────────────────────────────────────────────────────

fn arb_role() -> impl Strategy<Value = Role> {
    prop_oneof![Just(Role::Knight), Just(Role::Squire)]
}

fn arb_user() -> impl Strategy<Value = User> {
    (any::<u128>(), arb_role(), ".*", any::<bool>()).prop_map(|(id, role, name, active)| User {
        id: UserId(id),
        role,
        display_name: name,
        active,
    })
}

fn arb_weekday() -> impl Strategy<Value = Weekday> {
    prop_oneof![
        Just(Weekday::Mon),
        Just(Weekday::Tue),
        Just(Weekday::Wed),
        Just(Weekday::Thu),
        Just(Weekday::Fri),
        Just(Weekday::Sat),
        Just(Weekday::Sun),
    ]
}

fn arb_cadence() -> impl Strategy<Value = Cadence> {
    prop_oneof![
        proptest::option::of(any::<i32>().prop_map(Date)).prop_map(|due| Cadence::OneOff { due }),
        Just(Cadence::Recurring(Schedule::Daily)),
        prop::collection::btree_set(arb_weekday(), 0..7)
            .prop_map(|days| Cadence::Recurring(Schedule::Weekly { days })),
        (any::<u16>(), any::<i32>())
            .prop_map(|(n, a)| Cadence::Recurring(Schedule::EveryNDays { n, anchor: Date(a) })),
    ]
}

fn arb_assignment() -> impl Strategy<Value = Assignment> {
    prop_oneof![
        Just(Assignment::AllSquires),
        prop::collection::btree_set(any::<u128>().prop_map(UserId), 0..5)
            .prop_map(Assignment::Squires),
    ]
}

fn arb_completion() -> impl Strategy<Value = Completion> {
    prop_oneof![Just(Completion::EachAssignee), Just(Completion::Race)]
}

fn arb_quest() -> impl Strategy<Value = Quest> {
    (
        any::<u128>(),
        ".*",
        proptest::option::of(".*"),
        proptest::option::of(".*"),
        any::<u32>(),
        arb_cadence(),
        arb_assignment(),
        arb_completion(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        proptest::option::of(".*"),
    )
        .prop_map(
            |(
                id,
                title,
                description,
                category,
                reward,
                cadence,
                assignment,
                completion,
                auto_approve,
                repeatable_within_day,
                active,
                icon,
            )| Quest {
                id: QuestId(id),
                title,
                description,
                category: category.map(Category),
                reward: reward as Points,
                cadence,
                assignment,
                completion,
                auto_approve,
                repeatable_within_day,
                active,
                icon,
            },
        )
}

fn arb_availability() -> impl Strategy<Value = Availability> {
    prop_oneof![Just(Availability::Once), Just(Availability::Repeatable)]
}

fn arb_item() -> impl Strategy<Value = RedeemableItem> {
    (
        any::<u128>(),
        ".*",
        proptest::option::of(".*"),
        any::<u32>(),
        proptest::option::of(any::<u128>().prop_map(AchievementId)),
        arb_availability(),
        any::<bool>(),
        proptest::option::of(".*"),
    )
        .prop_map(
            |(id, name, description, cost, gate, availability, active, icon)| RedeemableItem {
                id: ItemId(id),
                name,
                description,
                cost: cost as Points,
                gate,
                availability,
                active,
                icon,
            },
        )
}

fn arb_scope() -> impl Strategy<Value = Scope> {
    prop_oneof![
        any::<u128>().prop_map(|q| Scope::Quest(QuestId(q))),
        ".*".prop_map(|c| Scope::Category(Category(c))),
        Just(Scope::Any),
    ]
}

fn arb_basis() -> impl Strategy<Value = StreakBasis> {
    prop_oneof![
        Just(StreakBasis::ScheduledOccurrences),
        Just(StreakBasis::CalendarDays),
    ]
}

fn arb_criterion() -> impl Strategy<Value = Criterion> {
    prop_oneof![
        (arb_scope(), any::<u32>(), arb_basis())
            .prop_map(|(scope, length, basis)| Criterion::Streak { scope, length, basis }),
        (arb_scope(), any::<u32>())
            .prop_map(|(scope, count)| Criterion::TotalCompletions { scope, count }),
        any::<u32>().prop_map(|total| Criterion::PointsEarned { total: total as Points }),
    ]
}

fn arb_achievement() -> impl Strategy<Value = Achievement> {
    (
        any::<u128>(),
        ".*",
        proptest::option::of(".*"),
        arb_criterion(),
        any::<u32>(),
        any::<bool>(),
    )
        .prop_map(|(id, name, description, criterion, bonus, active)| Achievement {
            id: AchievementId(id),
            name,
            description,
            criterion,
            bonus_points: bonus as Points,
            active,
        })
}

fn arb_opt_user() -> impl Strategy<Value = Option<UserId>> {
    proptest::option::of(any::<u128>().prop_map(UserId))
}

fn arb_event() -> impl Strategy<Value = Event> {
    let uid = any::<u128>().prop_map(UserId);
    let ts = any::<i64>().prop_map(Timestamp);
    let date = any::<i32>().prop_map(Date);
    prop_oneof![
        (any::<u128>(), uid.clone(), any::<u128>(), date.clone(), ts.clone()).prop_map(
            |(c, s, q, on, at)| Event::CompletionClaimed {
                claim_id: ClaimId(c),
                squire: s,
                quest_id: QuestId(q),
                on,
                at,
            }
        ),
        (any::<u128>(), uid.clone(), arb_opt_user(), any::<u32>(), ts.clone()).prop_map(
            |(c, s, actor, points, at)| Event::CompletionApproved {
                claim_id: ClaimId(c),
                squire: s,
                actor,
                points: points as Points,
                at,
            }
        ),
        (
            any::<u128>(),
            uid.clone(),
            arb_opt_user(),
            proptest::option::of(".*"),
            ts.clone()
        )
            .prop_map(|(c, s, actor, reason, at)| Event::CompletionRejected {
                claim_id: ClaimId(c),
                squire: s,
                actor,
                reason,
                at,
            }),
        (
            proptest::option::of(any::<u128>().prop_map(RequestId)),
            proptest::option::of(any::<u128>().prop_map(CommandId)),
            uid.clone(),
            arb_opt_user(),
            any::<u128>(),
            any::<u32>(),
            ts.clone()
        )
            .prop_map(|(rq, cmd, s, actor, item, cost, at)| Event::ItemRedeemed {
                request_id: rq,
                command_id: cmd,
                squire: s,
                actor,
                item_id: ItemId(item),
                cost: cost as Points,
                at,
            }),
        (uid.clone(), any::<u128>(), any::<u32>(), ts.clone()).prop_map(
            |(s, id, bonus, at)| Event::AchievementUnlocked {
                squire: s,
                id: AchievementId(id),
                bonus: bonus as Points,
                at,
            }
        ),
        (
            any::<u128>(),
            uid.clone(),
            arb_opt_user(),
            any::<i64>(),
            ".*",
            ts.clone()
        )
            .prop_map(|(cmd, s, actor, amount, reason, at)| Event::PointsAdjusted {
                command_id: CommandId(cmd),
                squire: s,
                actor,
                amount,
                reason,
                at,
            }),
        (any::<u128>(), uid.clone(), any::<u128>(), ts.clone()).prop_map(
            |(rq, s, item, at)| Event::RedemptionRequested {
                request_id: RequestId(rq),
                squire: s,
                item_id: ItemId(item),
                at,
            }
        ),
        (
            any::<u128>(),
            uid.clone(),
            arb_opt_user(),
            proptest::option::of(".*"),
            ts.clone()
        )
            .prop_map(|(rq, s, actor, reason, at)| Event::RedemptionRejected {
                request_id: RequestId(rq),
                squire: s,
                actor,
                reason,
                at,
            }),
    ]
}

// ─── pure round-trip tests ──────────────────────────────────────────────────

proptest! {
    #[test]
    fn user_round_trip(u in arb_user()) {
        let row = UserRow::from_user(&u, AUDIT);
        let back = row.to_user().expect("decode user");
        assert_dbg_eq!(u, back);
    }

    #[test]
    fn quest_round_trip(q in arb_quest()) {
        let row = QuestRow::from_quest(&q, AUDIT);
        let back = row.to_quest().expect("decode quest");
        assert_dbg_eq!(q, back);
    }

    #[test]
    fn item_round_trip(it in arb_item()) {
        let row = ItemRow::from_item(&it, AUDIT);
        let back = row.to_item().expect("decode item");
        assert_dbg_eq!(it, back);
    }

    #[test]
    fn achievement_round_trip(a in arb_achievement()) {
        let row = AchievementRow::from_achievement(&a, AUDIT);
        let back = row.to_achievement().expect("decode achievement");
        assert_dbg_eq!(a, back);
    }

    #[test]
    fn event_round_trip(ev in arb_event()) {
        let row = EventRow::from_event(42, &ev);
        let back = row.to_event().expect("decode event");
        assert_dbg_eq!(ev, back);
    }
}

// ─── audit columns survive the row layer ────────────────────────────────────

#[test]
fn audit_is_carried_on_rows_not_domain() {
    let u = User { id: UserId(1), role: Role::Squire, display_name: "Ned".into(), active: true };
    let row = UserRow::from_user(&u, AUDIT);
    assert_eq!(row.created_by.as_deref(), Some("7"));
    assert_eq!(row.created_at, 1_000);
    assert_eq!(row.updated_by, None);
    assert_eq!(row.updated_at, 2_000);
}

// ─── DB round-trip harness (both backends) ──────────────────────────────────

/// Run `test` against each available backend. SQLite always; Postgres only when built
/// with `--features postgres` AND `DATABASE_URL` is set.
fn each_backend(test: impl Fn(&mut AnyConnection)) {
    // SQLite (temp file) — always.
    {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("mapping-test.sqlite");
        let url = path.to_str().expect("utf-8 path");
        let mut conn = AnyConnection::Sqlite(SqliteConnection::establish(url).expect("sqlite"));
        // Run the embedded migrations on the SQLite arm.
        {
            let AnyConnection::Sqlite(ref mut c) = conn else { unreachable!() };
            store::run_migrations(c).expect("sqlite migrations");
        }
        test(&mut conn);
    }

    // Postgres — opt-in. The tests share one `public` schema, so serialize the
    // reset+migrate+exercise cycle (cargo runs test fns concurrently) to avoid a race on
    // `DROP SCHEMA … CASCADE; CREATE SCHEMA public`.
    #[cfg(feature = "postgres")]
    {
        use std::sync::Mutex;
        static PG_LOCK: Mutex<()> = Mutex::new(());

        if let Ok(url) = std::env::var("DATABASE_URL") {
            let _guard = PG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            store::pg::provision_clean(&url).expect("reset + migrate postgres");
            let mut conn = AnyConnection::Pg(
                diesel::pg::PgConnection::establish(&url).expect("pg connect"),
            );
            test(&mut conn);
        } else {
            eprintln!("skipping postgres DB round-trip: DATABASE_URL not set");
        }
    }
}

/// Clear all tables so each `test` body starts from a clean slate (SQLite arm reuses one
/// freshly-migrated file; Postgres is reset by `provision_clean`).
fn truncate_all(conn: &mut AnyConnection) {
    conn.batch_execute(
        "DELETE FROM events; DELETE FROM achievements; DELETE FROM items; \
         DELETE FROM quests; DELETE FROM users;",
    )
    .expect("truncate");
}

#[test]
fn db_round_trip_users() {
    each_backend(|conn| {
        truncate_all(conn);
        let samples = vec![
            User { id: UserId(1), role: Role::Knight, display_name: "Robb".into(), active: true },
            User { id: UserId(2), role: Role::Squire, display_name: "Arya".into(), active: false },
        ];
        for u in &samples {
            let row = UserRow::from_user(u, AUDIT);
            diesel::insert_into(users::table)
                .values(&row)
                .execute(conn)
                .expect("insert user");
        }
        let rows: Vec<UserRow> = users::table
            .select(UserRow::as_select())
            .order(users::id.asc())
            .load(conn)
            .expect("load users");
        let mut decoded: Vec<User> = rows.iter().map(|r| r.to_user().expect("decode")).collect();
        decoded.sort_by_key(|u| u.id.0);
        let mut expected = samples.clone();
        expected.sort_by_key(|u| u.id.0);
        for (a, b) in expected.iter().zip(decoded.iter()) {
            assert_dbg_eq!(a, b);
        }
    });
}

#[test]
fn db_round_trip_quests() {
    // One quest per cadence × representative assignment/completion combo.
    let samples = vec![
        Quest {
            id: QuestId(10),
            title: "OneOff".into(),
            description: Some("desc".into()),
            category: Some(Category("chores".into())),
            reward: 5,
            cadence: Cadence::OneOff { due: Some(Date(100)) },
            assignment: Assignment::AllSquires,
            completion: Completion::EachAssignee,
            auto_approve: true,
            repeatable_within_day: false,
            active: true,
            icon: Some("broom".into()),
        },
        Quest {
            id: QuestId(11),
            title: "Daily".into(),
            description: None,
            category: None,
            reward: 0,
            cadence: Cadence::Recurring(Schedule::Daily),
            assignment: Assignment::Squires(BTreeSet::from([UserId(1), UserId(2)])),
            completion: Completion::Race,
            auto_approve: false,
            repeatable_within_day: true,
            active: false,
            icon: None,
        },
        Quest {
            id: QuestId(12),
            title: "Weekly".into(),
            description: None,
            category: None,
            reward: 3,
            cadence: Cadence::Recurring(Schedule::Weekly {
                days: BTreeSet::from([Weekday::Mon, Weekday::Wed, Weekday::Fri]),
            }),
            assignment: Assignment::Squires(BTreeSet::new()),
            completion: Completion::EachAssignee,
            auto_approve: false,
            repeatable_within_day: false,
            active: true,
            icon: None,
        },
        Quest {
            id: QuestId(13),
            title: "EveryNDays".into(),
            description: None,
            category: None,
            reward: 7,
            cadence: Cadence::Recurring(Schedule::EveryNDays { n: 3, anchor: Date(-5) }),
            assignment: Assignment::AllSquires,
            completion: Completion::Race,
            auto_approve: true,
            repeatable_within_day: true,
            active: true,
            icon: None,
        },
        // OneOff with no due date.
        Quest {
            id: QuestId(14),
            title: "OneOffNoDue".into(),
            description: None,
            category: None,
            reward: 1,
            cadence: Cadence::OneOff { due: None },
            assignment: Assignment::AllSquires,
            completion: Completion::EachAssignee,
            auto_approve: false,
            repeatable_within_day: false,
            active: true,
            icon: None,
        },
    ];

    each_backend(|conn| {
        truncate_all(conn);
        for q in &samples {
            let row = QuestRow::from_quest(q, AUDIT);
            diesel::insert_into(quests::table)
                .values(&row)
                .execute(conn)
                .expect("insert quest");
        }
        let rows: Vec<QuestRow> = quests::table
            .select(QuestRow::as_select())
            .order(quests::id.asc())
            .load(conn)
            .expect("load quests");
        let mut decoded: Vec<Quest> =
            rows.iter().map(|r| r.to_quest().expect("decode")).collect();
        decoded.sort_by_key(|q| q.id.0);
        let mut expected = samples.clone();
        expected.sort_by_key(|q| q.id.0);
        for (a, b) in expected.iter().zip(decoded.iter()) {
            assert_dbg_eq!(a, b);
        }
    });
}

#[test]
fn db_round_trip_items() {
    let samples = vec![
        RedeemableItem {
            id: ItemId(20),
            name: "Toy".into(),
            description: Some("a toy".into()),
            cost: 100,
            gate: Some(AchievementId(99)),
            availability: Availability::Once,
            active: true,
            icon: Some("star".into()),
        },
        RedeemableItem {
            id: ItemId(21),
            name: "Snack".into(),
            description: None,
            cost: 5,
            gate: None,
            availability: Availability::Repeatable,
            active: false,
            icon: None,
        },
    ];

    each_backend(|conn| {
        truncate_all(conn);
        for it in &samples {
            let row = ItemRow::from_item(it, AUDIT);
            diesel::insert_into(items::table)
                .values(&row)
                .execute(conn)
                .expect("insert item");
        }
        let rows: Vec<ItemRow> = items::table
            .select(ItemRow::as_select())
            .order(items::id.asc())
            .load(conn)
            .expect("load items");
        let mut decoded: Vec<RedeemableItem> =
            rows.iter().map(|r| r.to_item().expect("decode")).collect();
        decoded.sort_by_key(|i| i.id.0);
        let mut expected = samples.clone();
        expected.sort_by_key(|i| i.id.0);
        for (a, b) in expected.iter().zip(decoded.iter()) {
            assert_dbg_eq!(a, b);
        }
    });
}

#[test]
fn db_round_trip_achievements() {
    let samples = vec![
        Achievement {
            id: AchievementId(30),
            name: "StreakQuest".into(),
            description: Some("d".into()),
            criterion: Criterion::Streak {
                scope: Scope::Quest(QuestId(10)),
                length: 7,
                basis: StreakBasis::ScheduledOccurrences,
            },
            bonus_points: 50,
            active: true,
        },
        Achievement {
            id: AchievementId(31),
            name: "StreakCategory".into(),
            description: None,
            criterion: Criterion::Streak {
                scope: Scope::Category(Category("reading".into())),
                length: 3,
                basis: StreakBasis::CalendarDays,
            },
            bonus_points: 10,
            active: true,
        },
        Achievement {
            id: AchievementId(32),
            name: "TotalAny".into(),
            description: None,
            criterion: Criterion::TotalCompletions { scope: Scope::Any, count: 100 },
            bonus_points: 0,
            active: false,
        },
        Achievement {
            id: AchievementId(33),
            name: "Points".into(),
            description: None,
            criterion: Criterion::PointsEarned { total: 1000 },
            bonus_points: 25,
            active: true,
        },
    ];

    each_backend(|conn| {
        truncate_all(conn);
        for a in &samples {
            let row = AchievementRow::from_achievement(a, AUDIT);
            diesel::insert_into(achievements::table)
                .values(&row)
                .execute(conn)
                .expect("insert achievement");
        }
        let rows: Vec<AchievementRow> = achievements::table
            .select(AchievementRow::as_select())
            .order(achievements::id.asc())
            .load(conn)
            .expect("load achievements");
        let mut decoded: Vec<Achievement> =
            rows.iter().map(|r| r.to_achievement().expect("decode")).collect();
        decoded.sort_by_key(|a| a.id.0);
        let mut expected = samples.clone();
        expected.sort_by_key(|a| a.id.0);
        for (a, b) in expected.iter().zip(decoded.iter()) {
            assert_dbg_eq!(a, b);
        }
    });
}

#[test]
fn db_round_trip_events_all_variants() {
    // All 8 variants, with Option fields exercised both Some and None.
    let samples: Vec<Event> = vec![
        Event::CompletionClaimed {
            claim_id: ClaimId(1),
            squire: UserId(100),
            quest_id: QuestId(10),
            on: Date(42),
            at: Timestamp(1_000),
        },
        Event::CompletionApproved {
            claim_id: ClaimId(2),
            squire: UserId(100),
            actor: Some(UserId(1)),
            points: 5,
            at: Timestamp(2_000),
        },
        Event::CompletionApproved {
            claim_id: ClaimId(3),
            squire: UserId(100),
            actor: None, // auto-approve
            points: 0,
            at: Timestamp(2_100),
        },
        Event::CompletionRejected {
            claim_id: ClaimId(4),
            squire: UserId(100),
            actor: Some(UserId(1)),
            reason: Some("not done".into()),
            at: Timestamp(3_000),
        },
        Event::CompletionRejected {
            claim_id: ClaimId(5),
            squire: UserId(100),
            actor: None,
            reason: None,
            at: Timestamp(3_100),
        },
        Event::ItemRedeemed {
            request_id: Some(RequestId(7)),
            command_id: None,
            squire: UserId(100),
            actor: Some(UserId(1)),
            item_id: ItemId(20),
            cost: 100,
            at: Timestamp(4_000),
        },
        Event::ItemRedeemed {
            request_id: None,
            command_id: Some(CommandId(8)),
            squire: UserId(100),
            actor: None,
            item_id: ItemId(21),
            cost: 5,
            at: Timestamp(4_100),
        },
        Event::AchievementUnlocked {
            squire: UserId(100),
            id: AchievementId(30),
            bonus: 50,
            at: Timestamp(5_000),
        },
        Event::PointsAdjusted {
            command_id: CommandId(9),
            squire: UserId(100),
            actor: Some(UserId(1)),
            amount: -25,
            reason: "correction".into(),
            at: Timestamp(6_000),
        },
        Event::RedemptionRequested {
            request_id: RequestId(11),
            squire: UserId(100),
            item_id: ItemId(20),
            at: Timestamp(7_000),
        },
        Event::RedemptionRejected {
            request_id: RequestId(12),
            squire: UserId(100),
            actor: Some(UserId(1)),
            reason: Some("nope".into()),
            at: Timestamp(8_000),
        },
        Event::RedemptionRejected {
            request_id: RequestId(13),
            squire: UserId(100),
            actor: None,
            reason: None,
            at: Timestamp(8_100),
        },
    ];

    each_backend(|conn| {
        truncate_all(conn);
        for (i, ev) in samples.iter().enumerate() {
            let row = EventRow::from_event(i as i64 + 1, ev);
            diesel::insert_into(events::table)
                .values(&row)
                .execute(conn)
                .expect("insert event");
        }
        let rows: Vec<EventRow> = events::table
            .select(EventRow::as_select())
            .order(events::seq.asc())
            .load(conn)
            .expect("load events");
        let decoded: Vec<Event> = rows.iter().map(|r| r.to_event().expect("decode")).collect();
        assert_eq!(decoded.len(), samples.len());
        for (a, b) in samples.iter().zip(decoded.iter()) {
            assert_dbg_eq!(a, b);
        }
    });
}
