//! Quest authoring (SQUIRE-T-0026): create / edit / archive quests, **engine-direct**.
//!
//! Every write goes through [`KeepState::commit`] with `by` = the acting Knight ([`Operator`]),
//! so the store stamps last-editor audit (A-0007). `DefineQuest` is an upsert — the same `id`
//! edits an existing quest; archive is `ArchiveQuest` (a `SetQuestActive(_, false)`), never a
//! delete, so historical events stay valid (AR-2). Reward edits are **forward-only** by
//! construction: the engine snapshots the reward onto `CompletionApproved` at approval time, so
//! editing a quest's reward only affects *future* approvals — there is no path here to rewrite a
//! past payout (REQ-1.1.4 / AR-4).
//!
//! The request/response bodies use the domain [`Quest`] directly (its `serde` impls are enabled),
//! so the full `cadence` / `assignment` / `completion` shape round-trips without bespoke DTOs.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;

use domain_core::contract::{Command, Quest, QuestId};
use store::AuditCols;

use crate::{domain_status, KeepState, Operator};

/// A quest plus its last-editor audit ("who set / last changed this"), for the authoring list.
#[derive(Debug, Serialize)]
pub struct QuestRow {
    pub quest: Quest,
    pub audit: AuditView,
}

/// The audit columns surfaced to the UI (ids as decimal strings — `u128` is not a JSON-safe
/// integer for every client).
#[derive(Debug, Default, Serialize)]
pub struct AuditView {
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<AuditCols> for AuditView {
    fn from(a: AuditCols) -> Self {
        Self {
            created_by: a.created_by.map(|u| u.0.to_string()),
            updated_by: a.updated_by.map(|u| u.0.to_string()),
            created_at: a.created_at.0,
            updated_at: a.updated_at.0,
        }
    }
}

/// The id of a created / edited quest, echoed back (as a string — `u128`).
#[derive(Debug, Serialize)]
pub struct IdResp {
    pub id: String,
}

/// `GET /api/quests` (Knight-only) — every quest (active and archived), each with its last-editor
/// audit so the form can show "who set / last changed this".
pub async fn list_quests(State(state): State<Arc<KeepState>>, _op: Operator) -> Json<Vec<QuestRow>> {
    let snap = state.snapshot();
    let mut guard = state.store.lock().expect("store mutex poisoned");
    let rows = snap
        .quests
        .iter()
        .map(|q| {
            let audit = store::quest_audit(&mut guard.connection(), q.id)
                .ok()
                .flatten()
                .map(AuditView::from)
                .unwrap_or_default();
            QuestRow { quest: q.clone(), audit }
        })
        .collect();
    Json(rows)
}

/// `POST /api/quests` (Knight-only) — create or edit a quest via `DefineQuest` (upsert by `id`),
/// audited to the acting Knight. A bad definition is a 400 (`InvalidDefinition`); the full
/// `cadence`/`assignment`/`completion` shape is accepted as JSON.
pub async fn create_quest(
    State(state): State<Arc<KeepState>>,
    Operator(op): Operator,
    Json(quest): Json<Quest>,
) -> Result<Json<IdResp>, StatusCode> {
    let id = quest.id;
    state
        .commit(Some(op.user), Command::DefineQuest(quest))
        .map_err(domain_status)?;
    Ok(Json(IdResp { id: id.0.to_string() }))
}

/// `POST /api/quests/{id}/archive` (Knight-only) — archive a quest (`ArchiveQuest` →
/// `SetQuestActive(_, false)`), audited to the acting Knight. Never deletes; a missing quest is a
/// 404. The id segment is parsed from a string (`u128` is not a supported path integer).
pub async fn archive_quest(
    State(state): State<Arc<KeepState>>,
    Operator(op): Operator,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let qid = QuestId(id.trim().parse().map_err(|_| StatusCode::BAD_REQUEST)?);
    state
        .commit(Some(op.user), Command::ArchiveQuest(qid))
        .map_err(domain_status)?;
    Ok(StatusCode::NO_CONTENT)
}
