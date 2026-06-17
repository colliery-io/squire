//! Claim & review (T-0003): `SubmitClaim` / `ReviewClaim` — per-Squire subject validation,
//! assignment gating, auto-approve, snapshot-at-approval, single-review, actor stamping,
//! claim idempotency, and the `Race` occurrence-resolution rule.
//!
//! Stub until T-0003 — returns no Changes.

use crate::contract::*;

pub(crate) fn handle(
    _snap: &Snapshot,
    _cmd: Command,
    _clock: &dyn Clock,
) -> Result<Vec<Change>, DomainError> {
    Ok(vec![]) // TODO(T-0003)
}
