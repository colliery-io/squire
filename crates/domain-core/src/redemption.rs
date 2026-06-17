//! Redemption & ledger (T-0005): `RequestRedemption` / `ReviewRedemption` / `RedeemItem` /
//! `AdjustPoints` — per-Squire balance, `can_redeem` at commit (active + gate + balance +
//! Once-out-of-stock), idempotency (`request_id` / `command_id`), actor stamping.
//!
//! Stub until T-0005 — returns no Changes.

use crate::contract::*;

pub(crate) fn handle(
    _snap: &Snapshot,
    _cmd: Command,
    _clock: &dyn Clock,
) -> Result<Vec<Change>, DomainError> {
    Ok(vec![]) // TODO(T-0005)
}
