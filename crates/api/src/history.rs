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

use domain_core::contract::{Currency, Event};

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
    /// The acting Knight, if any (`None` = auto-approve / system / squire-initiated).
    pub actor: Option<i64>,
    pub quest_id: Option<i64>,
    pub item_id: Option<i64>,
    pub achievement_id: Option<i64>,
    /// Balance delta where the event bears one; `None` for claims/requests (no delta yet).
    pub amount: Option<i64>,
    /// Currency for an `Adjusted` entry; `None` otherwise.
    pub currency: Option<Currency>,
    pub reason: Option<String>,
}

impl HistoryEntryDto {
    /// Flatten one domain [`Event`] into a display-ready feed entry.
    fn from_event(e: &Event) -> Self {
        // Shared defaults; each arm overrides only what it carries.
        let base = |at: i64, kind: &str, squire: u128| HistoryEntryDto {
            at,
            kind: kind.to_string(),
            squire: squire as i64,
            actor: None,
            quest_id: None,
            item_id: None,
            achievement_id: None,
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
    let events = state.store.lock().expect("store mutex poisoned").recent_events(limit);
    Json(events.iter().map(HistoryEntryDto::from_event).collect())
}
