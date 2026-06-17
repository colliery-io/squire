//! T-0007 consolidated invariant property-test suite (NFR-1.1.3 / NFR-1.1.5).
//!
//! A stateful proptest replays randomised `Action` sequences through `DomainEngine.handle`
//! (applying any resulting `Change`s by Knight `UserId(9)`) and asserts the domain's core
//! invariants over the resulting event log: idempotency (no duplicate facts), referential
//! integrity (approvals imply claims), the Race single-payout rule, `Once` single-redemption,
//! balance non-negativity (absent a negative adjust), and per-Squire isolation of all
//! derivations. Small id spaces are used deliberately so collisions exercise the dedup paths.
//!
//! Four explicit deterministic `#[test]`s pin the trickiest rules (Race resolution, Race
//! reject-reopens, household-wide `Once`, and replay-is-a-no-op).

use domain_core::testkit::*;
use domain_core::*;
use proptest::prelude::*;
use std::collections::{BTreeSet, HashMap};

// ── Fixture ────────────────────────────────────────────────────────────────────

const KNIGHT: u128 = 9;
const Q_RACE: u128 = 3; // q3 is the Race quest
const ITEM_ONCE: u128 = 2; // i2 is the `Once` item

fn squire(id: u128) -> User {
    User { id: UserId(id), role: Role::Squire, display_name: format!("S{id}"), active: true }
}
fn knight(id: u128) -> User {
    User { id: UserId(id), role: Role::Knight, display_name: format!("K{id}"), active: true }
}

fn quest(
    id: u128,
    reward: Points,
    completion: Completion,
    assignment: Assignment,
    auto_approve: bool,
    repeatable: bool,
) -> Quest {
    Quest {
        id: QuestId(id),
        title: format!("Q{id}"),
        description: None,
        category: None,
        reward,
        cadence: Cadence::Recurring(Schedule::Daily),
        assignment,
        completion,
        auto_approve,
        repeatable_within_day: repeatable,
        active: true,
        icon: None,
    }
}

fn item(id: u128, cost: Points, avail: Availability) -> RedeemableItem {
    RedeemableItem { id: ItemId(id), name: format!("I{id}"), description: None, cost, gate: None, availability: avail, active: true, icon: None }
}

fn all_squires() -> Assignment {
    Assignment::Squires([UserId(1), UserId(2), UserId(3)].into_iter().collect::<BTreeSet<_>>())
}

/// Fresh fixture: squires 1/2/3, knight 9; q1..q4, i1/i2, one Streak achievement.
fn fixture() -> InMemoryRepository {
    let mut r = InMemoryRepository::new();
    r.seed(&[
        Change::PutUser(squire(1)),
        Change::PutUser(squire(2)),
        Change::PutUser(squire(3)),
        Change::PutUser(knight(KNIGHT)),
        // q1: daily EachAssignee non-repeatable reward 10 (manual)
        Change::PutQuest(quest(1, 10, Completion::EachAssignee, Assignment::AllSquires, false, false)),
        // q2: daily EachAssignee repeatable reward 5 (manual)
        Change::PutQuest(quest(2, 5, Completion::EachAssignee, Assignment::AllSquires, false, true)),
        // q3: daily Race reward 20 AllSquires (manual)
        Change::PutQuest(quest(Q_RACE, 20, Completion::Race, all_squires(), false, false)),
        // q4: daily EachAssignee auto_approve reward 7
        Change::PutQuest(quest(4, 7, Completion::EachAssignee, Assignment::AllSquires, true, false)),
        // i1 Repeatable cost 10, i2 Once cost 15
        Change::PutItem(item(1, 10, Availability::Repeatable)),
        Change::PutItem(item(ITEM_ONCE, 15, Availability::Once)),
        // achievement: Streak{ Scope::Quest(q1), length 3, ScheduledOccurrences } bonus 50
        Change::PutAchievement(Achievement {
            id: AchievementId(1),
            name: "Streak3".into(),
            description: None,
            criterion: Criterion::Streak {
                scope: Scope::Quest(QuestId(1)),
                length: 3,
                basis: StreakBasis::ScheduledOccurrences,
            },
            bonus_points: 50,
            active: true,
        }),
    ]);
    r
}

// ── Action model ────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
enum Action {
    Submit { claim: u8, squire: u128, quest: u128, day: i32 },
    Redeem { cmd: u8, squire: u128, item: u128 },
    Adjust { cmd: u8, squire: u128, amount: i64 },
    Request { req: u8, squire: u128, item: u128 },
    ReviewClaim { approve: bool },
    ReviewRequest { approve: bool },
}

fn action_strategy() -> impl Strategy<Value = Action> {
    prop_oneof![
        (0u8..12, 1u128..=3, 1u128..=4, 0i32..5)
            .prop_map(|(claim, squire, quest, day)| Action::Submit { claim, squire, quest, day }),
        (0u8..12, 1u128..=3, 1u128..=2).prop_map(|(cmd, squire, item)| Action::Redeem { cmd, squire, item }),
        (0u8..12, 1u128..=3, -30i64..30).prop_map(|(cmd, squire, amount)| Action::Adjust { cmd, squire, amount }),
        (0u8..12, 1u128..=3, 1u128..=2).prop_map(|(req, squire, item)| Action::Request { req, squire, item }),
        any::<bool>().prop_map(|approve| Action::ReviewClaim { approve }),
        any::<bool>().prop_map(|approve| Action::ReviewRequest { approve }),
    ]
}

// ── Log scanning helpers ─────────────────────────────────────────────────────────

/// Oldest currently-pending claim: has `CompletionClaimed`, no Approved/Rejected.
fn oldest_pending_claim(repo: &InMemoryRepository) -> Option<ClaimId> {
    let resolved: BTreeSet<u128> = repo
        .events
        .iter()
        .filter_map(|e| match e {
            Event::CompletionApproved { claim_id, .. } => Some(claim_id.0),
            Event::CompletionRejected { claim_id, .. } => Some(claim_id.0),
            _ => None,
        })
        .collect();
    repo.events.iter().find_map(|e| match e {
        Event::CompletionClaimed { claim_id, .. } if !resolved.contains(&claim_id.0) => Some(*claim_id),
        _ => None,
    })
}

/// Oldest unresolved redemption request: has `RedemptionRequested`, no resolving event.
fn oldest_unresolved_request(repo: &InMemoryRepository) -> Option<RequestId> {
    let resolved: BTreeSet<u128> = repo
        .events
        .iter()
        .filter_map(|e| match e {
            Event::ItemRedeemed { request_id: Some(r), .. } => Some(r.0),
            Event::RedemptionRejected { request_id: r, .. } => Some(r.0),
            _ => None,
        })
        .collect();
    repo.events.iter().find_map(|e| match e {
        Event::RedemptionRequested { request_id, .. } if !resolved.contains(&request_id.0) => Some(*request_id),
        _ => None,
    })
}

fn decision(approve: bool) -> Decision {
    if approve {
        Decision::Approve
    } else {
        Decision::Reject { reason: Some("nope".into()) }
    }
}

// ── Replay ──────────────────────────────────────────────────────────────────────

fn replay(actions: &[Action]) -> InMemoryRepository {
    let mut repo = fixture();
    for (i, action) in actions.iter().enumerate() {
        // Timestamp = action index → strictly monotone; date fixed at Date(0) = Monday.
        let clock = FakeClock::at(Date(0), Timestamp(i as i64));
        let cmd: Option<Command> = match action {
            Action::Submit { claim, squire, quest, day } => Some(Command::SubmitClaim {
                claim_id: ClaimId(*claim as u128),
                squire: UserId(*squire),
                quest_id: QuestId(*quest),
                on: Date(*day),
            }),
            Action::Redeem { cmd, squire, item } => Some(Command::RedeemItem {
                command_id: CommandId(*cmd as u128),
                actor: UserId(KNIGHT),
                squire: UserId(*squire),
                item_id: ItemId(*item),
            }),
            Action::Adjust { cmd, squire, amount } => Some(Command::AdjustPoints {
                command_id: CommandId(1000 + *cmd as u128), // disjoint id space
                actor: UserId(KNIGHT),
                squire: UserId(*squire),
                amount: *amount,
                reason: "fix".into(),
            }),
            Action::Request { req, squire, item } => Some(Command::RequestRedemption {
                request_id: RequestId(*req as u128),
                squire: UserId(*squire),
                item_id: ItemId(*item),
            }),
            Action::ReviewClaim { approve } => oldest_pending_claim(&repo).map(|claim_id| {
                Command::ReviewClaim { actor: UserId(KNIGHT), claim_id, decision: decision(*approve) }
            }),
            Action::ReviewRequest { approve } => oldest_unresolved_request(&repo).map(|request_id| {
                Command::ReviewRedemption { actor: UserId(KNIGHT), request_id, decision: decision(*approve) }
            }),
        };
        if let Some(cmd) = cmd {
            let snap = repo.snapshot();
            if let Ok(changes) = DomainEngine.handle(&snap, cmd, &clock) {
                repo.apply(Some(UserId(KNIGHT)), &changes).unwrap();
            }
        }
    }
    repo
}

/// Build a snapshot whose `events` are only `squire`'s events (definitions/users kept).
fn snapshot_for_squire(repo: &InMemoryRepository, squire: UserId) -> Snapshot {
    let mut snap = repo.snapshot();
    // Determine which claim_ids belong to this squire (from CompletionClaimed).
    let own_claims: BTreeSet<u128> = repo
        .events
        .iter()
        .filter_map(|e| match e {
            Event::CompletionClaimed { claim_id, squire: s, .. } if *s == squire => Some(claim_id.0),
            _ => None,
        })
        .collect();
    snap.events.retain(|e| match e {
        Event::CompletionClaimed { squire: s, .. } => *s == squire,
        Event::CompletionApproved { squire: s, claim_id, .. } => *s == squire || own_claims.contains(&claim_id.0),
        Event::CompletionRejected { squire: s, claim_id, .. } => *s == squire || own_claims.contains(&claim_id.0),
        Event::ItemRedeemed { squire: s, .. } => *s == squire,
        Event::AchievementUnlocked { squire: s, .. } => *s == squire,
        Event::PointsAdjusted { squire: s, .. } => *s == squire,
        Event::RedemptionRequested { squire: s, .. } => *s == squire,
        Event::RedemptionRejected { squire: s, .. } => *s == squire,
    });
    snap
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 200, ..ProptestConfig::default() })]

    #[test]
    fn invariants_hold(actions in prop::collection::vec(action_strategy(), 0..60)) {
        let repo = replay(&actions);
        let snap = repo.snapshot();
        let events = &repo.events;

        // ── 1. No duplicate facts (idempotency) ──────────────────────────────────
        let mut claimed: HashMap<u128, u32> = HashMap::new();
        let mut approved: HashMap<u128, u32> = HashMap::new();
        let mut rejected: HashMap<u128, u32> = HashMap::new();
        let mut requested: HashMap<u128, u32> = HashMap::new();
        let mut by_command: HashMap<u128, u32> = HashMap::new();
        for e in events {
            match e {
                Event::CompletionClaimed { claim_id, .. } => *claimed.entry(claim_id.0).or_default() += 1,
                Event::CompletionApproved { claim_id, .. } => *approved.entry(claim_id.0).or_default() += 1,
                Event::CompletionRejected { claim_id, .. } => *rejected.entry(claim_id.0).or_default() += 1,
                Event::RedemptionRequested { request_id, .. } => *requested.entry(request_id.0).or_default() += 1,
                Event::ItemRedeemed { command_id: Some(c), .. } => *by_command.entry(c.0).or_default() += 1,
                Event::PointsAdjusted { command_id, .. } => *by_command.entry(command_id.0).or_default() += 1,
                _ => {}
            }
        }
        for (k, n) in &claimed { prop_assert!(*n <= 1, "claim_id {k} claimed {n}× "); }
        for (k, n) in &approved { prop_assert!(*n <= 1, "claim_id {k} approved {n}× "); }
        for (k, n) in &rejected { prop_assert!(*n <= 1, "claim_id {k} rejected {n}× "); }
        for (k, n) in &requested { prop_assert!(*n <= 1, "request_id {k} requested {n}× "); }
        for (k, n) in &by_command { prop_assert!(*n <= 1, "command_id {k} applied {n}× "); }

        // ── 2. Approved ⇒ has a matching claim ───────────────────────────────────
        for e in events {
            if let Event::CompletionApproved { claim_id, .. } = e {
                prop_assert!(claimed.contains_key(&claim_id.0), "approved claim {:?} has no CompletionClaimed", claim_id);
            }
        }

        // ── 3. Race ≤ one payout per (quest, on) occurrence ──────────────────────
        // Map claim_id → (quest, on) from CompletionClaimed; group approvals by occurrence.
        let claim_occ: HashMap<u128, (u128, i32)> = events
            .iter()
            .filter_map(|e| match e {
                Event::CompletionClaimed { claim_id, quest_id, on, .. } => Some((claim_id.0, (quest_id.0, on.0))),
                _ => None,
            })
            .collect();
        let mut race_payouts: HashMap<(u128, i32), u32> = HashMap::new();
        for e in events {
            if let Event::CompletionApproved { claim_id, .. } = e {
                if let Some(&(q, d)) = claim_occ.get(&claim_id.0) {
                    if q == Q_RACE {
                        *race_payouts.entry((q, d)).or_default() += 1;
                    }
                }
            }
        }
        for (occ, n) in &race_payouts {
            prop_assert!(*n <= 1, "Race occurrence {:?} paid out {n}× ", occ);
        }

        // ── 4. Once ≤ one redemption for i2 ──────────────────────────────────────
        let once_redemptions = events
            .iter()
            .filter(|e| matches!(e, Event::ItemRedeemed { item_id, .. } if item_id.0 == ITEM_ONCE))
            .count();
        prop_assert!(once_redemptions <= 1, "Once item redeemed {once_redemptions}× ");

        // ── 5. Balance non-negative unless a negative adjust for that squire ──────
        for sid in [1u128, 2, 3] {
            let has_negative_adjust = events.iter().any(|e| matches!(
                e,
                Event::PointsAdjusted { squire, amount, .. } if squire.0 == sid && *amount < 0
            ));
            if !has_negative_adjust {
                let bal = Proj::balance(&snap, UserId(sid));
                prop_assert!(bal >= 0, "squire {sid} balance {bal} < 0 with no negative adjust");
            }
        }

        // ── 6. Per-Squire isolation ──────────────────────────────────────────────
        for sid in [1u128, 2, 3] {
            let s = UserId(sid);
            let isolated = snapshot_for_squire(&repo, s);
            prop_assert_eq!(
                Proj::balance(&isolated, s),
                Proj::balance(&snap, s),
                "balance differs under isolation for squire {}", sid
            );
            prop_assert_eq!(
                Proj::current_streak(&isolated, s, &Scope::Quest(QuestId(1)), StreakBasis::ScheduledOccurrences, Date(4)),
                Proj::current_streak(&snap, s, &Scope::Quest(QuestId(1)), StreakBasis::ScheduledOccurrences, Date(4)),
                "q1 streak differs under isolation for squire {}", sid
            );
        }
    }
}

// ── Explicit deterministic tests ─────────────────────────────────────────────────

/// Run a command and apply resulting changes by Knight 9.
fn run(repo: &mut InMemoryRepository, cmd: Command, t: i64) -> Result<Vec<Change>, DomainError> {
    let snap = repo.snapshot();
    let r = DomainEngine.handle(&snap, cmd, &FakeClock::at(Date(0), Timestamp(t)));
    if let Ok(changes) = &r {
        repo.apply(Some(UserId(KNIGHT)), changes).unwrap();
    }
    r
}

fn submit(claim: u128, squire: u128, q: u128, day: i32) -> Command {
    Command::SubmitClaim { claim_id: ClaimId(claim), squire: UserId(squire), quest_id: QuestId(q), on: Date(day) }
}
fn approve(claim: u128) -> Command {
    Command::ReviewClaim { actor: UserId(KNIGHT), claim_id: ClaimId(claim), decision: Decision::Approve }
}
fn reject(claim: u128) -> Command {
    Command::ReviewClaim { actor: UserId(KNIGHT), claim_id: ClaimId(claim), decision: Decision::Reject { reason: Some("no".into()) } }
}

fn approvals_for_occurrence(repo: &InMemoryRepository, quest: u128, on: i32) -> usize {
    let claim_occ: HashMap<u128, (u128, i32)> = repo
        .events
        .iter()
        .filter_map(|e| match e {
            Event::CompletionClaimed { claim_id, quest_id, on, .. } => Some((claim_id.0, (quest_id.0, on.0))),
            _ => None,
        })
        .collect();
    repo.events
        .iter()
        .filter(|e| match e {
            Event::CompletionApproved { claim_id, .. } => claim_occ.get(&claim_id.0) == Some(&(quest, on)),
            _ => false,
        })
        .count()
}

#[test]
fn race_exactly_one_winner_others_taken() {
    // (a) Three squires submit the same Race occurrence; approving in turn pays exactly one,
    // the others error OccurrenceTaken.
    let mut r = fixture();
    run(&mut r, submit(1, 1, Q_RACE, 0), 0).unwrap();
    run(&mut r, submit(2, 2, Q_RACE, 0), 1).unwrap();
    run(&mut r, submit(3, 3, Q_RACE, 0), 2).unwrap();
    run(&mut r, approve(1), 3).unwrap();
    assert!(matches!(run(&mut r, approve(2), 4), Err(DomainError::OccurrenceTaken)));
    assert!(matches!(run(&mut r, approve(3), 5), Err(DomainError::OccurrenceTaken)));
    assert_eq!(approvals_for_occurrence(&r, Q_RACE, 0), 1, "exactly one payout for the occurrence");
}

#[test]
fn race_reject_reopens_lets_a_later_squire_win() {
    // (b) Rejecting the first claim reopens the occurrence so a later squire can win.
    let mut r = fixture();
    run(&mut r, submit(1, 1, Q_RACE, 0), 0).unwrap();
    run(&mut r, reject(1), 1).unwrap();
    run(&mut r, submit(2, 2, Q_RACE, 0), 2).unwrap();
    run(&mut r, approve(2), 3).unwrap();
    assert_eq!(approvals_for_occurrence(&r, Q_RACE, 0), 1, "the reopened occurrence is won once");
}

#[test]
fn once_item_redeemable_exactly_once_householdwide() {
    // (c) The Once item (i2) can be redeemed exactly once across the household.
    let mut r = fixture();
    // Fund squires 1 and 3.
    run(&mut r, Command::AdjustPoints { command_id: CommandId(900), actor: UserId(KNIGHT), squire: UserId(1), amount: 100, reason: "seed".into() }, 0).unwrap();
    run(&mut r, Command::AdjustPoints { command_id: CommandId(901), actor: UserId(KNIGHT), squire: UserId(3), amount: 100, reason: "seed".into() }, 1).unwrap();
    run(&mut r, Command::RedeemItem { command_id: CommandId(1), actor: UserId(KNIGHT), squire: UserId(1), item_id: ItemId(ITEM_ONCE) }, 2).unwrap();
    // Second redeem (different squire, fresh command_id) → OutOfStock.
    assert!(matches!(
        run(&mut r, Command::RedeemItem { command_id: CommandId(2), actor: UserId(KNIGHT), squire: UserId(3), item_id: ItemId(ITEM_ONCE) }, 3),
        Err(DomainError::Redeem(Blocked::OutOfStock))
    ));
    let count = r.events.iter().filter(|e| matches!(e, Event::ItemRedeemed { item_id, .. } if item_id.0 == ITEM_ONCE)).count();
    assert_eq!(count, 1);
}

#[test]
fn replaying_identical_keys_is_a_no_op() {
    // (d) Replaying the same claim_id / request_id / command_id appends nothing.
    let mut r = fixture();
    run(&mut r, Command::AdjustPoints { command_id: CommandId(900), actor: UserId(KNIGHT), squire: UserId(1), amount: 100, reason: "seed".into() }, 0).unwrap();
    // Establish each fact once.
    run(&mut r, submit(1, 1, 1, 0), 1).unwrap();
    run(&mut r, Command::RequestRedemption { request_id: RequestId(7), squire: UserId(1), item_id: ItemId(1) }, 2).unwrap();
    run(&mut r, Command::RedeemItem { command_id: CommandId(5), actor: UserId(KNIGHT), squire: UserId(1), item_id: ItemId(1) }, 3).unwrap();
    let before = r.events.len();
    // Replay identical keys.
    assert!(run(&mut r, submit(1, 1, 1, 0), 4).unwrap().is_empty());
    assert!(run(&mut r, Command::RequestRedemption { request_id: RequestId(7), squire: UserId(1), item_id: ItemId(1) }, 5).unwrap().is_empty());
    assert!(run(&mut r, Command::RedeemItem { command_id: CommandId(5), actor: UserId(KNIGHT), squire: UserId(1), item_id: ItemId(1) }, 6).unwrap().is_empty());
    assert!(run(&mut r, Command::AdjustPoints { command_id: CommandId(900), actor: UserId(KNIGHT), squire: UserId(1), amount: 100, reason: "seed".into() }, 7).unwrap().is_empty());
    assert_eq!(r.events.len(), before, "no new events appended on replay");
}
