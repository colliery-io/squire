//! Achievement authoring (SQUIRE-T-0027): create / edit / archive [`Achievement`]s,
//! **engine-direct**. Choose a `criterion` (Streak / TotalCompletions / PointsEarned), its scope,
//! and the bonus points. Writes go through [`KeepState::commit`] with `by` = the acting Knight;
//! archive is `ArchiveAchievement` (never a delete). The list view surfaces last-editor audit.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;

use domain_core::contract::{Achievement, AchievementId, Command};

use crate::quests::{AuditView, IdResp};
use crate::{domain_status, KeepState, Operator};

/// An achievement plus its last-editor audit, for the authoring list.
#[derive(Debug, Serialize)]
pub struct AchievementRow {
    pub achievement: Achievement,
    pub audit: AuditView,
}

/// `GET /api/achievements` (Knight-only) — every achievement with its last-editor audit.
pub async fn list_achievements(
    State(state): State<Arc<KeepState>>,
    _op: Operator,
) -> Json<Vec<AchievementRow>> {
    let snap = state.snapshot();
    let mut guard = state.store.lock().expect("store mutex poisoned");
    let rows = snap
        .achievements
        .iter()
        .map(|a| {
            let audit = store::achievement_audit(&mut guard.connection(), a.id)
                .ok()
                .flatten()
                .map(AuditView::from)
                .unwrap_or_default();
            AchievementRow { achievement: a.clone(), audit }
        })
        .collect();
    Json(rows)
}

/// `POST /api/achievements` (Knight-only) — create or edit an achievement via `DefineAchievement`
/// (upsert by `id`), audited to the acting Knight. A bad definition is a 400.
pub async fn create_achievement(
    State(state): State<Arc<KeepState>>,
    Operator(op): Operator,
    Json(achievement): Json<Achievement>,
) -> Result<Json<IdResp>, StatusCode> {
    let id = achievement.id;
    state
        .commit(Some(op.user), Command::DefineAchievement(achievement))
        .map_err(domain_status)?;
    Ok(Json(IdResp { id: id.0.to_string() }))
}

/// `POST /api/achievements/{id}/archive` (Knight-only) — archive (`ArchiveAchievement`), never
/// delete; a missing achievement is a 404.
pub async fn archive_achievement(
    State(state): State<Arc<KeepState>>,
    Operator(op): Operator,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let aid = AchievementId(id.trim().parse().map_err(|_| StatusCode::BAD_REQUEST)?);
    state
        .commit(Some(op.user), Command::ArchiveAchievement(aid))
        .map_err(domain_status)?;
    Ok(StatusCode::NO_CONTENT)
}
