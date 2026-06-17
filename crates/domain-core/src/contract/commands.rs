use super::*;

// ─── COMMANDS (intents into the one validated door) ─────────────────────────

#[derive(Clone, Debug)]
pub enum Command {
    // Parent / admin. Authoring (Define*/Archive*) is local to the Keep and never on the
    // network. The review/redeem/adjust subset below is ALSO reachable over the network
    // under a separate parent credential (the Knight); either way the Keep is the sole
    // writer and every command passes this one door. See ADR SQUIRE-A-0001, spec SQUIRE-S-0003.
    DefineQuest(Quest),
    ArchiveQuest(QuestId),
    DefineItem(RedeemableItem),
    ArchiveItem(ItemId),
    DefineAchievement(Achievement),
    ArchiveAchievement(AchievementId),
    ReviewClaim { actor: UserId, claim_id: ClaimId, decision: Decision },          // actor = acting Knight; target Squire derived from the claim
    ReviewRedemption { actor: UserId, request_id: RequestId, decision: Decision }, // approve → ItemRedeemed; Squire from the request
    RedeemItem { command_id: CommandId, actor: UserId, squire: UserId, item_id: ItemId },          // Knight `actor` redeems for `squire`; command_id = idempotency key
    AdjustPoints { command_id: CommandId, actor: UserId, squire: UserId, amount: i64, reason: String }, // Knight `actor` adjusts `squire`; "add funds" when amount > 0

    // Squire — arrives via the API; `squire` is the authenticated user, filled by the API
    // from the caller's token (the wire request DTOs intentionally omit it):
    SubmitClaim { claim_id: ClaimId, squire: UserId, quest_id: QuestId, on: Date },
    RequestRedemption { request_id: RequestId, squire: UserId, item_id: ItemId },
}

#[derive(Clone, Debug)]
pub enum Decision { Approve, Reject { reason: Option<String> } }

/// What `handle` emits: a unified set of mutations across both storage shapes.
#[derive(Clone, Debug)]
pub enum Change {
    Append(Event),
    PutQuest(Quest),
    PutItem(RedeemableItem),
    PutAchievement(Achievement),
    SetQuestActive(QuestId, bool),
    SetItemActive(ItemId, bool),
    SetAchievementActive(AchievementId, bool),
    // Identity lifecycle — produced by the Identity/Registration component, not by
    // `Engine::handle` (which never manages users). Still applied via the single writer.
    PutUser(User),
    SetUserActive(UserId, bool),
}
