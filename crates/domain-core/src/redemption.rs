//! Redemption & ledger (T-0005): `RequestRedemption` / `ReviewRedemption` / `RedeemItem` /
//! `AdjustPoints`.
//!
//! Affordability is **re-checked at commit** (FR-R4/R5) via `Projections::can_redeem` — no
//! points are reserved while a request is pending. Direct `RedeemItem` and `AdjustPoints` are
//! idempotent on their client-minted `command_id` (ADR SQUIRE-A-0001), derived from the log.
//! Every committed event carries the target `squire` and the acting Knight `actor`.

use crate::common::*;
use crate::contract::*;
use crate::projections::Proj;

pub(crate) fn handle(
    snap: &Snapshot,
    cmd: Command,
    clock: &dyn Clock,
) -> Result<Vec<Change>, DomainError> {
    match cmd {
        Command::RequestRedemption {
            request_id,
            squire,
            item_id,
        } => request(snap, request_id, squire, item_id, clock),
        Command::ReviewRedemption {
            actor,
            request_id,
            decision,
        } => review(snap, actor, request_id, decision, clock),
        Command::RedeemItem {
            command_id,
            actor,
            squire,
            item_id,
        } => redeem_direct(snap, command_id, actor, squire, item_id, clock),
        Command::AdjustPoints {
            command_id,
            actor,
            squire,
            currency,
            amount,
            reason,
        } => adjust(
            snap, command_id, actor, squire, currency, amount, reason, clock,
        ),
        Command::RequestCashOut {
            request_id,
            squire,
            amount,
        } => request_cashout(snap, request_id, squire, amount, clock),
        Command::ReviewCashOut {
            actor,
            request_id,
            decision,
        } => review_cashout(snap, actor, request_id, decision, clock),
        _ => unreachable!("redemption::handle only receives redemption/ledger commands"),
    }
}

fn request(
    snap: &Snapshot,
    request_id: RequestId,
    squire: UserId,
    item_id: ItemId,
    clock: &dyn Clock,
) -> Result<Vec<Change>, DomainError> {
    // Idempotent on the phone-minted request_id; reserves nothing (FR-R5).
    if request_meta(snap, request_id).is_some() {
        return Ok(Vec::new());
    }
    require_active_squire(snap, squire)?;
    let item = find_item(snap, item_id).ok_or(DomainError::ItemNotFound)?;
    if !item.active {
        return Err(DomainError::Inactive);
    }
    Ok(vec![Change::Append(Event::RedemptionRequested {
        request_id,
        squire,
        item_id,
        at: clock.now(),
    })])
}

fn review(
    snap: &Snapshot,
    actor: UserId,
    request_id: RequestId,
    decision: Decision,
    clock: &dyn Clock,
) -> Result<Vec<Change>, DomainError> {
    let (squire, item_id) = request_meta(snap, request_id).ok_or(DomainError::RequestNotFound)?;
    if request_resolved(snap, request_id) {
        return Err(DomainError::AlreadyReviewed);
    }
    match decision {
        Decision::Approve => {
            commit_redeem(snap, squire, item_id, Some(request_id), None, actor, clock)
        }
        Decision::Reject { reason } => Ok(vec![Change::Append(Event::RedemptionRejected {
            request_id,
            squire,
            actor: Some(actor),
            reason,
            at: clock.now(),
        })]),
    }
}

fn redeem_direct(
    snap: &Snapshot,
    command_id: CommandId,
    actor: UserId,
    squire: UserId,
    item_id: ItemId,
    clock: &dyn Clock,
) -> Result<Vec<Change>, DomainError> {
    // Idempotent on command_id (retry-safe outbox).
    if command_already_applied(snap, command_id) {
        return Ok(Vec::new());
    }
    require_active_squire(snap, squire)?;
    commit_redeem(snap, squire, item_id, None, Some(command_id), actor, clock)
}

/// Shared redeem commit (request-approval and direct): re-check `can_redeem` at commit, then
/// emit `ItemRedeemed` snapshotting the item's current cost. Exactly one of `request_id` /
/// `command_id` is `Some`.
fn commit_redeem(
    snap: &Snapshot,
    squire: UserId,
    item_id: ItemId,
    request_id: Option<RequestId>,
    command_id: Option<CommandId>,
    actor: UserId,
    clock: &dyn Clock,
) -> Result<Vec<Change>, DomainError> {
    let item = find_item(snap, item_id).ok_or(DomainError::ItemNotFound)?;
    if !item.active {
        return Err(DomainError::Inactive);
    }
    Proj::can_redeem(snap, squire, item_id, clock.today()).map_err(DomainError::Redeem)?;
    Ok(vec![Change::Append(Event::ItemRedeemed {
        request_id,
        command_id,
        squire,
        actor: Some(actor),
        item_id,
        cost: item.cost,
        at: clock.now(),
    })])
}

// ── Cash-out (T-0118): a Squire requests to draw down owed Cash; reviewed like a redemption ──

/// A cash-out must be a positive whole-dollar amount no larger than the Squire's currently-owed
/// Cash. Re-used at request and at commit (owed can change in between), mirroring `can_redeem`.
fn check_cashout_affordable(
    snap: &Snapshot,
    squire: UserId,
    amount: i64,
) -> Result<(), DomainError> {
    let owed = Proj::balance_in(snap, squire, Currency::Cash);
    if amount <= 0 || owed < amount {
        return Err(DomainError::Redeem(Blocked::InsufficientPoints {
            needed: amount.max(0) as Points,
            have: owed,
        }));
    }
    Ok(())
}

fn request_cashout(
    snap: &Snapshot,
    request_id: RequestId,
    squire: UserId,
    amount: i64,
    clock: &dyn Clock,
) -> Result<Vec<Change>, DomainError> {
    // Idempotent on the phone-minted request_id; reserves nothing (mirrors redemption FR-R5).
    if cashout_meta(snap, request_id).is_some() {
        return Ok(Vec::new());
    }
    require_active_squire(snap, squire)?;
    check_cashout_affordable(snap, squire, amount)?;
    Ok(vec![Change::Append(Event::CashOutRequested {
        request_id,
        squire,
        amount,
        at: clock.now(),
    })])
}

fn review_cashout(
    snap: &Snapshot,
    actor: UserId,
    request_id: RequestId,
    decision: Decision,
    clock: &dyn Clock,
) -> Result<Vec<Change>, DomainError> {
    let (squire, amount) = cashout_meta(snap, request_id).ok_or(DomainError::RequestNotFound)?;
    if cashout_resolved(snap, request_id) {
        return Err(DomainError::AlreadyReviewed);
    }
    match decision {
        Decision::Approve => {
            // Re-check at commit — the owed Cash may have changed since the request (FR-R4 analogue).
            check_cashout_affordable(snap, squire, amount)?;
            Ok(vec![Change::Append(Event::CashOutApproved {
                request_id,
                squire,
                actor: Some(actor),
                amount,
                at: clock.now(),
            })])
        }
        Decision::Reject { reason } => Ok(vec![Change::Append(Event::CashOutRejected {
            request_id,
            squire,
            actor: Some(actor),
            reason,
            at: clock.now(),
        })]),
    }
}

#[allow(clippy::too_many_arguments)]
fn adjust(
    snap: &Snapshot,
    command_id: CommandId,
    actor: UserId,
    squire: UserId,
    currency: Currency,
    amount: i64,
    reason: String,
    clock: &dyn Clock,
) -> Result<Vec<Change>, DomainError> {
    if command_already_applied(snap, command_id) {
        return Ok(Vec::new());
    }
    require_active_squire(snap, squire)?;
    // Adjustments must be explained (FR-P2); an empty reason is a malformed command.
    if reason.trim().is_empty() {
        return Err(DomainError::InvalidDefinition);
    }
    Ok(vec![Change::Append(Event::Adjusted {
        command_id,
        squire,
        actor: Some(actor),
        currency,
        amount,
        reason,
        at: clock.now(),
    })])
}
