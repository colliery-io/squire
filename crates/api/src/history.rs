//! Household activity history on the LAN api (SQUIRE-T-0112): a Knight-gated, read-only feed of the
//! most recent household events, flattened to a codegen-friendly DTO for the parent phone — at
//! parity with the Keep's event-log inspector, but **household-wide** (a feed, not per-id drill-down).
//!
//! The Android Knight history screen renders these directly; it resolves quest/item display names
//! from data it already holds (the household-review), so this DTO carries ids, not names.

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;

use domain_core::claim_meta;
use domain_core::contract::{Currency, Event, Repository, Snapshot};

use crate::auth::RequireKnight;
use crate::AppState;

/// `GET /admin/history` query: how many recent entries to return (default 50, clamped to 1..=200).
#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    #[serde(default)]
    pub limit: Option<i64>,
}

/// One flattened activity entry for the Knight history feed. Every domain [`Event`] maps to exactly
/// one of these. Ids are int64 on the wire (the repo's fits-in-i64 invariant). `amount` carries the
/// balance delta where the event bears one (approve `+points`, redeem `−cost`, unlock `+bonus`,
/// adjust `±amount`); `currency` is set only for an `Adjusted` entry (other events are always coins).
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct HistoryEntryDto {
    /// Event time, unix millis.
    pub at: i64,
    /// Variant tag: `Claimed | Approved | Rejected | Redeemed | Requested | RedemptionRejected | Unlocked | Adjusted`.
    pub kind: String,
    /// The Squire the event is scoped to.
    pub squire: i64,
    /// The Squire's display name, resolved server-side from `squire` (SQUIRE-T-0121). `None` only if
    /// the id no longer maps to any household member.
    pub squire_name: Option<String>,
    /// The acting Knight, if any (`None` = auto-approve / system / squire-initiated).
    pub actor: Option<i64>,
    /// The acting Knight's display name, resolved from `actor` (SQUIRE-T-0121) — so the feed can say
    /// "approved by Dad" instead of leaking a raw id. `None` when there's no actor.
    pub actor_name: Option<String>,
    pub quest_id: Option<i64>,
    /// The quest's title, resolved from `quest_id` (SQUIRE-T-0121) — present for claim/approval rows.
    pub quest_title: Option<String>,
    pub item_id: Option<i64>,
    /// The reward's name, resolved from `item_id` (SQUIRE-T-0121) — present for redemption rows.
    pub item_title: Option<String>,
    pub achievement_id: Option<i64>,
    /// The achievement's name, resolved from `achievement_id` (SQUIRE-T-0121) — present for unlock rows.
    pub achievement_name: Option<String>,
    /// Balance delta where the event bears one; `None` for claims/requests (no delta yet).
    pub amount: Option<i64>,
    /// Currency for an `Adjusted` entry; `None` otherwise.
    pub currency: Option<Currency>,
    pub reason: Option<String>,
}

impl HistoryEntryDto {
    /// Flatten one domain [`Event`] into a display-ready feed entry, resolving every id to a display
    /// name/title from `snap` so the phone can render "Matrim earned 2 coins for Make your bed —
    /// approved by Dad" instead of bare ids (SQUIRE-T-0121).
    fn from_event(snap: &Snapshot, e: &Event) -> Self {
        let mut dto = Self::flatten(e);
        // The earn event is claim-keyed, so `flatten` leaves its `quest_id` empty — recover it from
        // the originating claim so approvals can name their quest.
        if dto.quest_id.is_none() {
            if let Event::CompletionApproved { claim_id, .. } = e {
                dto.quest_id = claim_meta(snap, *claim_id).map(|(_, q, _)| q.0 as i64);
            }
        }
        // Resolve ids → display strings. People are looked up across ALL users (no active/role
        // filter), so renamed or deactivated members referenced by old events still resolve and never
        // leak as a bare id.
        dto.squire_name = user_name(snap, dto.squire);
        dto.actor_name = dto.actor.and_then(|a| user_name(snap, a));
        dto.quest_title = dto
            .quest_id
            .and_then(|q| snap.quests.iter().find(|x| x.id.0 as i64 == q).map(|x| x.title.clone()));
        dto.item_title = dto
            .item_id
            .and_then(|i| snap.items.iter().find(|x| x.id.0 as i64 == i).map(|x| x.name.clone()));
        dto.achievement_name = dto.achievement_id.and_then(|a| {
            snap.achievements.iter().find(|x| x.id.0 as i64 == a).map(|x| x.name.clone())
        });
        dto
    }

    /// The id-only flattening: one arm per [`Event`] variant. Display names are filled in by
    /// [`Self::from_event`].
    fn flatten(e: &Event) -> Self {
        // Shared defaults; each arm overrides only what it carries.
        let base = |at: i64, kind: &str, squire: u128| HistoryEntryDto {
            at,
            kind: kind.to_string(),
            squire: squire as i64,
            squire_name: None,
            actor: None,
            actor_name: None,
            quest_id: None,
            quest_title: None,
            item_id: None,
            item_title: None,
            achievement_id: None,
            achievement_name: None,
            amount: None,
            currency: None,
            reason: None,
        };
        match e {
            Event::CompletionClaimed { squire, quest_id, at, .. } => HistoryEntryDto {
                quest_id: Some(quest_id.0 as i64),
                ..base(at.0, "Claimed", squire.0)
            },
            Event::CompletionApproved { squire, actor, points, at, .. } => HistoryEntryDto {
                actor: actor.map(|u| u.0 as i64),
                amount: Some(*points as i64),
                ..base(at.0, "Approved", squire.0)
            },
            Event::CompletionRejected { squire, actor, reason, at, .. } => HistoryEntryDto {
                actor: actor.map(|u| u.0 as i64),
                reason: reason.clone(),
                ..base(at.0, "Rejected", squire.0)
            },
            Event::ItemRedeemed { squire, actor, item_id, cost, at, .. } => HistoryEntryDto {
                actor: actor.map(|u| u.0 as i64),
                item_id: Some(item_id.0 as i64),
                amount: Some(-(*cost as i64)),
                ..base(at.0, "Redeemed", squire.0)
            },
            Event::AchievementUnlocked { squire, id, bonus, at } => HistoryEntryDto {
                achievement_id: Some(id.0 as i64),
                amount: Some(*bonus as i64),
                ..base(at.0, "Unlocked", squire.0)
            },
            Event::Adjusted { squire, actor, currency, amount, reason, at, .. } => HistoryEntryDto {
                actor: actor.map(|u| u.0 as i64),
                amount: Some(*amount),
                currency: Some(*currency),
                reason: Some(reason.clone()),
                ..base(at.0, "Adjusted", squire.0)
            },
            Event::RedemptionRequested { squire, item_id, at, .. } => HistoryEntryDto {
                item_id: Some(item_id.0 as i64),
                ..base(at.0, "Requested", squire.0)
            },
            Event::RedemptionRejected { squire, actor, reason, at, .. } => HistoryEntryDto {
                actor: actor.map(|u| u.0 as i64),
                reason: reason.clone(),
                ..base(at.0, "RedemptionRejected", squire.0)
            },
            Event::CashOutRequested { squire, amount, at, .. } => HistoryEntryDto {
                amount: Some(*amount),
                currency: Some(Currency::Cash),
                ..base(at.0, "CashOutRequested", squire.0)
            },
            Event::CashOutApproved { squire, actor, amount, at, .. } => HistoryEntryDto {
                actor: actor.map(|u| u.0 as i64),
                amount: Some(-*amount),
                currency: Some(Currency::Cash),
                ..base(at.0, "CashedOut", squire.0)
            },
            Event::CashOutRejected { squire, actor, reason, at, .. } => HistoryEntryDto {
                actor: actor.map(|u| u.0 as i64),
                reason: reason.clone(),
                ..base(at.0, "CashOutRejected", squire.0)
            },
        }
    }
}

/// A member's display name by id, across ALL household users (active or not) so historical events
/// referencing a renamed/deactivated member still resolve (SQUIRE-T-0121).
fn user_name(snap: &Snapshot, id: i64) -> Option<String> {
    snap.users.iter().find(|u| u.id.0 as i64 == id).map(|u| u.display_name.clone())
}

/// `GET /admin/history?limit=N` (Knight-only) — the most recent household events, newest first,
/// flattened for the phone. Default 50, clamped to 1..=200.
#[utoipa::path(
    get, path = "/admin/history", tag = "knight",
    params(
        ("X-Household" = String, Header, description = "Tenant household handle"),
        ("limit" = Option<i64>, Query, description = "Max entries (default 50, capped 200)"),
    ),
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Recent household activity, newest first", body = Vec<HistoryEntryDto>)),
)]
pub async fn history(
    State(state): State<Arc<AppState>>,
    _knight: RequireKnight,
    Query(q): Query<HistoryQuery>,
) -> Json<Vec<HistoryEntryDto>> {
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    // One lock: the recent events plus a snapshot to resolve their ids → names/titles.
    let (events, snap) = {
        let store = state.store.lock().expect("store mutex poisoned");
        (store.recent_events(limit), store.snapshot())
    };
    Json(events.iter().map(|e| HistoryEntryDto::from_event(&snap, e)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain_core::contract::{
        Assignment, Availability, Cadence, ClaimId, Completion, Date, ItemId, Quest, QuestId,
        RedeemableItem, Role, Schedule, Timestamp, User, UserId,
    };

    /// A household with Dad (Knight 1) + Matrim (Squire 2), one quest, one reward, and the events of
    /// Matrim claiming the quest then Dad approving it (plus a direct redeem).
    fn snap() -> Snapshot {
        let quest = Quest {
            id: QuestId(100),
            title: "Make your bed".into(),
            description: None,
            category: None,
            reward: 2,
            cash: 0,
            cadence: Cadence::Recurring(Schedule::Daily),
            assignment: Assignment::AllSquires,
            completion: Completion::EachAssignee,
            auto_approve: false,
            repeatable_within_day: false,
            active: true,
            icon: None,
        };
        let item = RedeemableItem {
            id: ItemId(200),
            name: "Ice cream".into(),
            description: None,
            cost: 3,
            gate: None,
            availability: Availability::Repeatable,
            active: true,
            icon: None,
        };
        Snapshot {
            users: vec![
                User { id: UserId(1), role: Role::Knight, display_name: "Dad".into(), active: true },
                User { id: UserId(2), role: Role::Squire, display_name: "Matrim".into(), active: true },
            ],
            quests: vec![quest],
            items: vec![item],
            achievements: vec![],
            events: vec![
                Event::CompletionClaimed { claim_id: ClaimId(9), squire: UserId(2), quest_id: QuestId(100), on: Date(1), at: Timestamp(10) },
                Event::CompletionApproved { claim_id: ClaimId(9), squire: UserId(2), actor: Some(UserId(1)), points: 2, at: Timestamp(20) },
                Event::ItemRedeemed { request_id: None, command_id: None, squire: UserId(2), actor: Some(UserId(1)), item_id: ItemId(200), cost: 3, at: Timestamp(30) },
            ],
        }
    }

    #[test]
    fn approval_resolves_actor_and_backfills_quest_title() {
        let s = snap();
        let approved = &s.events[1];
        let dto = HistoryEntryDto::from_event(&s, approved);
        assert_eq!(dto.kind, "Approved");
        assert_eq!(dto.squire_name.as_deref(), Some("Matrim"));
        assert_eq!(dto.actor_name.as_deref(), Some("Dad"), "the approving Knight is named");
        // CompletionApproved is claim-keyed; the quest is recovered from the originating claim.
        assert_eq!(dto.quest_id, Some(100));
        assert_eq!(dto.quest_title.as_deref(), Some("Make your bed"));
        assert_eq!(dto.amount, Some(2));
    }

    #[test]
    fn claim_and_redeem_resolve_their_subjects() {
        let s = snap();
        let claimed = HistoryEntryDto::from_event(&s, &s.events[0]);
        assert_eq!(claimed.quest_title.as_deref(), Some("Make your bed"));
        assert_eq!(claimed.actor_name, None, "a claim has no actor");

        let redeemed = HistoryEntryDto::from_event(&s, &s.events[2]);
        assert_eq!(redeemed.item_title.as_deref(), Some("Ice cream"));
        assert_eq!(redeemed.actor_name.as_deref(), Some("Dad"));
    }

    /// A renamed/deactivated member referenced by an old event still resolves to a name (it is not
    /// filtered out), so the feed never leaks a bare id.
    #[test]
    fn inactive_member_still_resolves() {
        let mut s = snap();
        s.users[1].active = false; // Matrim deactivated after the events
        let dto = HistoryEntryDto::from_event(&s, &s.events[1]);
        assert_eq!(dto.squire_name.as_deref(), Some("Matrim"));
    }
}
