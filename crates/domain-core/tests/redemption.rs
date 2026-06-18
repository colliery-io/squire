//! T-0005 redemption & ledger tests: per-Squire balance, can_redeem (gate/balance/Once),
//! request/review/redeem/adjust, idempotency, affordability-at-commit (AC-6), actor.

use domain_core::testkit::*;
use domain_core::*;

fn squire(id: u128) -> User {
    User { id: UserId(id), role: Role::Squire, display_name: format!("S{id}"), active: true }
}
fn knight(id: u128) -> User {
    User { id: UserId(id), role: Role::Knight, display_name: format!("K{id}"), active: true }
}
fn item(id: u128, cost: Points, avail: Availability, gate: Option<AchievementId>) -> RedeemableItem {
    RedeemableItem { id: ItemId(id), name: "I".into(), description: None, cost, gate, availability: avail, active: true, icon: None }
}
fn repo() -> InMemoryRepository {
    let mut r = InMemoryRepository::new();
    r.seed(&[Change::PutUser(squire(1)), Change::PutUser(squire(3)), Change::PutUser(knight(2))]);
    r
}
fn run(repo: &mut InMemoryRepository, cmd: Command) -> Result<Vec<Change>, DomainError> {
    let snap = repo.snapshot();
    let r = DomainEngine.handle(&snap, cmd, &FakeClock::at(Date(0), Timestamp(1)));
    if let Ok(changes) = &r {
        repo.apply(Some(UserId(2)), changes).unwrap();
    }
    r
}
fn bal(repo: &InMemoryRepository, squire: u128) -> i64 {
    Proj::balance(&repo.snapshot(), UserId(squire))
}
/// Seed a Squire's balance directly (bypasses the engine) for setup.
fn seed_points(repo: &mut InMemoryRepository, squire: u128, amount: i64, cmd: u128) {
    repo.seed(&[Change::Append(Event::PointsAdjusted {
        command_id: CommandId(cmd), squire: UserId(squire), actor: Some(UserId(2)),
        amount, reason: "seed".into(), at: Timestamp(0),
    })]);
}
fn adjust(cmd: u128, squire: u128, amount: i64) -> Command {
    Command::AdjustPoints { command_id: CommandId(cmd), actor: UserId(2), squire: UserId(squire), amount, reason: "fix".into() }
}
fn request(req: u128, squire: u128, it: u128) -> Command {
    Command::RequestRedemption { request_id: RequestId(req), squire: UserId(squire), item_id: ItemId(it) }
}
fn redeem(cmd: u128, squire: u128, it: u128) -> Command {
    Command::RedeemItem { command_id: CommandId(cmd), actor: UserId(2), squire: UserId(squire), item_id: ItemId(it) }
}

#[test]
fn balance_is_per_squire_and_signed() {
    let mut r = repo();
    r.seed(&[Change::PutItem(item(1, 30, Availability::Repeatable, None))]);
    seed_points(&mut r, 1, 100, 1);
    seed_points(&mut r, 3, 40, 2);
    assert_eq!(bal(&r, 1), 100);
    assert_eq!(bal(&r, 3), 40);
    run(&mut r, redeem(10, 1, 1)).unwrap();
    assert_eq!(bal(&r, 1), 70, "redeem subtracts cost");
    run(&mut r, adjust(11, 1, -20)).unwrap();
    assert_eq!(bal(&r, 1), 50);
    assert_eq!(bal(&r, 3), 40, "Squire 3 untouched");
}

#[test]
fn request_review_approve_emits_item_redeemed_with_actor() {
    let mut r = repo();
    r.seed(&[Change::PutItem(item(1, 30, Availability::Repeatable, None))]);
    seed_points(&mut r, 1, 100, 1);
    run(&mut r, request(7, 1, 1)).unwrap();
    // idempotent on request_id
    let again = run(&mut r, request(7, 1, 1)).unwrap();
    assert!(again.is_empty());
    run(&mut r, Command::ReviewRedemption { actor: UserId(2), request_id: RequestId(7), decision: Decision::Approve }).unwrap();
    let redeemed = r.events.iter().find_map(|e| match e {
        Event::ItemRedeemed { request_id, squire, actor, cost, .. } => Some((*request_id, *squire, *actor, *cost)),
        _ => None,
    });
    assert_eq!(redeemed, Some((Some(RequestId(7)), UserId(1), Some(UserId(2)), 30)));
    assert_eq!(bal(&r, 1), 70);
    // already reviewed
    assert!(matches!(
        run(&mut r, Command::ReviewRedemption { actor: UserId(2), request_id: RequestId(7), decision: Decision::Approve }),
        Err(DomainError::AlreadyReviewed)
    ));
}

#[test]
fn review_reject_and_request_not_found() {
    let mut r = repo();
    r.seed(&[Change::PutItem(item(1, 30, Availability::Repeatable, None))]);
    seed_points(&mut r, 1, 100, 1);
    run(&mut r, request(7, 1, 1)).unwrap();
    run(&mut r, Command::ReviewRedemption { actor: UserId(2), request_id: RequestId(7), decision: Decision::Reject { reason: Some("no".into()) } }).unwrap();
    assert_eq!(bal(&r, 1), 100, "reject spends nothing");
    assert!(matches!(
        run(&mut r, Command::ReviewRedemption { actor: UserId(2), request_id: RequestId(404), decision: Decision::Approve }),
        Err(DomainError::RequestNotFound)
    ));
}

#[test]
fn direct_redeem_is_idempotent_on_command_id() {
    let mut r = repo();
    r.seed(&[Change::PutItem(item(1, 30, Availability::Repeatable, None))]);
    seed_points(&mut r, 1, 100, 1);
    run(&mut r, redeem(5, 1, 1)).unwrap();
    assert_eq!(bal(&r, 1), 70);
    let again = run(&mut r, redeem(5, 1, 1)).unwrap();
    assert!(again.is_empty(), "replay of a command_id is a no-op");
    assert_eq!(bal(&r, 1), 70, "no double spend");
}

#[test]
fn adjust_idempotent_and_requires_reason() {
    let mut r = repo();
    run(&mut r, adjust(5, 1, 25)).unwrap();
    assert_eq!(bal(&r, 1), 25);
    let again = run(&mut r, adjust(5, 1, 25)).unwrap();
    assert!(again.is_empty(), "replay of a command_id is a no-op");
    assert_eq!(bal(&r, 1), 25, "no double credit");
    assert!(matches!(
        run(&mut r, Command::AdjustPoints { command_id: CommandId(6), actor: UserId(2), squire: UserId(1), amount: 5, reason: "  ".into() }),
        Err(DomainError::InvalidDefinition)
    ));
}

#[test]
fn can_redeem_insufficient_points_blocks_and_cannot_go_negative() {
    let mut r = repo();
    r.seed(&[Change::PutItem(item(1, 30, Availability::Repeatable, None))]);
    seed_points(&mut r, 1, 5, 1);
    let snap = r.snapshot();
    assert!(matches!(
        Proj::can_redeem(&snap, UserId(1), ItemId(1), Date(0)),
        Err(Blocked::InsufficientPoints { needed: 30, have: 5 })
    ));
    assert!(matches!(run(&mut r, redeem(9, 1, 1)), Err(DomainError::Redeem(Blocked::InsufficientPoints { .. }))));
    assert_eq!(bal(&r, 1), 5, "a redeem can never push the balance negative");
}

#[test]
fn once_item_is_out_of_stock_after_one_redemption() {
    let mut r = repo();
    r.seed(&[
        Change::PutItem(item(1, 10, Availability::Once, None)),
        Change::PutItem(item(2, 10, Availability::Repeatable, None)),
    ]);
    seed_points(&mut r, 1, 100, 1);
    run(&mut r, redeem(5, 1, 1)).unwrap(); // Once item
    assert!(matches!(Proj::can_redeem(&r.snapshot(), UserId(3), ItemId(1), Date(0)), Err(Blocked::OutOfStock)));
    assert!(matches!(run(&mut r, redeem(6, 1, 1)), Err(DomainError::Redeem(Blocked::OutOfStock))));
    // Repeatable item: fine to redeem twice.
    run(&mut r, redeem(7, 1, 2)).unwrap();
    run(&mut r, redeem(8, 1, 2)).unwrap();
}

#[test]
fn gated_item_is_locked_until_unlocked() {
    let mut r = repo();
    r.seed(&[Change::PutItem(item(1, 10, Availability::Repeatable, Some(AchievementId(7))))]);
    seed_points(&mut r, 1, 100, 1);
    assert!(matches!(
        Proj::can_redeem(&r.snapshot(), UserId(1), ItemId(1), Date(0)),
        Err(Blocked::AchievementLocked { id: AchievementId(7) })
    ));
    // Unlock for Squire 1 only.
    r.seed(&[Change::Append(Event::AchievementUnlocked { squire: UserId(1), id: AchievementId(7), bonus: 0, at: Timestamp(0) })]);
    assert!(Proj::can_redeem(&r.snapshot(), UserId(1), ItemId(1), Date(0)).is_ok());
    assert!(matches!(
        Proj::can_redeem(&r.snapshot(), UserId(3), ItemId(1), Date(0)),
        Err(Blocked::AchievementLocked { .. }),
    ), "gate is per-Squire");
}

#[test]
fn affordability_is_rechecked_at_approval_ac6() {
    let mut r = repo();
    r.seed(&[Change::PutItem(item(1, 30, Availability::Repeatable, None))]);
    seed_points(&mut r, 1, 30, 1);
    run(&mut r, request(7, 1, 1)).unwrap(); // affordable at request time
    run(&mut r, adjust(8, 1, -30)).unwrap(); // ...then the balance is drained
    assert!(matches!(
        run(&mut r, Command::ReviewRedemption { actor: UserId(2), request_id: RequestId(7), decision: Decision::Approve }),
        Err(DomainError::Redeem(Blocked::InsufficientPoints { .. }))
    ), "AC-6: re-checked at commit, fails when no longer affordable");
}

#[test]
fn balance_can_go_negative_only_via_adjustment() {
    let mut r = repo();
    run(&mut r, adjust(1, 1, -100)).unwrap();
    assert_eq!(bal(&r, 1), -100, "explicit adjustment may go negative");
}

// ── Hardening (SQUIRE-T-0077) ──────────────────────────────────────────────────────────────────

fn reject(req: u128, reason: &str) -> Command {
    Command::ReviewRedemption { actor: UserId(2), request_id: RequestId(req), decision: Decision::Reject { reason: Some(reason.into()) } }
}
fn approve(req: u128) -> Command {
    Command::ReviewRedemption { actor: UserId(2), request_id: RequestId(req), decision: Decision::Approve }
}

/// A second review of an already-rejected request is `AlreadyReviewed` (a resolved request, reject
/// or approve, can't be re-decided).
#[test]
fn double_reject_is_already_reviewed() {
    let mut r = repo();
    r.seed(&[Change::PutItem(item(1, 30, Availability::Repeatable, None))]);
    seed_points(&mut r, 1, 100, 1);
    run(&mut r, request(7, 1, 1)).unwrap();
    run(&mut r, reject(7, "not today")).unwrap();
    assert_eq!(bal(&r, 1), 100, "reject spends nothing");
    assert!(matches!(run(&mut r, reject(7, "again")), Err(DomainError::AlreadyReviewed)));
    assert!(matches!(run(&mut r, approve(7)), Err(DomainError::AlreadyReviewed)), "can't approve a rejected request either");
}

/// A request for a gated item is allowed while locked (no gate check at request time); approving it
/// while still locked fails, but once the gate unlocks the same request approves and debits.
#[test]
fn gated_request_unlocks_before_approval_then_debits() {
    let mut r = repo();
    r.seed(&[Change::PutItem(item(1, 30, Availability::Repeatable, Some(AchievementId(9))))]);
    seed_points(&mut r, 1, 100, 1);
    // Requesting is allowed even though the gate is locked.
    run(&mut r, request(7, 1, 1)).unwrap();
    // Approving while still locked fails (re-checked at commit), leaving the request pending.
    assert!(matches!(run(&mut r, approve(7)), Err(DomainError::Redeem(Blocked::AchievementLocked { .. }))));
    assert_eq!(bal(&r, 1), 100, "a blocked approval debits nothing");
    // Unlock the gate for this Squire, then the same request approves and debits.
    r.seed(&[Change::Append(Event::AchievementUnlocked { squire: UserId(1), id: AchievementId(9), bonus: 0, at: Timestamp(0) })]);
    run(&mut r, approve(7)).unwrap();
    assert_eq!(bal(&r, 1), 70, "the now-unlocked request debits the cost");
}
