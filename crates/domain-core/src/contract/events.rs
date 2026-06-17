use super::*;

// ─── ACTIVITY (append-only event log) ───────────────────────────────────────

/// Facts, in order. Only these move the balance. **Every event is scoped to one Squire**
/// (`squire`) — the household's log is the union of its Squires' sub-logs, so per-Squire
/// balance / streaks / `StateView` are derived by filtering on `squire`. Knight-committed
/// events also record the acting Knight (`actor`; `None` = auto-approve / system).
#[derive(Clone, Debug)]
pub enum Event {
    /// Proposed by/for a Squire; appended by the server on submit.
    CompletionClaimed  { claim_id: ClaimId, squire: UserId, quest_id: QuestId, on: Date, at: Timestamp },
    /// Committed by a Knight (or auto, if Quest.auto_approve) — point-bearing (+).
    /// `actor` = the approving Knight, or `None` when auto-approved.
    CompletionApproved { claim_id: ClaimId, squire: UserId, actor: Option<UserId>, points: Points, at: Timestamp },
    CompletionRejected { claim_id: ClaimId, squire: UserId, actor: Option<UserId>, reason: Option<String>, at: Timestamp },
    /// Committed spend — point-bearing (−). `request_id` is set when it came from a
    /// Squire's request (deduped via the request lifecycle); `command_id` is set for a
    /// direct Knight redeem (deduped via that key). For client-originated redeems exactly
    /// one is `Some`; a local Keep redeem may leave both `None`.
    ItemRedeemed       { request_id: Option<RequestId>, command_id: Option<CommandId>, squire: UserId, actor: Option<UserId>, item_id: ItemId, cost: Points, at: Timestamp },
    /// Emitted by the engine when a Squire first meets a criterion — sticky thereafter.
    AchievementUnlocked{ squire: UserId, id: AchievementId, bonus: Points, at: Timestamp },
    /// Knight override / correction targeting a Squire's balance. `command_id` is the
    /// idempotency key (always present: minted by the Knight's outbox, or by the Keep for
    /// a local adjustment) so a replay is deduped from the log.
    PointsAdjusted     { command_id: CommandId, squire: UserId, actor: Option<UserId>, amount: i64, reason: String, at: Timestamp },
    /// Squire-initiated, via the API. Reviewed exactly like a completion claim —
    /// it never spends on its own; approval emits `ItemRedeemed`.
    RedemptionRequested{ request_id: RequestId, squire: UserId, item_id: ItemId, at: Timestamp },
    RedemptionRejected { request_id: RequestId, squire: UserId, actor: Option<UserId>, reason: Option<String>, at: Timestamp },
}
