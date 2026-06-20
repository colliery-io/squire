//! T-0002 authoring tests: Define/Archive × Quest/Item/Achievement validation & Change emission.

use domain_core::testkit::*;
use domain_core::*;
use std::collections::BTreeSet;

fn squire(id: u128) -> User {
    User { id: UserId(id), role: Role::Squire, display_name: format!("S{id}"), active: true }
}
fn knight(id: u128) -> User {
    User { id: UserId(id), role: Role::Knight, display_name: format!("K{id}"), active: true }
}
fn base_repo() -> InMemoryRepository {
    let mut r = InMemoryRepository::new();
    r.seed(&[Change::PutUser(squire(1)), Change::PutUser(knight(2))]);
    r
}
fn daily_quest(id: u128, assignment: Assignment) -> Quest {
    Quest {
        id: QuestId(id),
        title: "Q".into(),
        description: None,
        category: None,
        reward: 10,
        cash: 0,
        cadence: Cadence::Recurring(Schedule::Daily),
        assignment,
        completion: Completion::EachAssignee,
        auto_approve: false,
        repeatable_within_day: false,
        active: true,
        icon: None,
    }
}
fn squires(ids: &[u128]) -> Assignment {
    Assignment::Squires(ids.iter().map(|i| UserId(*i)).collect::<BTreeSet<_>>())
}
fn eng() -> DomainEngine {
    DomainEngine
}
fn clk() -> FakeClock {
    FakeClock::at(Date(0), Timestamp(0))
}

#[test]
fn define_quest_all_squires_emits_put() {
    let snap = base_repo().snapshot();
    let changes = eng()
        .handle(&snap, Command::DefineQuest(daily_quest(10, Assignment::AllSquires)), &clk())
        .unwrap();
    assert!(matches!(changes.as_slice(), [Change::PutQuest(q)] if q.id == QuestId(10)));
}

#[test]
fn define_quest_empty_assignment_rejected() {
    let snap = base_repo().snapshot();
    let q = daily_quest(10, squires(&[]));
    assert!(matches!(
        eng().handle(&snap, Command::DefineQuest(q), &clk()),
        Err(DomainError::InvalidDefinition)
    ));
}

#[test]
fn define_quest_unknown_assignee_is_user_not_found() {
    let snap = base_repo().snapshot();
    let q = daily_quest(10, squires(&[99]));
    assert!(matches!(
        eng().handle(&snap, Command::DefineQuest(q), &clk()),
        Err(DomainError::UserNotFound)
    ));
}

#[test]
fn define_quest_knight_assignee_is_not_a_squire() {
    let snap = base_repo().snapshot();
    let q = daily_quest(10, squires(&[2])); // 2 is a Knight
    assert!(matches!(
        eng().handle(&snap, Command::DefineQuest(q), &clk()),
        Err(DomainError::NotASquire)
    ));
}

#[test]
fn define_quest_degenerate_schedules_rejected() {
    let snap = base_repo().snapshot();
    let mut weekly = daily_quest(10, Assignment::AllSquires);
    weekly.cadence = Cadence::Recurring(Schedule::Weekly { days: BTreeSet::new() });
    assert!(matches!(
        eng().handle(&snap, Command::DefineQuest(weekly), &clk()),
        Err(DomainError::InvalidDefinition)
    ));
    let mut every0 = daily_quest(11, Assignment::AllSquires);
    every0.cadence = Cadence::Recurring(Schedule::EveryNDays { n: 0, anchor: Date(0) });
    assert!(matches!(
        eng().handle(&snap, Command::DefineQuest(every0), &clk()),
        Err(DomainError::InvalidDefinition)
    ));
}

#[test]
fn archive_quest_unknown_then_known() {
    let mut repo = base_repo();
    let snap = repo.snapshot();
    assert!(matches!(
        eng().handle(&snap, Command::ArchiveQuest(QuestId(404)), &clk()),
        Err(DomainError::QuestNotFound)
    ));
    repo.seed(&[Change::PutQuest(daily_quest(10, Assignment::AllSquires))]);
    let snap = repo.snapshot();
    let changes = eng().handle(&snap, Command::ArchiveQuest(QuestId(10)), &clk()).unwrap();
    assert!(matches!(changes.as_slice(), [Change::SetQuestActive(QuestId(10), false)]));
}

#[test]
fn reward_edit_upserts_definition() {
    let mut repo = base_repo();
    repo.seed(&[Change::PutQuest(daily_quest(10, Assignment::AllSquires))]);
    let snap = repo.snapshot();
    let mut edited = daily_quest(10, Assignment::AllSquires);
    edited.reward = 15;
    let changes = eng().handle(&snap, Command::DefineQuest(edited), &clk()).unwrap();
    repo.apply(Some(UserId(2)), &changes).unwrap();
    assert_eq!(repo.quests.len(), 1, "edit upserts, not appends");
    assert_eq!(repo.quests[0].reward, 15);
}

#[test]
fn define_item_gate_must_exist() {
    let snap = base_repo().snapshot();
    let gated = RedeemableItem {
        id: ItemId(1),
        name: "Toy".into(),
        description: None,
        cost: 5,
        gate: Some(AchievementId(7)),
        availability: Availability::Once,
        active: true,
        icon: None,
    };
    assert!(matches!(
        eng().handle(&snap, Command::DefineItem(gated), &clk()),
        Err(DomainError::AchievementNotFound)
    ));
}

#[test]
fn define_item_ungated_ok() {
    let snap = base_repo().snapshot();
    let item = RedeemableItem {
        id: ItemId(1),
        name: "Screen time".into(),
        description: None,
        cost: 5,
        gate: None,
        availability: Availability::Repeatable,
        active: true,
        icon: None,
    };
    let changes = eng().handle(&snap, Command::DefineItem(item), &clk()).unwrap();
    assert!(matches!(changes.as_slice(), [Change::PutItem(i)] if i.id == ItemId(1)));
}

#[test]
fn define_achievement_scope_and_criterion_validation() {
    let mut repo = base_repo();
    repo.seed(&[Change::PutQuest(daily_quest(10, Assignment::AllSquires))]);
    let snap = repo.snapshot();

    let bad_scope = Achievement {
        id: AchievementId(1),
        name: "A".into(),
        description: None,
        criterion: Criterion::Streak {
            scope: Scope::Quest(QuestId(404)),
            length: 3,
            basis: StreakBasis::ScheduledOccurrences,
        },
        bonus_points: 5,
        active: true,
    };
    assert!(matches!(
        eng().handle(&snap, Command::DefineAchievement(bad_scope), &clk()),
        Err(DomainError::QuestNotFound)
    ));

    let zero_len = Achievement {
        id: AchievementId(2),
        name: "A".into(),
        description: None,
        criterion: Criterion::Streak {
            scope: Scope::Any,
            length: 0,
            basis: StreakBasis::CalendarDays,
        },
        bonus_points: 5,
        active: true,
    };
    assert!(matches!(
        eng().handle(&snap, Command::DefineAchievement(zero_len), &clk()),
        Err(DomainError::InvalidDefinition)
    ));

    // A blank Category scope is meaningless → rejected (SQUIRE-T-0071).
    let empty_cat = Achievement {
        id: AchievementId(4),
        name: "A".into(),
        description: None,
        criterion: Criterion::TotalCompletions { scope: Scope::Category(Category("  ".into())), count: 5 },
        bonus_points: 5,
        active: true,
    };
    assert!(
        matches!(
            eng().handle(&snap, Command::DefineAchievement(empty_cat), &clk()),
            Err(DomainError::InvalidDefinition)
        ),
        "a blank Category scope must be rejected",
    );

    let ok = Achievement {
        id: AchievementId(3),
        name: "A".into(),
        description: None,
        criterion: Criterion::Streak {
            scope: Scope::Quest(QuestId(10)),
            length: 3,
            basis: StreakBasis::ScheduledOccurrences,
        },
        bonus_points: 5,
        active: true,
    };
    let changes = eng().handle(&snap, Command::DefineAchievement(ok), &clk()).unwrap();
    assert!(matches!(changes.as_slice(), [Change::PutAchievement(a)] if a.id == AchievementId(3)));
}

#[test]
fn archive_item_and_achievement_unknown() {
    let snap = base_repo().snapshot();
    assert!(matches!(
        eng().handle(&snap, Command::ArchiveItem(ItemId(404)), &clk()),
        Err(DomainError::ItemNotFound)
    ));
    assert!(matches!(
        eng().handle(&snap, Command::ArchiveAchievement(AchievementId(404)), &clk()),
        Err(DomainError::AchievementNotFound)
    ));
}
