//! T-0004 scheduling & due-logic tests: cadence resolution (Daily/Weekly/EveryNDays/OneOff),
//! assignment gating, Race-open gating, per-Squire already-satisfied subtraction, and the
//! `quest_status` derivation (incl. Race `TakenByOther`). Date(0) is a Monday by convention.

use domain_core::testkit::*;
use domain_core::*;
use std::collections::BTreeSet;

fn squire(id: u128) -> User {
    User {
        id: UserId(id),
        role: Role::Squire,
        display_name: format!("S{id}"),
        active: true,
    }
}
fn knight(id: u128) -> User {
    User {
        id: UserId(id),
        role: Role::Knight,
        display_name: format!("K{id}"),
        active: true,
    }
}
fn base() -> Quest {
    Quest {
        id: QuestId(0),
        title: "Q".into(),
        description: None,
        category: None,
        reward: 10,
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
fn with_id(mut q: Quest, id: u128) -> Quest {
    q.id = QuestId(id);
    q
}

fn repo() -> InMemoryRepository {
    let mut r = InMemoryRepository::new();
    r.seed(&[
        Change::PutUser(squire(1)),
        Change::PutUser(squire(3)),
        Change::PutUser(knight(2)),
    ]);
    r
}
fn run(repo: &mut InMemoryRepository, cmd: Command) -> Result<Vec<Change>, DomainError> {
    let snap = repo.snapshot();
    let r = DomainEngine.handle(&snap, cmd, &FakeClock::at(Date(0), Timestamp(0)));
    if let Ok(changes) = &r {
        repo.apply(Some(UserId(2)), changes).unwrap();
    }
    r
}
fn due(repo: &InMemoryRepository, squire: u128, day: i32) -> Vec<QuestId> {
    Proj::quests_due(&repo.snapshot(), UserId(squire), Date(day))
}
fn is_due(repo: &InMemoryRepository, squire: u128, day: i32, q: u128) -> bool {
    due(repo, squire, day).contains(&QuestId(q))
}

#[test]
fn daily_is_due_every_day() {
    let mut r = repo();
    r.seed(&[Change::PutQuest(with_id(base(), 10))]);
    assert!(is_due(&r, 1, 0, 10));
    assert!(is_due(&r, 1, 1, 10));
}

#[test]
fn weekly_is_due_only_on_listed_weekdays() {
    let mut r = repo();
    let mut q = with_id(base(), 14);
    q.cadence = Cadence::Recurring(Schedule::Weekly {
        days: [Weekday::Mon, Weekday::Wed, Weekday::Fri]
            .into_iter()
            .collect::<BTreeSet<_>>(),
    });
    r.seed(&[Change::PutQuest(q)]);
    assert!(is_due(&r, 1, 0, 14), "Mon");
    assert!(!is_due(&r, 1, 1, 14), "Tue");
    assert!(is_due(&r, 1, 2, 14), "Wed");
    assert!(!is_due(&r, 1, 5, 14), "Sat");
}

#[test]
fn every_n_days_from_anchor() {
    let mut r = repo();
    let mut q = with_id(base(), 15);
    q.cadence = Cadence::Recurring(Schedule::EveryNDays {
        n: 3,
        anchor: Date(0),
    });
    r.seed(&[Change::PutQuest(q)]);
    assert!(is_due(&r, 1, 0, 15));
    assert!(!is_due(&r, 1, 1, 15));
    assert!(is_due(&r, 1, 3, 15));
    assert!(is_due(&r, 1, 6, 15));
    assert!(!is_due(&r, 1, -3, 15), "before the anchor is not due");
}

#[test]
fn oneoff_with_and_without_due_date() {
    let mut r = repo();
    let mut dated = with_id(base(), 16);
    dated.cadence = Cadence::OneOff { due: Some(Date(5)) };
    let mut undated = with_id(base(), 17);
    undated.cadence = Cadence::OneOff { due: None };
    r.seed(&[Change::PutQuest(dated), Change::PutQuest(undated)]);
    assert!(
        is_due(&r, 1, 5, 16) && !is_due(&r, 1, 4, 16),
        "dated one-off due only on its day"
    );
    assert!(
        is_due(&r, 1, 0, 17) && is_due(&r, 1, 9, 17),
        "undated one-off due any day (until done)"
    );
}

#[test]
fn assignment_filters_the_due_list() {
    let mut r = repo();
    let mut q = with_id(base(), 18);
    q.assignment = Assignment::Squires([UserId(1)].into_iter().collect::<BTreeSet<_>>());
    r.seed(&[Change::PutQuest(q)]);
    assert!(is_due(&r, 1, 0, 18));
    assert!(!is_due(&r, 3, 0, 18), "Squire 3 is not assigned");
}

#[test]
fn approved_completion_removes_today_but_not_tomorrow() {
    let mut r = repo();
    r.seed(&[Change::PutQuest(with_id(base(), 10))]);
    run(
        &mut r,
        Command::SubmitClaim {
            claim_id: ClaimId(1),
            squire: UserId(1),
            quest_id: QuestId(10),
            on: Date(0),
        },
    )
    .unwrap();
    run(
        &mut r,
        Command::ReviewClaim {
            actor: UserId(2),
            claim_id: ClaimId(1),
            decision: Decision::Approve,
        },
    )
    .unwrap();
    assert!(!is_due(&r, 1, 0, 10), "completed today");
    assert!(is_due(&r, 1, 1, 10), "still due the next day");
    assert!(
        is_due(&r, 3, 0, 10),
        "the other Squire's occurrence is independent"
    );
}

#[test]
fn pending_claim_drops_from_due_but_status_is_pending() {
    let mut r = repo();
    r.seed(&[Change::PutQuest(with_id(base(), 10))]);
    run(
        &mut r,
        Command::SubmitClaim {
            claim_id: ClaimId(1),
            squire: UserId(1),
            quest_id: QuestId(10),
            on: Date(0),
        },
    )
    .unwrap();
    assert!(
        !is_due(&r, 1, 0, 10),
        "a pending non-repeatable claim is not re-claimable"
    );
    let snap = r.snapshot();
    let q = snap.quests.iter().find(|q| q.id == QuestId(10)).unwrap();
    assert!(matches!(
        quest_status(&snap, UserId(1), q, Date(0)),
        QuestStatus::Pending
    ));
}

#[test]
fn repeatable_stays_due_while_pending() {
    let mut r = repo();
    let mut q = with_id(base(), 11);
    q.repeatable_within_day = true;
    r.seed(&[Change::PutQuest(q)]);
    run(
        &mut r,
        Command::SubmitClaim {
            claim_id: ClaimId(1),
            squire: UserId(1),
            quest_id: QuestId(11),
            on: Date(0),
        },
    )
    .unwrap();
    assert!(
        is_due(&r, 1, 0, 11),
        "repeatable quest can be claimed again"
    );
}

#[test]
fn race_due_until_closed_then_taken_by_other() {
    let mut r = repo();
    let mut q = with_id(base(), 12);
    q.completion = Completion::Race;
    q.reward = 20;
    r.seed(&[Change::PutQuest(q)]);
    assert!(
        is_due(&r, 1, 0, 12) && is_due(&r, 3, 0, 12),
        "open race is due for all assignees"
    );
    run(
        &mut r,
        Command::SubmitClaim {
            claim_id: ClaimId(1),
            squire: UserId(1),
            quest_id: QuestId(12),
            on: Date(0),
        },
    )
    .unwrap();
    run(
        &mut r,
        Command::ReviewClaim {
            actor: UserId(2),
            claim_id: ClaimId(1),
            decision: Decision::Approve,
        },
    )
    .unwrap();
    assert!(
        !is_due(&r, 1, 0, 12) && !is_due(&r, 3, 0, 12),
        "won race drops from every due list"
    );
    let snap = r.snapshot();
    let q = snap.quests.iter().find(|q| q.id == QuestId(12)).unwrap();
    assert!(matches!(
        quest_status(&snap, UserId(1), q, Date(0)),
        QuestStatus::CompletedToday
    ));
    assert!(matches!(
        quest_status(&snap, UserId(3), q, Date(0)),
        QuestStatus::TakenByOther
    ));
}

#[test]
fn archived_quest_is_never_due() {
    let mut r = repo();
    r.seed(&[Change::PutQuest(with_id(base(), 10))]);
    run(&mut r, Command::ArchiveQuest(QuestId(10))).unwrap();
    assert!(!is_due(&r, 1, 0, 10));
}

// ─── a due time is presentation, never a rule (SQUIRE-T-0142) ────────────────────────────────

/// **The pin.** A chore's `due_time` drives reminders and ordering; the engine must never read it.
///
/// Two quests identical but for the time must be due on the same days, carry the same status, and
/// score identically — including a claim made long "after" the due time, which is still a perfectly
/// good claim. If this test ever fails, a due time has quietly become a deadline, and that is a
/// product decision (and an ADR), not an implementation detail.
#[test]
fn due_time_changes_nothing_the_engine_does() {
    let mut r = repo();
    let plain = with_id(base(), 1);
    let timed = Quest {
        due_time: Some(7 * 60), // 07:00 — long past by the time the claim lands
        ..with_id(base(), 2)
    };
    r.seed(&[
        Change::PutQuest(plain.clone()),
        Change::PutQuest(timed.clone()),
    ]);

    // Due on exactly the same days.
    for day in 0..7 {
        assert_eq!(
            is_due(&r, 1, day, 1),
            is_due(&r, 1, day, 2),
            "a due time must not change which days a chore is due (day {day})",
        );
    }

    // The same status, before either is claimed.
    let snap = r.snapshot();
    assert_eq!(
        format!("{:?}", quest_status(&snap, UserId(1), &plain, Date(0))),
        format!("{:?}", quest_status(&snap, UserId(1), &timed, Date(0))),
    );

    // And a claim is accepted, and scores, identically — "late" is not a thing.
    let claim_plain = run(
        &mut r,
        Command::SubmitClaim {
            claim_id: ClaimId(1),
            squire: UserId(1),
            quest_id: QuestId(1),
            on: Date(0),
        },
    )
    .expect("a claim on an untimed chore");
    let claim_timed = run(
        &mut r,
        Command::SubmitClaim {
            claim_id: ClaimId(2),
            squire: UserId(1),
            quest_id: QuestId(2),
            on: Date(0),
        },
    )
    .expect("a claim after the due time is still a good claim");
    assert_eq!(
        claim_plain.len(),
        claim_timed.len(),
        "the same command must produce the same changes whether or not a time was set",
    );
}
