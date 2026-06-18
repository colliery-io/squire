//! Knight quest authoring over the LAN api (SQUIRE-T-0064): create / list / archive quests from the
//! parent phone, gated by [`RequireKnight`]. Mirrors the Keep's authoring (engine-direct
//! `DefineQuest` / `ArchiveQuest`, audited to the acting Knight) but uses **flat, codegen-friendly
//! DTOs** instead of the domain [`Quest`] — the externally-tagged `Cadence`/`Assignment` enums
//! mangle in the generated SDK, so the wire carries a discriminant + flat fields and the handler
//! reconstructs the domain shape (the same FLAT-STRUCT pattern as `DecisionDto`).

use std::collections::BTreeSet;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use domain_core::contract::{
    Achievement, AchievementId, Assignment, Cadence, Category, Clock, Command, Completion,
    Criterion, Date, Quest, QuestId, Repository, Schedule, Scope, Snapshot, StreakBasis, Weekday,
};

use crate::auth::RequireKnight;
use crate::squire::{domain_status, handle_command};
use crate::AppState;

/// Quest cadence on the wire (flat). `weekdays` applies to `Weekly`; `due` (a `Date` day-count) to
/// `OneOff`; `Daily` uses neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum CadenceKind {
    Daily,
    Weekly,
    OneOff,
}

/// A weekday on the wire (mirrors the domain `Weekday`, flat string enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum WeekdayDto {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

impl From<WeekdayDto> for Weekday {
    fn from(d: WeekdayDto) -> Self {
        match d {
            WeekdayDto::Mon => Weekday::Mon,
            WeekdayDto::Tue => Weekday::Tue,
            WeekdayDto::Wed => Weekday::Wed,
            WeekdayDto::Thu => Weekday::Thu,
            WeekdayDto::Fri => Weekday::Fri,
            WeekdayDto::Sat => Weekday::Sat,
            WeekdayDto::Sun => Weekday::Sun,
        }
    }
}

/// How an assigned occurrence is satisfied, on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum CompletionDto {
    EachAssignee,
    Race,
}

impl From<CompletionDto> for Completion {
    fn from(c: CompletionDto) -> Self {
        match c {
            CompletionDto::EachAssignee => Completion::EachAssignee,
            CompletionDto::Race => Completion::Race,
        }
    }
}

impl From<Completion> for CompletionDto {
    fn from(c: Completion) -> Self {
        match c {
            Completion::EachAssignee => CompletionDto::EachAssignee,
            Completion::Race => CompletionDto::Race,
        }
    }
}

/// `POST /admin/quests` request — a flat quest definition authored from the parent phone.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateQuestReq {
    /// Existing quest id to edit (upsert), or null/absent to create a new one (the server assigns).
    pub id: Option<QuestId>,
    pub title: String,
    pub reward: i64,
    /// Optional free-text grouping (e.g. "Bedroom").
    pub category: Option<String>,
    pub cadence: CadenceKind,
    /// Days for a `Weekly` cadence (≥1 required for `Weekly`). `null`/absent ⇒ none.
    #[serde(default)]
    pub weekdays: Option<Vec<WeekdayDto>>,
    /// Optional due date (a `Date` day-count) for a `OneOff` cadence.
    pub due: Option<Date>,
    pub completion: CompletionDto,
    /// True = all squires (auto-includes ones added later); false = the explicit `squires` list.
    pub assign_all: bool,
    /// Explicit assignees when `assign_all` is false (≥1 active Squire required). `null`/absent ⇒ none.
    #[serde(default)]
    pub squires: Option<Vec<u64>>,
    pub repeatable_within_day: bool,
    pub auto_approve: bool,
}

/// The id of a created / edited quest, echoed back.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CreatedQuest {
    pub id: QuestId,
}

/// A quest in the authoring list — flat, with cadence/assignment **labels computed server-side**
/// (so the phone list needs no enum decoding).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct QuestSummaryDto {
    pub id: QuestId,
    pub title: String,
    pub reward: i64,
    pub category: Option<String>,
    /// e.g. "Daily", "Mon/Wed/Fri", "One-time".
    pub cadence_label: String,
    /// e.g. "All squires", "Gawain, Percival".
    pub assignment_label: String,
    pub completion: CompletionDto,
    pub repeatable_within_day: bool,
    pub auto_approve: bool,
    pub active: bool,
}

/// `POST /admin/quests` (RequireKnight) — create or edit a quest. A bad definition (empty Weekly
/// days, empty/inactive assignees) is a 400.
#[utoipa::path(
    post,
    path = "/admin/quests",
    tag = "knight",
    security(("bearer_auth" = [])),
    params(("X-Household" = String, Header, description = "Opaque household handle routing the request to its tenant")),
    request_body = CreateQuestReq,
    responses(
        (status = 200, description = "The created/edited quest id", body = CreatedQuest),
        (status = 400, description = "Invalid quest definition"),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Authenticated but not a Knight"),
    ),
)]
pub async fn create_quest(
    State(state): State<Arc<AppState>>,
    RequireKnight(principal): RequireKnight,
    Json(req): Json<CreateQuestReq>,
) -> Result<Json<CreatedQuest>, StatusCode> {
    let weekdays = req.weekdays.unwrap_or_default();
    let squires = req.squires.unwrap_or_default();
    let cadence = match req.cadence {
        CadenceKind::Daily => Cadence::Recurring(Schedule::Daily),
        CadenceKind::Weekly => {
            let days: BTreeSet<Weekday> = weekdays.iter().map(|d| Weekday::from(*d)).collect();
            Cadence::Recurring(Schedule::Weekly { days })
        }
        CadenceKind::OneOff => Cadence::OneOff { due: req.due },
    };
    let assignment = if req.assign_all {
        Assignment::AllSquires
    } else {
        let set: BTreeSet<_> = squires
            .iter()
            .map(|u| domain_core::contract::UserId(u128::from(*u)))
            .collect();
        Assignment::Squires(set)
    };
    // New quests get a time-based id (like the Keep's Date.now()); edits keep the supplied id.
    let id = req
        .id
        .unwrap_or_else(|| QuestId(state.clock.now().0 as u128));
    let quest = Quest {
        id,
        title: req.title,
        description: None,
        category: req.category.filter(|c| !c.is_empty()).map(domain_core::contract::Category),
        reward: req.reward.max(0) as u32,
        cadence,
        assignment,
        completion: req.completion.into(),
        auto_approve: req.auto_approve,
        repeatable_within_day: req.repeatable_within_day,
        active: true,
        icon: None,
    };
    handle_command(&state, Some(principal.user), Command::DefineQuest(quest)).map_err(domain_status)?;
    Ok(Json(CreatedQuest { id }))
}

/// `GET /admin/quests` (RequireKnight) — every quest (active + archived) as flat summaries with
/// cadence/assignment labels resolved against current members.
#[utoipa::path(
    get,
    path = "/admin/quests",
    tag = "knight",
    security(("bearer_auth" = [])),
    params(("X-Household" = String, Header, description = "Opaque household handle routing the request to its tenant")),
    responses(
        (status = 200, description = "All quests, flat", body = [QuestSummaryDto]),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Authenticated but not a Knight"),
    ),
)]
pub async fn list_quests(
    State(state): State<Arc<AppState>>,
    RequireKnight(_principal): RequireKnight,
) -> Json<Vec<QuestSummaryDto>> {
    let snap = state.store.lock().expect("store mutex poisoned").snapshot();
    let rows = snap
        .quests
        .iter()
        .map(|q| QuestSummaryDto {
            id: q.id,
            title: q.title.clone(),
            reward: q.reward as i64,
            category: q.category.as_ref().map(|c| c.0.clone()),
            cadence_label: cadence_label(&q.cadence),
            assignment_label: assignment_label(&snap, &q.assignment),
            completion: q.completion.into(),
            repeatable_within_day: q.repeatable_within_day,
            auto_approve: q.auto_approve,
            active: q.active,
        })
        .collect();
    Json(rows)
}

/// `POST /admin/quests/{id}/archive` (RequireKnight) — archive a quest (never deletes). 404 if the
/// quest does not exist.
#[utoipa::path(
    post,
    path = "/admin/quests/{id}/archive",
    tag = "knight",
    security(("bearer_auth" = [])),
    params(
        ("X-Household" = String, Header, description = "Opaque household handle routing the request to its tenant"),
        ("id" = i64, Path, description = "Quest id"),
    ),
    responses(
        (status = 204, description = "Archived"),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Authenticated but not a Knight"),
        (status = 404, description = "Quest not found"),
    ),
)]
pub async fn archive_quest(
    State(state): State<Arc<AppState>>,
    RequireKnight(principal): RequireKnight,
    Path(id): Path<u64>,
) -> Result<StatusCode, StatusCode> {
    let qid = QuestId(u128::from(id));
    handle_command(&state, Some(principal.user), Command::ArchiveQuest(qid)).map_err(domain_status)?;
    Ok(StatusCode::NO_CONTENT)
}

/// A short human label for a cadence (mirrors the Keep's list summary).
fn cadence_label(c: &Cadence) -> String {
    match c {
        Cadence::Recurring(Schedule::Daily) => "Daily".to_string(),
        Cadence::Recurring(Schedule::Weekly { days }) => days
            .iter()
            .map(weekday_short)
            .collect::<Vec<_>>()
            .join("/"),
        Cadence::Recurring(Schedule::EveryNDays { n, .. }) => format!("Every {n} days"),
        Cadence::OneOff { .. } => "One-time".to_string(),
    }
}

fn weekday_short(d: &Weekday) -> &'static str {
    match d {
        Weekday::Mon => "Mon",
        Weekday::Tue => "Tue",
        Weekday::Wed => "Wed",
        Weekday::Thu => "Thu",
        Weekday::Fri => "Fri",
        Weekday::Sat => "Sat",
        Weekday::Sun => "Sun",
    }
}

/// A short human label for an assignment, resolving squire ids to display names.
fn assignment_label(snap: &Snapshot, a: &Assignment) -> String {
    match a {
        Assignment::AllSquires => "All squires".to_string(),
        Assignment::Squires(set) => set
            .iter()
            .map(|uid| {
                snap.users
                    .iter()
                    .find(|u| u.id == *uid)
                    .map(|u| u.display_name.clone())
                    .unwrap_or_else(|| format!("#{}", uid.0))
            })
            .collect::<Vec<_>>()
            .join(", "),
    }
}

// ─── Achievement authoring (SQUIRE-T-0072) ───────────────────────────────────────────────────────
// Same flat-DTO pattern as quests above: the domain `Criterion`/`Scope` are externally-tagged enums
// that mangle in codegen, so the wire carries a discriminant + flat fields and the handler rebuilds
// the domain shape. Validation (zero length/count/total, blank category, missing quest) is enforced
// by the engine's `validate_achievement` and surfaced via `domain_status`.

/// Achievement criterion kind on the wire (flat).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum AchCriterionKind {
    Streak,
    TotalCompletions,
    PointsEarned,
}

/// Achievement scope kind on the wire (flat).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum AchScopeKind {
    Any,
    Quest,
    Category,
}

/// Streak basis on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum AchBasisKind {
    ScheduledOccurrences,
    CalendarDays,
}

/// `POST /admin/achievements` request — a flat achievement definition authored from the phone.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateAchievementReq {
    /// Existing id to edit (upsert), or null/absent to create (the server assigns).
    pub id: Option<AchievementId>,
    pub name: String,
    pub criterion: AchCriterionKind,
    /// Scope for Streak/TotalCompletions (ignored for PointsEarned).
    pub scope: AchScopeKind,
    /// Quest id when `scope = Quest`.
    pub scope_quest: Option<QuestId>,
    /// Category label when `scope = Category` (must be non-blank).
    pub scope_category: Option<String>,
    /// Streak length (≥1) when criterion = Streak.
    pub length: Option<i64>,
    /// Streak basis when criterion = Streak.
    pub basis: Option<AchBasisKind>,
    /// Count (≥1) when criterion = TotalCompletions.
    pub count: Option<i64>,
    /// Total points (≥1) when criterion = PointsEarned.
    pub total: Option<i64>,
    /// Bonus points awarded (may be 0 for a pure unlock).
    pub bonus: i64,
}

/// The id of a created / edited achievement, echoed back.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CreatedAchievement {
    pub id: AchievementId,
}

/// An achievement in the authoring list — flat, with a server-computed summary.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AchievementSummaryDto {
    pub id: AchievementId,
    pub name: String,
    /// e.g. "7-day streak · Bedroom", "20 completions · Kitchen", "100 points".
    pub summary: String,
    pub bonus: i64,
    pub active: bool,
}

/// `POST /admin/achievements` (RequireKnight) — create or edit an achievement.
#[utoipa::path(
    post,
    path = "/admin/achievements",
    tag = "knight",
    security(("bearer_auth" = [])),
    params(("X-Household" = String, Header, description = "Opaque household handle routing the request to its tenant")),
    request_body = CreateAchievementReq,
    responses(
        (status = 200, description = "The created/edited achievement id", body = CreatedAchievement),
        (status = 400, description = "Invalid achievement definition"),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Authenticated but not a Knight"),
        (status = 404, description = "Scope quest not found"),
    ),
)]
pub async fn create_achievement(
    State(state): State<Arc<AppState>>,
    RequireKnight(principal): RequireKnight,
    Json(req): Json<CreateAchievementReq>,
) -> Result<Json<CreatedAchievement>, StatusCode> {
    let scope = match req.scope {
        AchScopeKind::Any => Scope::Any,
        AchScopeKind::Quest => Scope::Quest(req.scope_quest.ok_or(StatusCode::BAD_REQUEST)?),
        AchScopeKind::Category => Scope::Category(Category(req.scope_category.clone().unwrap_or_default())),
    };
    let criterion = match req.criterion {
        AchCriterionKind::PointsEarned => Criterion::PointsEarned { total: req.total.unwrap_or(0).max(0) as u32 },
        AchCriterionKind::TotalCompletions => {
            Criterion::TotalCompletions { scope, count: req.count.unwrap_or(0).max(0) as u32 }
        }
        AchCriterionKind::Streak => Criterion::Streak {
            scope,
            length: req.length.unwrap_or(0).max(0) as u32,
            basis: match req.basis.unwrap_or(AchBasisKind::CalendarDays) {
                AchBasisKind::ScheduledOccurrences => StreakBasis::ScheduledOccurrences,
                AchBasisKind::CalendarDays => StreakBasis::CalendarDays,
            },
        },
    };
    let id = req.id.unwrap_or_else(|| AchievementId(state.clock.now().0 as u128));
    let achievement = Achievement {
        id,
        name: req.name,
        description: None,
        criterion,
        bonus_points: req.bonus.max(0) as u32,
        active: true,
    };
    handle_command(&state, Some(principal.user), Command::DefineAchievement(achievement)).map_err(domain_status)?;
    Ok(Json(CreatedAchievement { id }))
}

/// `GET /admin/achievements` (RequireKnight) — all achievements as flat summaries.
#[utoipa::path(
    get,
    path = "/admin/achievements",
    tag = "knight",
    security(("bearer_auth" = [])),
    params(("X-Household" = String, Header, description = "Opaque household handle routing the request to its tenant")),
    responses(
        (status = 200, description = "All achievements, flat", body = [AchievementSummaryDto]),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Authenticated but not a Knight"),
    ),
)]
pub async fn list_achievements(
    State(state): State<Arc<AppState>>,
    RequireKnight(_principal): RequireKnight,
) -> Json<Vec<AchievementSummaryDto>> {
    let snap = state.store.lock().expect("store mutex poisoned").snapshot();
    let rows = snap
        .achievements
        .iter()
        .map(|a| AchievementSummaryDto {
            id: a.id,
            name: a.name.clone(),
            summary: achievement_summary(&snap, &a.criterion),
            bonus: a.bonus_points as i64,
            active: a.active,
        })
        .collect();
    Json(rows)
}

/// `POST /admin/achievements/{id}/archive` (RequireKnight) — archive (never delete). 404 if absent.
#[utoipa::path(
    post,
    path = "/admin/achievements/{id}/archive",
    tag = "knight",
    security(("bearer_auth" = [])),
    params(
        ("X-Household" = String, Header, description = "Opaque household handle routing the request to its tenant"),
        ("id" = i64, Path, description = "Achievement id"),
    ),
    responses(
        (status = 204, description = "Archived"),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Authenticated but not a Knight"),
        (status = 404, description = "Achievement not found"),
    ),
)]
pub async fn archive_achievement(
    State(state): State<Arc<AppState>>,
    RequireKnight(principal): RequireKnight,
    Path(id): Path<u64>,
) -> Result<StatusCode, StatusCode> {
    let aid = AchievementId(u128::from(id));
    handle_command(&state, Some(principal.user), Command::ArchiveAchievement(aid)).map_err(domain_status)?;
    Ok(StatusCode::NO_CONTENT)
}

/// A short human summary of an achievement criterion (mirrors the Keep + library).
fn achievement_summary(snap: &Snapshot, c: &Criterion) -> String {
    let scope_label = |s: &Scope| -> String {
        match s {
            Scope::Any => "any".to_string(),
            Scope::Category(cat) => cat.0.clone(),
            Scope::Quest(qid) => snap
                .quests
                .iter()
                .find(|q| q.id == *qid)
                .map(|q| q.title.clone())
                .unwrap_or_else(|| format!("quest #{}", qid.0)),
        }
    };
    match c {
        Criterion::PointsEarned { total } => format!("{total} points"),
        Criterion::TotalCompletions { scope, count } => format!("{count} completions · {}", scope_label(scope)),
        Criterion::Streak { scope, length, .. } => format!("{length}-day streak · {}", scope_label(scope)),
    }
}
