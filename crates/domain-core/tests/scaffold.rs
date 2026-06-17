//! T-0001 scaffold tests: the in-memory repository round-trips, the child-surface guard
//! enforces the trust boundary, dispatch reaches every command family, and a small
//! deterministic generative ("property") harness runs. Real per-command behaviour is
//! tested in T-0002..T-0006; here the handlers are stubs.

use domain_core::testkit::*;
use domain_core::*;

fn empty_snapshot() -> Snapshot {
    InMemoryRepository::new().snapshot()
}

fn clock() -> FakeClock {
    FakeClock::at(Date(0), Timestamp(0))
}

#[test]
fn apply_then_snapshot_round_trips() {
    let mut repo = InMemoryRepository::new();
    let kid = User {
        id: UserId(1),
        role: Role::Squire,
        display_name: "Kid".into(),
        active: true,
    };
    repo.apply(Some(UserId(9)), &[Change::PutUser(kid)]).unwrap();
    repo.apply(
        Some(UserId(2)),
        &[Change::Append(Event::PointsAdjusted {
            command_id: CommandId(1),
            squire: UserId(1),
            actor: Some(UserId(2)),
            amount: 5,
            reason: "seed".into(),
            at: Timestamp(0),
        })],
    )
    .unwrap();

    let snap = repo.snapshot();
    assert_eq!(snap.users.len(), 1);
    assert_eq!(snap.events.len(), 1);
    // `by` is recorded as a last-editor audit entry for the PutUser, but NOT for the Append
    // (an event already carries its own `actor`/`squire`) — ADR SQUIRE-A-0007.
    assert_eq!(repo.audit.len(), 1);
    assert_eq!(repo.audit[0].by, Some(UserId(9)));
}

#[test]
fn upsert_replaces_definition_by_id() {
    let mut repo = InMemoryRepository::new();
    let mut q = Quest {
        id: QuestId(7),
        title: "Trash".into(),
        description: None,
        category: None,
        reward: 10,
        cadence: Cadence::Recurring(Schedule::Daily),
        assignment: Assignment::AllSquires,
        completion: Completion::Race,
        auto_approve: false,
        repeatable_within_day: false,
        active: true,
        icon: None,
    };
    repo.apply(Some(UserId(2)), &[Change::PutQuest(q.clone())]).unwrap();
    q.reward = 15;
    repo.apply(Some(UserId(3)), &[Change::PutQuest(q.clone())]).unwrap();
    assert_eq!(repo.quests.len(), 1, "upsert, not append");
    assert_eq!(repo.quests[0].reward, 15);

    repo.apply(Some(UserId(2)), &[Change::SetQuestActive(QuestId(7), false)]).unwrap();
    assert!(!repo.quests[0].active, "archive flips active, never deletes");
}

#[test]
fn child_surface_guard_rejects_admin_commands() {
    let eng = DomainEngine;
    let snap = empty_snapshot();
    let admin = Command::AdjustPoints {
        command_id: CommandId(1),
        actor: UserId(2),
        squire: UserId(1),
        amount: 5,
        reason: "x".into(),
    };
    assert!(matches!(
        eng.handle_child(&snap, admin, &clock()),
        Err(DomainError::BadCommandForActor)
    ));
    assert!(!child_originable(&Command::RedeemItem {
        command_id: CommandId(1),
        actor: UserId(2),
        squire: UserId(1),
        item_id: ItemId(1),
    }));
}

#[test]
fn child_surface_guard_admits_child_commands() {
    let eng = DomainEngine;
    let snap = empty_snapshot();
    // The guard's job is to *admit* child commands (it may then fail downstream validation —
    // here UserNotFound on an empty snapshot — but it must NOT reject with BadCommandForActor).
    let r = eng.handle_child(
        &snap,
        Command::SubmitClaim {
            claim_id: ClaimId(1),
            squire: UserId(1),
            quest_id: QuestId(1),
            on: Date(0),
        },
        &clock(),
    );
    assert!(!matches!(r, Err(DomainError::BadCommandForActor)));
}

#[test]
fn handle_dispatches_every_command_family_without_panicking() {
    let eng = DomainEngine;
    let snap = empty_snapshot();
    let c = clock();
    let cmds = vec![
        Command::DefineQuest(Quest {
            id: QuestId(1),
            title: "q".into(),
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
        }),
        Command::ArchiveQuest(QuestId(1)),
        Command::SubmitClaim { claim_id: ClaimId(1), squire: UserId(1), quest_id: QuestId(1), on: Date(0) },
        Command::ReviewClaim { actor: UserId(2), claim_id: ClaimId(1), decision: Decision::Approve },
        Command::RequestRedemption { request_id: RequestId(1), squire: UserId(1), item_id: ItemId(1) },
        Command::AdjustPoints { command_id: CommandId(1), actor: UserId(2), squire: UserId(1), amount: 1, reason: "r".into() },
    ];
    for cmd in cmds {
        // The point is dispatch reaches a handler for each family without panicking. The
        // Ok/Err verdict depends on each family's (evolving) rules — covered by the
        // per-family test suites — so here we only assert the call returns.
        let _ = eng.handle(&snap, cmd, &c);
    }
}

/// Deterministic generative harness (std-only stand-in for proptest, which T-0007 may adopt
/// if the registry is reachable): applying N appends always yields N events.
#[test]
fn generative_append_count_is_preserved() {
    for n in 0..64u128 {
        let mut repo = InMemoryRepository::new();
        let changes: Vec<Change> = (0..n)
            .map(|i| {
                Change::Append(Event::CompletionClaimed {
                    claim_id: ClaimId(i),
                    squire: UserId(1),
                    quest_id: QuestId(1),
                    on: Date(0),
                    at: Timestamp(i as i64),
                })
            })
            .collect();
        repo.apply(None, &changes).unwrap();
        assert_eq!(repo.snapshot().events.len() as u128, n);
    }
}
