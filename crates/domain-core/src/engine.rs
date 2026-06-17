//! `Engine::handle` — the single validated door (AR-5).
//!
//! T-0001 provides the dispatch skeleton + the child-surface guard. The per-family handlers
//! below are stubs (`Ok(vec![])`) filled in by later tasks: authoring (T-0002), claim/review
//! (T-0003), redemption & ledger (T-0005). `handle` decides `Change`s; it never applies them.

use crate::contract::*;

/// The concrete pure engine. Carries no state — all inputs arrive via the `Snapshot`,
/// `Command`, and `&dyn Clock`.
#[derive(Clone, Copy, Debug, Default)]
pub struct DomainEngine;

/// Commands a child (Squire) may originate over the network surface. Everything else is an
/// admin/Knight operation and must be rejected on the child path (`BadCommandForActor`).
pub fn child_originable(cmd: &Command) -> bool {
    matches!(
        cmd,
        Command::SubmitClaim { .. } | Command::RequestRedemption { .. }
    )
}

impl DomainEngine {
    /// Guarded entry for the network/child surface: rejects any admin-only command with
    /// `BadCommandForActor`, then delegates to [`Engine::handle`]. The in-process admin
    /// (the Keep) calls `handle` directly. The contract `Engine::handle` takes no caller
    /// role, so the trust-boundary check lives here, where the surface is known
    /// (REQ-1.1.2, AR-8).
    pub fn handle_child(
        &self,
        snap: &Snapshot,
        cmd: Command,
        clock: &dyn Clock,
    ) -> Result<Vec<Change>, DomainError> {
        if !child_originable(&cmd) {
            return Err(DomainError::BadCommandForActor);
        }
        self.handle(snap, cmd, clock)
    }
}

impl Engine for DomainEngine {
    fn handle(
        &self,
        snap: &Snapshot,
        cmd: Command,
        clock: &dyn Clock,
    ) -> Result<Vec<Change>, DomainError> {
        match cmd {
            // ── Authoring → Put*/SetXActive (T-0002) ────────────────────────────
            Command::DefineQuest(_)
            | Command::ArchiveQuest(_)
            | Command::DefineItem(_)
            | Command::ArchiveItem(_)
            | Command::DefineAchievement(_)
            | Command::ArchiveAchievement(_) => authoring::handle(snap, cmd, clock),

            // ── Claim & review (T-0003) ─────────────────────────────────────────
            Command::SubmitClaim { .. } | Command::ReviewClaim { .. } => {
                claims::handle(snap, cmd, clock)
            }

            // ── Redemption & ledger (T-0005) ────────────────────────────────────
            Command::RequestRedemption { .. }
            | Command::ReviewRedemption { .. }
            | Command::RedeemItem { .. }
            | Command::AdjustPoints { .. } => redemption::handle(snap, cmd, clock),
        }
    }
}

// Per-family handlers. Stubs in T-0001 (return no Changes); replaced with real validation
// and event/Change emission in the named tasks.

mod authoring {
    use super::*;
    pub fn handle(
        _snap: &Snapshot,
        _cmd: Command,
        _clock: &dyn Clock,
    ) -> Result<Vec<Change>, DomainError> {
        // TODO(T-0002): validate definitions (incl. quest assignment) → Put*/SetXActive.
        Ok(vec![])
    }
}

mod claims {
    use super::*;
    pub fn handle(
        _snap: &Snapshot,
        _cmd: Command,
        _clock: &dyn Clock,
    ) -> Result<Vec<Change>, DomainError> {
        // TODO(T-0003): subject validation, assignment/Race gating, auto-approve,
        // snapshot-at-approval, single-review, actor stamping, claim idempotency.
        Ok(vec![])
    }
}

mod redemption {
    use super::*;
    pub fn handle(
        _snap: &Snapshot,
        _cmd: Command,
        _clock: &dyn Clock,
    ) -> Result<Vec<Change>, DomainError> {
        // TODO(T-0005): request/review/redeem/adjust; can_redeem at commit; idempotency.
        Ok(vec![])
    }
}
