//! T-0006 streaks & achievements: ScheduledOccurrences vs CalendarDays streaks (AC-4),
//! pending-today forgiveness, repeatable collapse, sticky per-Squire unlock emission via the
//! real approve flow (bonus to balance + gated item unlock), and `streak_view` derivation.

use domain_core::testkit::*;
use domain_core::*;
use std::collections::BTreeSet;

fn squire(id: u128) -> User {
    User { id: UserId(id), role: Role::Squire, display_name: format!("S{id}"), active: true }
}
fn knight(id: u128) -> User {
    User { id: UserId(id), role: Role::Knight, display_name: format!("K{id}"), active: true }
}

/// A daily, EachAssignee quest assigned to all Squires, auto-approve toggleable.
fn daily(id: u128, reward: Points, auto: bool, repeatable: bool, category: Option<&str>) -> Quest {
    Quest {
        id: QuestId(id),
        title: "Q".into(),
        description: None,
        category: category.map(|c| Category(c.into())),
        reward,
        cadence: Cadence::Recurring(Schedule::Daily),
        assignment: Assignment::AllSquires,
        completion: Completion::EachAssignee,
        auto_approve: auto,
        repeatable_within_day: repeatable,
        active: true,
        icon: None,
    }
}

/// A Mon/Wed/Fri weekly quest (Date(0) = Monday convention).
fn mwf(id: u128, reward: Points, auto: bool) -> Quest {
    let mut days = BTreeSet::new();
    days.insert(Weekday::Mon);
    days.insert(Weekday::Wed);
    days.insert(Weekday::Fri);
    Quest {
        id: QuestId(id),
        title: "MWF".into(),
        description: None,
        category: None,
        reward,
        cadence: Cadence::Recurring(Schedule::Weekly { days }),
        assignment: Assignment::AllSquires,
        completion: Completion::EachAssignee,
        auto_approve: auto,
        repeatable_within_day: false,
        active: true,
        icon: None,
    }
}

fn achievement(id: u128, criterion: Criterion, bonus: Points) -> Achievement {
    Achievement { id: AchievementId(id), name: "A".into(), description: None, criterion, bonus_points: bonus, active: true }
}
fn item(id: u128, cost: Points, gate: Option<AchievementId>) -> RedeemableItem {
    RedeemableItem { id: ItemId(id), name: "I".into(), description: None, cost, gate, availability: Availability::Repeatable, active: true, icon: None }
}

fn repo() -> InMemoryRepository {
    let mut r = InMemoryRepository::new();
    r.seed(&[Change::PutUser(squire(1)), Change::PutUser(squire(3)), Change::PutUser(knight(2))]);
    r
}

/// Run a command through the engine and apply any resulting Changes (by = Knight 2).
fn run(repo: &mut InMemoryRepository, cmd: Command) -> Result<Vec<Change>, DomainError> {
    let snap = repo.snapshot();
    let r = DomainEngine.handle(&snap, cmd, &FakeClock::at(Date(0), Timestamp(1)));
    if let Ok(changes) = &r {
        repo.apply(Some(UserId(2)), changes).unwrap();
    }
    r
}

fn submit(claim: u128, squire: u128, q: u128, day: i32) -> Command {
    Command::SubmitClaim { claim_id: ClaimId(claim), squire: UserId(squire), quest_id: QuestId(q), on: Date(day) }
}
fn approve(claim: u128) -> Command {
    Command::ReviewClaim { actor: UserId(2), claim_id: ClaimId(claim), decision: Decision::Approve }
}

/// Submit + approve one completion of quest `q` for `squire` on `day` (manual review).
fn complete(repo: &mut InMemoryRepository, claim: u128, squire: u128, q: u128, day: i32) {
    run(repo, submit(claim, squire, q, day)).unwrap();
    run(repo, approve(claim)).unwrap();
}

fn bal(repo: &InMemoryRepository, squire: u128) -> i64 {
    Proj::balance(&repo.snapshot(), UserId(squire))
}
fn unlock_count(repo: &InMemoryRepository, squire: u128, id: u128) -> usize {
    repo.events
        .iter()
        .filter(|e| matches!(e, Event::AchievementUnlocked { squire: s, id: a, .. } if *s == UserId(squire) && *a == AchievementId(id)))
        .count()
}

// ── streaks ──────────────────────────────────────────────────────────────────

#[test]
fn scheduled_streak_skips_weekend_gaps_ac4() {
    // Mon/Wed/Fri quest completed on Mon(0), Wed(2), Fri(4) reads 3 despite the weekend.
    let mut r = repo();
    r.seed(&[Change::PutQuest(mwf(10, 5, false))]);
    complete(&mut r, 1, 1, 10, 0); // Mon
    complete(&mut r, 2, 1, 10, 2); // Wed
    complete(&mut r, 3, 1, 10, 4); // Fri
    let snap = r.snapshot();
    let streak = Proj::current_streak(&snap, UserId(1), &Scope::Quest(QuestId(10)), StreakBasis::ScheduledOccurrences, Date(4));
    assert_eq!(streak, 3, "AC-4: M/W/F done 3 scheduled days = 3");
}

#[test]
fn scheduled_streak_missed_day_resets() {
    // Skip Wed(2); the streak as of Fri only counts back to Fri (the Wed gap breaks it).
    let mut r = repo();
    r.seed(&[Change::PutQuest(mwf(10, 5, false))]);
    complete(&mut r, 1, 1, 10, 0); // Mon
    complete(&mut r, 3, 1, 10, 4); // Fri (Wed skipped)
    let snap = r.snapshot();
    let streak = Proj::current_streak(&snap, UserId(1), &Scope::Quest(QuestId(10)), StreakBasis::ScheduledOccurrences, Date(4));
    assert_eq!(streak, 1, "a missed scheduled occurrence resets the run");
}

#[test]
fn scheduled_streak_pending_today_does_not_break() {
    // Done Mon(0) & Wed(2); Fri(4) not yet done — as of Fri the streak is still 2.
    let mut r = repo();
    r.seed(&[Change::PutQuest(mwf(10, 5, false))]);
    complete(&mut r, 1, 1, 10, 0);
    complete(&mut r, 2, 1, 10, 2);
    let snap = r.snapshot();
    let streak = Proj::current_streak(&snap, UserId(1), &Scope::Quest(QuestId(10)), StreakBasis::ScheduledOccurrences, Date(4));
    assert_eq!(streak, 2, "a not-yet-done occurrence today is forgiven, not a break");
}

#[test]
fn scheduled_streak_dated_one_off_is_one_when_completed() {
    let mut r = repo();
    let mut q = daily(10, 5, false, false, None);
    q.cadence = Cadence::OneOff { due: Some(Date(3)) };
    r.seed(&[Change::PutQuest(q)]);
    complete(&mut r, 1, 1, 10, 3);
    let snap = r.snapshot();
    assert_eq!(
        Proj::current_streak(&snap, UserId(1), &Scope::Quest(QuestId(10)), StreakBasis::ScheduledOccurrences, Date(3)),
        1
    );
}

#[test]
fn calendar_streak_counts_consecutive_days() {
    let mut r = repo();
    r.seed(&[Change::PutQuest(daily(10, 5, false, false, Some("chores")))]);
    complete(&mut r, 1, 1, 10, 0);
    complete(&mut r, 2, 1, 10, 1);
    complete(&mut r, 3, 1, 10, 2);
    let snap = r.snapshot();
    assert_eq!(
        Proj::current_streak(&snap, UserId(1), &Scope::Category(Category("chores".into())), StreakBasis::CalendarDays, Date(2)),
        3
    );
    // A gap (day 3 missed) means as-of day 4 the run is 0 (nothing on day 4, nothing on 3).
    assert_eq!(
        Proj::current_streak(&snap, UserId(1), &Scope::Category(Category("chores".into())), StreakBasis::CalendarDays, Date(4)),
        0
    );
}

#[test]
fn calendar_streak_repeatable_counts_once_per_day() {
    let mut r = repo();
    r.seed(&[Change::PutQuest(daily(10, 5, false, true, None))]);
    // Two approved completions same day (repeatable) — counts as one day.
    complete(&mut r, 1, 1, 10, 0);
    complete(&mut r, 2, 1, 10, 0);
    complete(&mut r, 3, 1, 10, 1);
    let snap = r.snapshot();
    assert_eq!(
        Proj::current_streak(&snap, UserId(1), &Scope::Any, StreakBasis::CalendarDays, Date(1)),
        2,
        "a repeatable quest contributes once/day"
    );
}

#[test]
fn streaks_are_per_squire() {
    let mut r = repo();
    r.seed(&[Change::PutQuest(daily(10, 5, false, false, None))]);
    complete(&mut r, 1, 1, 10, 0);
    complete(&mut r, 2, 1, 10, 1);
    complete(&mut r, 3, 3, 10, 0); // Squire 3 only one day
    let snap = r.snapshot();
    assert_eq!(Proj::current_streak(&snap, UserId(1), &Scope::Any, StreakBasis::CalendarDays, Date(1)), 2);
    assert_eq!(Proj::current_streak(&snap, UserId(3), &Scope::Any, StreakBasis::CalendarDays, Date(1)), 1);
}

// ── achievement unlock emission ───────────────────────────────────────────────

#[test]
fn streak_criterion_unlocks_once_and_is_sticky() {
    let mut r = repo();
    r.seed(&[
        Change::PutQuest(daily(10, 5, false, false, None)),
        Change::PutAchievement(achievement(
            7,
            Criterion::Streak { scope: Scope::Quest(QuestId(10)), length: 3, basis: StreakBasis::ScheduledOccurrences },
            50,
        )),
    ]);
    complete(&mut r, 1, 1, 10, 0);
    complete(&mut r, 2, 1, 10, 1);
    assert_eq!(unlock_count(&r, 1, 7), 0, "not yet at length 3");
    complete(&mut r, 3, 1, 10, 2); // third scheduled day → unlock
    assert_eq!(unlock_count(&r, 1, 7), 1, "unlocked exactly once on first satisfaction");
    assert!(Proj::is_unlocked(&r.snapshot(), UserId(1), AchievementId(7)));
    // Sticky: a fourth completion doesn't re-emit.
    complete(&mut r, 4, 1, 10, 3);
    assert_eq!(unlock_count(&r, 1, 7), 1, "sticky — no re-emit on later completions");
}

#[test]
fn unlock_awards_bonus_to_that_squire_only() {
    let mut r = repo();
    r.seed(&[
        Change::PutQuest(daily(10, 5, false, false, None)),
        Change::PutAchievement(achievement(
            7,
            Criterion::TotalCompletions { scope: Scope::Any, count: 2 },
            40,
        )),
    ]);
    complete(&mut r, 1, 1, 10, 0);
    complete(&mut r, 2, 1, 10, 1); // 2 completions → unlock, +40 bonus
    assert_eq!(unlock_count(&r, 1, 7), 1);
    // balance = 5 + 5 + 40 bonus = 50.
    assert_eq!(bal(&r, 1), 50);
    assert_eq!(unlock_count(&r, 3, 7), 0, "Squire 3 hasn't met it");
    assert_eq!(bal(&r, 3), 0, "no bonus for Squire 3");
}

#[test]
fn points_earned_criterion_uses_earned_total_not_balance() {
    let mut r = repo();
    r.seed(&[
        Change::PutQuest(daily(10, 30, false, false, None)),
        Change::PutItem(item(1, 10, None)),
        Change::PutAchievement(achievement(7, Criterion::PointsEarned { total: 50 }, 0)),
    ]);
    complete(&mut r, 1, 1, 10, 0); // earned 30
    assert_eq!(unlock_count(&r, 1, 7), 0);
    complete(&mut r, 2, 1, 10, 1); // earned 60 total → unlock (even though spends could lower balance)
    assert_eq!(unlock_count(&r, 1, 7), 1, "PointsEarned tracks lifetime earnings");
}

#[test]
fn points_earned_unlock_cascades_via_bonus() {
    // First achievement (TotalCompletions=1, bonus 100) pushes earned over the PointsEarned=100
    // line in the same approval — the cascade unlocks both at once.
    let mut r = repo();
    r.seed(&[
        Change::PutQuest(daily(10, 5, false, false, None)),
        Change::PutAchievement(achievement(7, Criterion::TotalCompletions { scope: Scope::Any, count: 1 }, 100)),
        Change::PutAchievement(achievement(8, Criterion::PointsEarned { total: 100 }, 0)),
    ]);
    complete(&mut r, 1, 1, 10, 0); // earns 5, then +100 bonus → earned 105 ≥ 100
    assert_eq!(unlock_count(&r, 1, 7), 1);
    assert_eq!(unlock_count(&r, 1, 8), 1, "bonus cascade unlocks the PointsEarned achievement");
}

#[test]
fn unlock_flips_gated_item_to_redeemable_for_that_squire() {
    let mut r = repo();
    r.seed(&[
        Change::PutQuest(daily(10, 100, false, false, None)),
        Change::PutAchievement(achievement(7, Criterion::TotalCompletions { scope: Scope::Any, count: 1 }, 0)),
        Change::PutItem(item(1, 10, Some(AchievementId(7)))),
    ]);
    // Before: gated.
    assert!(matches!(
        Proj::can_redeem(&r.snapshot(), UserId(1), ItemId(1), Date(0)),
        Err(Blocked::AchievementLocked { id: AchievementId(7) })
    ));
    complete(&mut r, 1, 1, 10, 0); // unlock achievement 7 for Squire 1
    assert!(Proj::can_redeem(&r.snapshot(), UserId(1), ItemId(1), Date(0)).is_ok(), "gate lifted for Squire 1");
    // Per-Squire: Squire 3 still locked.
    assert!(matches!(
        Proj::can_redeem(&r.snapshot(), UserId(3), ItemId(1), Date(0)),
        Err(Blocked::AchievementLocked { .. })
    ));
}

#[test]
fn auto_approve_emits_unlock_in_one_command() {
    let mut r = repo();
    r.seed(&[
        Change::PutQuest(daily(10, 5, true, false, None)), // auto-approve
        Change::PutAchievement(achievement(7, Criterion::TotalCompletions { scope: Scope::Any, count: 1 }, 25)),
    ]);
    // A single submit auto-approves AND unlocks.
    run(&mut r, submit(1, 1, 10, 0)).unwrap();
    assert_eq!(unlock_count(&r, 1, 7), 1);
    assert_eq!(bal(&r, 1), 30, "5 reward + 25 bonus");
}

// ── streak_view ───────────────────────────────────────────────────────────────

#[test]
fn streak_view_current_best_alive_next_milestone() {
    let mut r = repo();
    r.seed(&[
        Change::PutQuest(daily(10, 5, false, false, None)),
        Change::PutAchievement(achievement(7, Criterion::Streak { scope: Scope::Any, length: 5, basis: StreakBasis::CalendarDays }, 0)),
    ]);
    // Days 0,1,2 then a gap, then day 4 alone. Best run = 3 (days 0-2); current as-of day 4 = 1.
    complete(&mut r, 1, 1, 10, 0);
    complete(&mut r, 2, 1, 10, 1);
    complete(&mut r, 3, 1, 10, 2);
    complete(&mut r, 4, 1, 10, 4);
    let snap = r.snapshot();
    let (current, best, alive, next) = streak_view(&snap, UserId(1), &Scope::Any, StreakBasis::CalendarDays, Date(4));
    assert_eq!(current, 1, "only day 4 in the current run");
    assert_eq!(best, 3, "high-water mark over days 0-2");
    assert!(alive, "current run is non-zero");
    assert_eq!(next, Some(5), "next milestone is the length-5 achievement");
}

#[test]
fn streak_view_dead_when_lapsed() {
    let mut r = repo();
    r.seed(&[Change::PutQuest(daily(10, 5, false, false, None))]);
    complete(&mut r, 1, 1, 10, 0);
    complete(&mut r, 2, 1, 10, 1);
    let snap = r.snapshot();
    // As of day 3, the last completion was day 1 → lapsed.
    let (current, best, alive, next) = streak_view(&snap, UserId(1), &Scope::Any, StreakBasis::CalendarDays, Date(3));
    assert_eq!(current, 0);
    assert_eq!(best, 2);
    assert!(!alive, "alive=false once an occurrence has lapsed");
    assert_eq!(next, None, "no active streak achievement in scope");
}
