//! T-0003 claim & review tests: subject validation, dup/AlreadyClaimedToday, repeatable,
//! auto-approve, snapshot-at-approval, single-review, actor stamping, claim idempotency,
//! and the Race occurrence-resolution rule (first-approved-wins, reject-reopens).

use domain_core::testkit::*;
use domain_core::*;
use std::collections::BTreeSet;

fn squire(id: u128) -> User {
    User { id: UserId(id), role: Role::Squire, display_name: format!("S{id}"), active: true }
}
fn knight(id: u128) -> User {
    User { id: UserId(id), role: Role::Knight, display_name: format!("K{id}"), active: true }
}

struct QuestSpec {
    id: u128,
    reward: Points,
    assignment: Assignment,
    completion: Completion,
    auto_approve: bool,
    repeatable: bool,
}
fn quest(q: QuestSpec) -> Quest {
    Quest {
        id: QuestId(q.id),
        title: "Q".into(),
        description: None,
        category: None,
        reward: q.reward,
        cash: 0,
        cadence: Cadence::Recurring(Schedule::Daily),
        assignment: q.assignment,
        completion: q.completion,
        auto_approve: q.auto_approve,
        repeatable_within_day: q.repeatable,
        active: true,
        icon: None,
    }
}
fn each(id: u128, reward: Points, repeatable: bool, auto: bool) -> Quest {
    quest(QuestSpec { id, reward, assignment: Assignment::AllSquires, completion: Completion::EachAssignee, auto_approve: auto, repeatable })
}
fn race(id: u128, reward: Points) -> Quest {
    quest(QuestSpec { id, reward, assignment: Assignment::AllSquires, completion: Completion::Race, auto_approve: false, repeatable: false })
}
fn squires_set(ids: &[u128]) -> Assignment {
    Assignment::Squires(ids.iter().map(|i| UserId(*i)).collect::<BTreeSet<_>>())
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

/// Run a command through the engine and apply any resulting Changes (by = Knight 2).
fn run(repo: &mut InMemoryRepository, cmd: Command) -> Result<Vec<Change>, DomainError> {
    let snap = repo.snapshot();
    let r = DomainEngine.handle(&snap, cmd, &FakeClock::at(Date(0), Timestamp(0)));
    if let Ok(changes) = &r {
        repo.apply(Some(UserId(2)), changes).unwrap();
    }
    r
}

fn submit(claim: u128, squire: u128, q: u128, day: i32) -> Command {
    Command::SubmitClaim { claim_id: ClaimId(claim), squire: UserId(squire), quest_id: QuestId(q), on: Date(day) }
}
fn approve(actor: u128, claim: u128) -> Command {
    Command::ReviewClaim { actor: UserId(actor), claim_id: ClaimId(claim), decision: Decision::Approve }
}
fn reject(actor: u128, claim: u128) -> Command {
    Command::ReviewClaim { actor: UserId(actor), claim_id: ClaimId(claim), decision: Decision::Reject { reason: Some("nope".into()) } }
}

fn approved(repo: &InMemoryRepository, claim: u128) -> Option<(UserId, Option<UserId>, Points)> {
    repo.events.iter().find_map(|e| match e {
        Event::CompletionApproved { claim_id, squire, actor, points, .. } if *claim_id == ClaimId(claim) => {
            Some((*squire, *actor, *points))
        }
        _ => None,
    })
}

#[test]
fn submit_emits_completion_claimed_only() {
    let mut r = repo();
    r.seed(&[Change::PutQuest(each(10, 10, false, false))]);
    let changes = run(&mut r, submit(1, 1, 10, 0)).unwrap();
    assert!(matches!(
        changes.as_slice(),
        [Change::Append(Event::CompletionClaimed { squire: UserId(1), .. })]
    ));
    assert_eq!(approved(&r, 1), None, "a plain claim is zero-value until reviewed");
}

#[test]
fn submit_is_idempotent_on_claim_id() {
    let mut r = repo();
    r.seed(&[Change::PutQuest(each(10, 10, false, false))]);
    run(&mut r, submit(1, 1, 10, 0)).unwrap();
    let before = r.events.len();
    let again = run(&mut r, submit(1, 1, 10, 0)).unwrap();
    assert!(again.is_empty(), "replay of a seen claim_id is a no-op");
    assert_eq!(r.events.len(), before, "no duplicate CompletionClaimed");
}

#[test]
fn subject_validation() {
    let mut r = repo();
    r.seed(&[Change::PutQuest(each(10, 10, false, false))]);
    assert!(matches!(run(&mut r, submit(1, 99, 10, 0)), Err(DomainError::UserNotFound)));
    assert!(matches!(run(&mut r, submit(1, 2, 10, 0)), Err(DomainError::NotASquire))); // 2 = Knight
}

#[test]
fn non_assignee_rejected() {
    let mut r = repo();
    r.seed(&[Change::PutQuest(quest(QuestSpec {
        id: 10, reward: 10, assignment: squires_set(&[1]),
        completion: Completion::EachAssignee, auto_approve: false, repeatable: false,
    }))]);
    assert!(matches!(run(&mut r, submit(1, 3, 10, 0)), Err(DomainError::NotAssigned))); // 3 not assigned
    assert!(run(&mut r, submit(1, 1, 10, 0)).is_ok()); // 1 is assigned
}

#[test]
fn already_claimed_today_is_per_squire() {
    let mut r = repo();
    r.seed(&[Change::PutQuest(each(10, 10, false, false))]);
    run(&mut r, submit(1, 1, 10, 0)).unwrap();
    // same squire, same day, new claim_id → blocked
    assert!(matches!(run(&mut r, submit(2, 1, 10, 0)), Err(DomainError::AlreadyClaimedToday)));
    // different squire, same day → allowed
    assert!(run(&mut r, submit(3, 3, 10, 0)).is_ok());
    // same squire, different day → allowed
    assert!(run(&mut r, submit(4, 1, 10, 1)).is_ok());
}

#[test]
fn repeatable_within_day_allows_multiple() {
    let mut r = repo();
    r.seed(&[Change::PutQuest(each(11, 5, true, false))]);
    assert!(run(&mut r, submit(1, 1, 11, 0)).is_ok());
    assert!(run(&mut r, submit(2, 1, 11, 0)).is_ok(), "repeatable: a second same-day claim is allowed");
}

#[test]
fn auto_approve_credits_immediately_with_no_actor() {
    let mut r = repo();
    r.seed(&[Change::PutQuest(each(13, 7, false, true))]);
    run(&mut r, submit(1, 1, 13, 0)).unwrap();
    assert_eq!(approved(&r, 1), Some((UserId(1), None, 7)), "auto-approve: actor None, reward snapshotted");
}

#[test]
fn review_approve_then_reject_and_already_reviewed() {
    let mut r = repo();
    r.seed(&[Change::PutQuest(each(10, 10, false, false))]);
    run(&mut r, submit(1, 1, 10, 0)).unwrap();
    run(&mut r, approve(2, 1)).unwrap();
    assert_eq!(approved(&r, 1), Some((UserId(1), Some(UserId(2)), 10)), "actor = Knight, reward snapshotted");
    assert!(matches!(run(&mut r, approve(2, 1)), Err(DomainError::AlreadyReviewed)));
    assert!(matches!(run(&mut r, reject(2, 1)), Err(DomainError::AlreadyReviewed)));
    assert!(matches!(run(&mut r, approve(2, 404)), Err(DomainError::ClaimNotFound)));
}

#[test]
fn reject_lets_the_squire_claim_again() {
    let mut r = repo();
    r.seed(&[Change::PutQuest(each(10, 10, false, false))]);
    run(&mut r, submit(1, 1, 10, 0)).unwrap();
    run(&mut r, reject(2, 1)).unwrap();
    // rejected claim is not "live" → a fresh claim for the same (squire, quest, day) is allowed
    assert!(run(&mut r, submit(2, 1, 10, 0)).is_ok());
}

#[test]
fn snapshot_at_approval_is_immutable_under_later_edits() {
    let mut r = repo();
    r.seed(&[Change::PutQuest(each(10, 10, false, false))]);
    run(&mut r, submit(1, 1, 10, 0)).unwrap();
    run(&mut r, approve(2, 1)).unwrap();
    assert_eq!(approved(&r, 1).unwrap().2, 10);
    // Edit the reward 10 → 15.
    let mut edited = each(10, 15, false, false);
    edited.title = "Q".into();
    run(&mut r, Command::DefineQuest(edited)).unwrap();
    // The already-approved completion is unchanged (event log is immutable) — AC-7.
    assert_eq!(approved(&r, 1).unwrap().2, 10);
    // A fresh claim approved now snapshots the new reward.
    run(&mut r, submit(2, 3, 10, 0)).unwrap();
    run(&mut r, approve(2, 2)).unwrap();
    assert_eq!(approved(&r, 2).unwrap().2, 15);
}

#[test]
fn race_first_approved_wins_and_closes_occurrence() {
    let mut r = repo();
    r.seed(&[Change::PutQuest(race(12, 20))]);
    // Two assignees may both hold pending claims while the occurrence is open.
    assert!(run(&mut r, submit(1, 1, 12, 0)).is_ok());
    assert!(run(&mut r, submit(2, 3, 12, 0)).is_ok());
    // First approval wins and closes the occurrence.
    run(&mut r, approve(2, 1)).unwrap();
    assert_eq!(approved(&r, 1).unwrap().2, 20);
    // The other pending claim can no longer be approved.
    assert!(matches!(run(&mut r, approve(2, 2)), Err(DomainError::OccurrenceTaken)));
    // And no new claim can be submitted for the closed occurrence.
    assert!(matches!(run(&mut r, submit(9, 3, 12, 0)), Err(DomainError::OccurrenceTaken)));
}

#[test]
fn race_rejected_claim_reopens_the_occurrence() {
    let mut r = repo();
    r.seed(&[Change::PutQuest(race(12, 20))]);
    run(&mut r, submit(1, 1, 12, 0)).unwrap();
    // Same Squire can't double-claim a live race occurrence.
    assert!(matches!(run(&mut r, submit(2, 1, 12, 0)), Err(DomainError::AlreadyClaimedToday)));
    // Reject the first claim → occurrence reopens.
    run(&mut r, reject(2, 1)).unwrap();
    // Another assignee can now claim and win.
    run(&mut r, submit(3, 3, 12, 0)).unwrap();
    run(&mut r, approve(2, 3)).unwrap();
    assert_eq!(approved(&r, 3).unwrap().2, 20);
}

// ── Hardening (SQUIRE-T-0076) ──────────────────────────────────────────────────────────────────

/// An `Any`-scoped TotalCompletions achievement, unlocked once `count` completions are approved.
fn total_achievement(id: u128, count: u32, bonus: Points) -> Achievement {
    Achievement {
        id: AchievementId(id),
        name: "First chore".into(),
        description: None,
        criterion: Criterion::TotalCompletions { scope: Scope::Any, count },
        bonus_points: bonus,
        active: true,
    }
}

/// Approving a claim that completes an achievement criterion emits `AchievementUnlocked` and the
/// bonus lands on the balance — exercised end-to-end via the claim→approve path (not just
/// `achievements.rs`), so a regression in `approval_events` → `unlocks_after` is caught here.
#[test]
fn approving_a_claim_that_completes_a_criterion_unlocks_and_credits_bonus() {
    let mut r = repo();
    r.seed(&[
        Change::PutQuest(each(10, 10, false, false)),
        Change::PutAchievement(total_achievement(900, 1, 50)),
    ]);
    run(&mut r, submit(1, 1, 10, 0)).unwrap();
    run(&mut r, approve(2, 1)).unwrap();

    // The reward (10) plus the unlock bonus (50) are both on the balance.
    assert_eq!(Proj::balance(&r.snapshot(), UserId(1)), 60, "reward + achievement bonus");
    let unlocked = r.events.iter().any(|e| matches!(
        e, Event::AchievementUnlocked { squire, id, bonus, .. }
            if *squire == UserId(1) && *id == AchievementId(900) && *bonus == 50
    ));
    assert!(unlocked, "approval emits the AchievementUnlocked event");
}

/// A quest with real-money `cash` set accrues dollars to the Squire on approval (SQUIRE-T-0099),
/// tracked as a separate currency: the coin balance gets `reward`, the Cash balance gets `cash`.
#[test]
fn approving_a_cash_quest_accrues_dollars_separately() {
    let mut r = repo();
    let mut q = each(10, 10, false, false);
    q.cash = 5; // $5 chore
    r.seed(&[Change::PutQuest(q)]);
    run(&mut r, submit(1, 1, 10, 0)).unwrap();
    run(&mut r, approve(2, 1)).unwrap();

    let snap = r.snapshot();
    assert_eq!(Proj::balance_in(&snap, UserId(1), Currency::Coins), 10, "coins = reward");
    assert_eq!(Proj::balance_in(&snap, UserId(1), Currency::Cash), 5, "cash = the dollar award");
    // The cash accrual is one Adjusted{Cash} carrying the quest title as its reason.
    let cash_credit = snap.events.iter().any(|e| matches!(
        e, Event::Adjusted { squire, currency: Currency::Cash, amount, .. }
            if *squire == UserId(1) && *amount == 5
    ));
    assert!(cash_credit, "approval emits an Adjusted{{Cash}} for the dollar reward");
}

/// A quest with no cash (`cash = 0`) accrues no dollars — the Cash balance stays zero.
#[test]
fn approving_a_cashless_quest_accrues_no_dollars() {
    let mut r = repo();
    r.seed(&[Change::PutQuest(each(10, 10, false, false))]);
    run(&mut r, submit(1, 1, 10, 0)).unwrap();
    run(&mut r, approve(2, 1)).unwrap();
    assert_eq!(Proj::balance_in(&r.snapshot(), UserId(1), Currency::Cash), 0, "no cash quest ⇒ no dollars");
}

/// The full reject → re-claim → approve cycle: a rejected claim credits nothing and re-opens the
/// slot; the child's *second* claim, once approved, credits exactly once.
#[test]
fn reject_then_reclaim_then_approve_credits_once() {
    let mut r = repo();
    r.seed(&[Change::PutQuest(each(10, 10, false, false))]);
    run(&mut r, submit(1, 1, 10, 0)).unwrap();
    run(&mut r, reject(2, 1)).unwrap();
    assert_eq!(Proj::balance(&r.snapshot(), UserId(1)), 0, "a rejected claim credits nothing");

    // Re-claim the same (squire, quest, day) with a fresh claim id, then approve it.
    run(&mut r, submit(2, 1, 10, 0)).unwrap();
    run(&mut r, approve(2, 2)).unwrap();
    assert_eq!(Proj::balance(&r.snapshot(), UserId(1)), 10, "the re-claim credits exactly once");
    let approvals = r.events.iter().filter(|e| matches!(e, Event::CompletionApproved { squire, .. } if *squire == UserId(1))).count();
    assert_eq!(approvals, 1, "only the second claim was approved");
}
