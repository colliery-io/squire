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
    Achievement, AchievementId, Assignment, Availability, Cadence, Category, Change, Clock, Command,
    Completion, Criterion, Date, ItemId, Quest, QuestId, RedeemableItem, Repository, Role, Schedule,
    Scope, Snapshot, StreakBasis, User, UserId, Weekday,
};

use crate::auth::RequireKnight;
use crate::squire::{domain_status, handle_command};
use crate::AppState;

/// Quest cadence on the wire (flat). `weekdays` applies to `Weekly`; `due` (a `Date` day-count) to
/// `OneOff`; `Daily` uses neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
pub enum CadenceKind {
    #[default]
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

impl From<Weekday> for WeekdayDto {
    fn from(d: Weekday) -> Self {
        match d {
            Weekday::Mon => WeekdayDto::Mon,
            Weekday::Tue => WeekdayDto::Tue,
            Weekday::Wed => WeekdayDto::Wed,
            Weekday::Thu => WeekdayDto::Thu,
            Weekday::Fri => WeekdayDto::Fri,
            Weekday::Sat => WeekdayDto::Sat,
            Weekday::Sun => WeekdayDto::Sun,
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
    /// Optional authored blurb shown in the Squire's quest detail (SQUIRE-T-0094 #8). `serde(default)`
    /// for back-compat with older clients that omit it.
    #[serde(default)]
    pub description: Option<String>,
    pub reward: i64,
    /// Real-money award in whole dollars (SQUIRE-T-0099); omitted ⇒ 0 (no cash), back-compat.
    #[serde(default)]
    pub cash: i64,
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
    /// The authored blurb (so the editor can pre-fill it on edit). `serde(default)` for back-compat.
    #[serde(default)]
    pub description: Option<String>,
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
    // Raw fields so the phone can pre-fill the edit form (SQUIRE-T-0120/0126) — the labels above are
    // for display; these round-trip into `CreateQuestReq`. `serde(default)` for back-compat.
    #[serde(default)]
    pub cash: i64,
    /// `Daily | Weekly | OneOff` (an `EveryNDays` quest reports `Daily` — not phone-authorable).
    #[serde(default)]
    pub cadence: CadenceKind,
    #[serde(default)]
    pub weekdays: Option<Vec<WeekdayDto>>,
    #[serde(default)]
    pub due: Option<Date>,
    /// True = all squires; false = the explicit `squires` list.
    #[serde(default)]
    pub assign_all: bool,
    #[serde(default)]
    pub squires: Option<Vec<u64>>,
    #[serde(default)]
    pub icon: Option<String>,
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
        description: req.description.filter(|d| !d.trim().is_empty()),
        category: req.category.filter(|c| !c.is_empty()).map(domain_core::contract::Category),
        reward: req.reward.max(0) as u32,
        cash: req.cash.max(0) as u32,
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
        .map(|q| {
            let (cadence, weekdays, due) = quest_cadence_flat(&q.cadence);
            let (assign_all, squires) = quest_assignment_flat(&q.assignment);
            QuestSummaryDto {
                id: q.id,
                title: q.title.clone(),
                description: q.description.clone(),
                reward: q.reward as i64,
                category: q.category.as_ref().map(|c| c.0.clone()),
                cadence_label: cadence_label(&q.cadence),
                assignment_label: assignment_label(&snap, &q.assignment),
                completion: q.completion.into(),
                repeatable_within_day: q.repeatable_within_day,
                auto_approve: q.auto_approve,
                active: q.active,
                cash: q.cash as i64,
                cadence,
                weekdays,
                due,
                assign_all,
                squires,
                icon: q.icon.clone(),
            }
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
pub enum AchCriterionKind {
    Streak,
    TotalCompletions,
    #[default]
    PointsEarned,
}

/// Achievement scope kind on the wire (flat).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
pub enum AchScopeKind {
    #[default]
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
    // Raw fields so the phone can pre-fill the edit form (SQUIRE-T-0120/0126); the `summary` above is
    // for display. `serde(default)` for back-compat.
    #[serde(default)]
    pub criterion: AchCriterionKind,
    #[serde(default)]
    pub scope: AchScopeKind,
    #[serde(default)]
    pub scope_quest: Option<QuestId>,
    #[serde(default)]
    pub scope_category: Option<String>,
    #[serde(default)]
    pub length: Option<i64>,
    #[serde(default)]
    pub basis: Option<AchBasisKind>,
    #[serde(default)]
    pub count: Option<i64>,
    #[serde(default)]
    pub total: Option<i64>,
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
        .map(|a| {
            let f = ach_flat(&a.criterion);
            AchievementSummaryDto {
                id: a.id,
                name: a.name.clone(),
                summary: achievement_summary(&snap, &a.criterion),
                bonus: a.bonus_points as i64,
                active: a.active,
                criterion: f.criterion,
                scope: f.scope,
                scope_quest: f.scope_quest,
                scope_category: f.scope_category,
                length: f.length,
                basis: f.basis,
                count: f.count,
                total: f.total,
            }
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

// ─── Reward (item) authoring (SQUIRE-T-0074) ─────────────────────────────────────────────────────
// The redemption catalog, authored from the parent phone. `RedeemableItem`'s fields are flat already,
// but `Availability` is an enum, so the wire uses a flat `AvailabilityKind` discriminant (matching the
// quest/achievement pattern). An optional `gate` requires an achievement to be unlocked first; the
// engine's `validate_item` rejects a gate to a missing achievement (404 via `domain_status`).

/// Reward availability on the wire (flat): `Repeatable` (a recurring privilege) or `Once` (a one-time
/// treat, out of stock household-wide after the first redemption).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
pub enum AvailabilityKind {
    Once,
    #[default]
    Repeatable,
}

impl From<AvailabilityKind> for Availability {
    fn from(a: AvailabilityKind) -> Self {
        match a {
            AvailabilityKind::Once => Availability::Once,
            AvailabilityKind::Repeatable => Availability::Repeatable,
        }
    }
}

impl From<Availability> for AvailabilityKind {
    fn from(a: Availability) -> Self {
        match a {
            Availability::Once => AvailabilityKind::Once,
            Availability::Repeatable => AvailabilityKind::Repeatable,
        }
    }
}

/// `POST /admin/items` request — a flat reward definition authored from the phone.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateItemReq {
    /// Existing id to edit (upsert), or null/absent to create (the server assigns).
    pub id: Option<ItemId>,
    pub name: String,
    /// Optional flavor text shown to the child.
    pub description: Option<String>,
    /// Cost in points (clamped ≥ 0; the phone form enforces ≥ 1).
    pub cost: i64,
    pub availability: AvailabilityKind,
    /// Optional achievement that must be unlocked before this reward can be redeemed.
    pub gate: Option<AchievementId>,
    /// Optional display icon (emoji).
    pub icon: Option<String>,
}

/// The id of a created / edited reward, echoed back.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CreatedItem {
    pub id: ItemId,
}

/// A reward in the authoring list — flat, with a server-computed availability/gate summary.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ItemSummaryDto {
    pub id: ItemId,
    pub name: String,
    pub cost: i64,
    /// e.g. "Repeatable", "Once", "Once · needs: Saver".
    pub summary: String,
    pub active: bool,
    // Raw fields so the phone can pre-fill the edit form (SQUIRE-T-0120/0126); the `summary` above is
    // for display. `serde(default)` for back-compat.
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub availability: AvailabilityKind,
    #[serde(default)]
    pub gate: Option<AchievementId>,
    #[serde(default)]
    pub icon: Option<String>,
}

/// `POST /admin/items` (RequireKnight) — create or edit a reward. A gate to a missing achievement
/// is a 404.
#[utoipa::path(
    post,
    path = "/admin/items",
    tag = "knight",
    security(("bearer_auth" = [])),
    params(("X-Household" = String, Header, description = "Opaque household handle routing the request to its tenant")),
    request_body = CreateItemReq,
    responses(
        (status = 200, description = "The created/edited reward id", body = CreatedItem),
        (status = 400, description = "Invalid reward definition"),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Authenticated but not a Knight"),
        (status = 404, description = "Gate achievement not found"),
    ),
)]
pub async fn create_item(
    State(state): State<Arc<AppState>>,
    RequireKnight(principal): RequireKnight,
    Json(req): Json<CreateItemReq>,
) -> Result<Json<CreatedItem>, StatusCode> {
    let id = req.id.unwrap_or_else(|| ItemId(state.clock.now().0 as u128));
    let item = RedeemableItem {
        id,
        name: req.name,
        description: req.description.filter(|d| !d.is_empty()),
        cost: req.cost.max(0) as u32,
        gate: req.gate,
        availability: req.availability.into(),
        active: true,
        icon: req.icon.filter(|s| !s.is_empty()),
    };
    handle_command(&state, Some(principal.user), Command::DefineItem(item)).map_err(domain_status)?;
    Ok(Json(CreatedItem { id }))
}

/// `GET /admin/items` (RequireKnight) — every reward (active + archived) as flat summaries.
#[utoipa::path(
    get,
    path = "/admin/items",
    tag = "knight",
    security(("bearer_auth" = [])),
    params(("X-Household" = String, Header, description = "Opaque household handle routing the request to its tenant")),
    responses(
        (status = 200, description = "All rewards, flat", body = [ItemSummaryDto]),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Authenticated but not a Knight"),
    ),
)]
pub async fn list_items(
    State(state): State<Arc<AppState>>,
    RequireKnight(_principal): RequireKnight,
) -> Json<Vec<ItemSummaryDto>> {
    let snap = state.store.lock().expect("store mutex poisoned").snapshot();
    let rows = snap
        .items
        .iter()
        .map(|i| ItemSummaryDto {
            id: i.id,
            name: i.name.clone(),
            cost: i.cost as i64,
            summary: item_summary(&snap, i),
            active: i.active,
            description: i.description.clone(),
            availability: i.availability.into(),
            gate: i.gate,
            icon: i.icon.clone(),
        })
        .collect();
    Json(rows)
}

/// `POST /admin/items/{id}/archive` (RequireKnight) — archive (never delete). 404 if absent.
#[utoipa::path(
    post,
    path = "/admin/items/{id}/archive",
    tag = "knight",
    security(("bearer_auth" = [])),
    params(
        ("X-Household" = String, Header, description = "Opaque household handle routing the request to its tenant"),
        ("id" = i64, Path, description = "Reward id"),
    ),
    responses(
        (status = 204, description = "Archived"),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Authenticated but not a Knight"),
        (status = 404, description = "Reward not found"),
    ),
)]
pub async fn archive_item(
    State(state): State<Arc<AppState>>,
    RequireKnight(principal): RequireKnight,
    Path(id): Path<u64>,
) -> Result<StatusCode, StatusCode> {
    let iid = ItemId(u128::from(id));
    handle_command(&state, Some(principal.user), Command::ArchiveItem(iid)).map_err(domain_status)?;
    Ok(StatusCode::NO_CONTENT)
}

/// A short human summary of a reward's availability + gate (mirrors the Keep's list label).
fn item_summary(snap: &Snapshot, item: &RedeemableItem) -> String {
    let avail = match item.availability {
        Availability::Once => "Once",
        Availability::Repeatable => "Repeatable",
    };
    match item.gate {
        Some(aid) => {
            let name = snap
                .achievements
                .iter()
                .find(|a| a.id == aid)
                .map(|a| a.name.clone())
                .unwrap_or_else(|| format!("#{}", aid.0));
            format!("{avail} · needs: {name}")
        }
        None => avail.to_string(),
    }
}

// ─── Domain → flat round-trip helpers (SQUIRE-T-0120/0126) ───────────────────────────────────────
// Inverse of the `create_*` builders: turn a stored domain object's enums back into the flat wire
// fields the phone's edit form pre-fills from (the list `*_label`/`summary` strings are display-only).

/// Domain quest cadence → flat `(kind, weekdays, due)`. `EveryNDays` isn't expressible in the flat
/// phone form, so it reports `Daily` (the phone never authors `EveryNDays`).
fn quest_cadence_flat(c: &Cadence) -> (CadenceKind, Option<Vec<WeekdayDto>>, Option<Date>) {
    match c {
        Cadence::Recurring(Schedule::Daily) => (CadenceKind::Daily, None, None),
        Cadence::Recurring(Schedule::Weekly { days }) => {
            (CadenceKind::Weekly, Some(days.iter().map(|d| WeekdayDto::from(*d)).collect()), None)
        }
        Cadence::OneOff { due } => (CadenceKind::OneOff, None, *due),
        Cadence::Recurring(Schedule::EveryNDays { .. }) => (CadenceKind::Daily, None, None),
    }
}

/// Domain quest assignment → flat `(assign_all, squires)`.
fn quest_assignment_flat(a: &Assignment) -> (bool, Option<Vec<u64>>) {
    match a {
        Assignment::AllSquires => (true, None),
        Assignment::Squires(ids) => (false, Some(ids.iter().map(|u| u.0 as u64).collect())),
    }
}

/// The flat round-trip fields for an achievement criterion.
struct AchFlat {
    criterion: AchCriterionKind,
    scope: AchScopeKind,
    scope_quest: Option<QuestId>,
    scope_category: Option<String>,
    length: Option<i64>,
    basis: Option<AchBasisKind>,
    count: Option<i64>,
    total: Option<i64>,
}

/// Domain achievement criterion → its flat round-trip fields.
fn ach_flat(c: &Criterion) -> AchFlat {
    let scope_flat = |s: &Scope| -> (AchScopeKind, Option<QuestId>, Option<String>) {
        match s {
            Scope::Any => (AchScopeKind::Any, None, None),
            Scope::Quest(qid) => (AchScopeKind::Quest, Some(*qid), None),
            Scope::Category(cat) => (AchScopeKind::Category, None, Some(cat.0.clone())),
        }
    };
    match c {
        Criterion::PointsEarned { total } => AchFlat {
            criterion: AchCriterionKind::PointsEarned,
            scope: AchScopeKind::Any,
            scope_quest: None,
            scope_category: None,
            length: None,
            basis: None,
            count: None,
            total: Some(*total as i64),
        },
        Criterion::TotalCompletions { scope, count } => {
            let (sk, sq, sc) = scope_flat(scope);
            AchFlat {
                criterion: AchCriterionKind::TotalCompletions,
                scope: sk,
                scope_quest: sq,
                scope_category: sc,
                length: None,
                basis: None,
                count: Some(*count as i64),
                total: None,
            }
        }
        Criterion::Streak { scope, length, basis } => {
            let (sk, sq, sc) = scope_flat(scope);
            AchFlat {
                criterion: AchCriterionKind::Streak,
                scope: sk,
                scope_quest: sq,
                scope_category: sc,
                length: Some(*length as i64),
                basis: Some(match basis {
                    StreakBasis::ScheduledOccurrences => AchBasisKind::ScheduledOccurrences,
                    StreakBasis::CalendarDays => AchBasisKind::CalendarDays,
                }),
                count: None,
                total: None,
            }
        }
    }
}

// ─── Member administration (SQUIRE-T-0075) ───────────────────────────────────────────────────────
// The phone can already *add* a member (`POST /members`) and *mint* a pairing code (`POST /pair/codes`)
// via the control-plane (both RequireKnight). These two endpoints fill the gap: a Knight-gated member
// **list** and **de/reactivate** (mirroring the Keep's `members.rs`). Set-active applies a raw
// `Change::SetUserActive` (members are identity-owned, not engine-commanded), audited to the caller.

/// A household member in the authoring list — flat (id as an i64 wire number, role as a plain string).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MemberSummaryDto {
    pub user: UserId,
    pub display_name: String,
    pub role: Role,
    pub active: bool,
}

/// `POST /admin/members/{id}/active` body.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SetActiveReq {
    pub active: bool,
}

/// `GET /admin/members` (RequireKnight) — every household member (Knights + Squires) with role and
/// active flag.
#[utoipa::path(
    get,
    path = "/admin/members",
    tag = "knight",
    security(("bearer_auth" = [])),
    params(("X-Household" = String, Header, description = "Opaque household handle routing the request to its tenant")),
    responses(
        (status = 200, description = "All members, flat", body = [MemberSummaryDto]),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Authenticated but not a Knight"),
    ),
)]
pub async fn list_members(
    State(state): State<Arc<AppState>>,
    RequireKnight(_principal): RequireKnight,
) -> Json<Vec<MemberSummaryDto>> {
    let snap = state.store.lock().expect("store mutex poisoned").snapshot();
    let rows = snap
        .users
        .iter()
        .map(|u| MemberSummaryDto {
            user: u.id,
            display_name: u.display_name.clone(),
            role: u.role,
            active: u.active,
        })
        .collect();
    Json(rows)
}

/// `POST /admin/members/{id}/active` (RequireKnight) — de/reactivate a member via `SetUserActive`
/// (archive-not-delete), audited to the acting Knight. A missing member is a 404; a Knight cannot
/// deactivate **their own** account (no self-lockout) → 400.
#[utoipa::path(
    post,
    path = "/admin/members/{id}/active",
    tag = "knight",
    security(("bearer_auth" = [])),
    params(
        ("X-Household" = String, Header, description = "Opaque household handle routing the request to its tenant"),
        ("id" = i64, Path, description = "Member user id"),
    ),
    request_body = SetActiveReq,
    responses(
        (status = 204, description = "Updated"),
        (status = 400, description = "A Knight may not deactivate their own account"),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Authenticated but not a Knight"),
        (status = 404, description = "Member not found"),
    ),
)]
pub async fn set_member_active(
    State(state): State<Arc<AppState>>,
    RequireKnight(principal): RequireKnight,
    Path(id): Path<u64>,
    Json(req): Json<SetActiveReq>,
) -> Result<StatusCode, StatusCode> {
    let uid = UserId(u128::from(id));
    // No self-lockout: a Knight can't deactivate the account they're acting as.
    if uid == principal.user && !req.active {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut store = state.store.lock().expect("store mutex poisoned");
    if !store.snapshot().users.iter().any(|u| u.id == uid) {
        return Err(StatusCode::NOT_FOUND);
    }
    store
        .apply(Some(principal.user), &[Change::SetUserActive(uid, req.active)])
        .expect("apply: single-writer store write failed");
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /admin/members/{id}/name` body — a member's new display name (SQUIRE-T-0120/0126).
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RenameMemberReq {
    pub display_name: String,
}

/// `POST /admin/members/{id}/name` (RequireKnight) — rename a member in place. Reuses the
/// `PutUser` upsert (keeps the id, role, active flag, and `created_*` audit; moves `updated_*`),
/// audited to the acting Knight. Blank name → 400; a missing member → 404.
#[utoipa::path(
    post,
    path = "/admin/members/{id}/name",
    tag = "knight",
    security(("bearer_auth" = [])),
    params(
        ("X-Household" = String, Header, description = "Opaque household handle routing the request to its tenant"),
        ("id" = i64, Path, description = "Member user id"),
    ),
    request_body = RenameMemberReq,
    responses(
        (status = 204, description = "Renamed"),
        (status = 400, description = "Blank display name"),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Authenticated but not a Knight"),
        (status = 404, description = "Member not found"),
    ),
)]
pub async fn rename_member(
    State(state): State<Arc<AppState>>,
    RequireKnight(principal): RequireKnight,
    Path(id): Path<u64>,
    Json(req): Json<RenameMemberReq>,
) -> Result<StatusCode, StatusCode> {
    let name = req.display_name.trim();
    if name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let uid = UserId(u128::from(id));
    let mut store = state.store.lock().expect("store mutex poisoned");
    let user = store
        .snapshot()
        .users
        .iter()
        .find(|u| u.id == uid)
        .cloned()
        .ok_or(StatusCode::NOT_FOUND)?;
    let updated = User { display_name: name.to_string(), ..user };
    store
        .apply(Some(principal.user), &[Change::PutUser(updated)])
        .expect("apply: single-writer store write failed");
    Ok(StatusCode::NO_CONTENT)
}
