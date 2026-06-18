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
    Assignment, Cadence, Clock, Command, Completion, Date, Quest, QuestId, Repository, Schedule,
    Snapshot, Weekday,
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
    /// Days for a `Weekly` cadence (≥1 required for `Weekly`).
    #[serde(default)]
    pub weekdays: Vec<WeekdayDto>,
    /// Optional due date (a `Date` day-count) for a `OneOff` cadence.
    pub due: Option<Date>,
    pub completion: CompletionDto,
    /// True = all squires (auto-includes ones added later); false = the explicit `squires` list.
    pub assign_all: bool,
    /// Explicit assignees when `assign_all` is false (≥1 active Squire required).
    #[serde(default)]
    pub squires: Vec<u64>,
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
    let cadence = match req.cadence {
        CadenceKind::Daily => Cadence::Recurring(Schedule::Daily),
        CadenceKind::Weekly => {
            let days: BTreeSet<Weekday> = req.weekdays.iter().map(|d| Weekday::from(*d)).collect();
            Cadence::Recurring(Schedule::Weekly { days })
        }
        CadenceKind::OneOff => Cadence::OneOff { due: req.due },
    };
    let assignment = if req.assign_all {
        Assignment::AllSquires
    } else {
        let set: BTreeSet<_> = req
            .squires
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
