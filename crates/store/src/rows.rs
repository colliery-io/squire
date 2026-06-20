//! Lossless, backend-agnostic mapping between the domain contract types and their
//! row representation (SQUIRE-T-0009).
//!
//! This is the serialization seam the later snapshot/apply (T-0010) builds on. It is
//! pure data-shaping: every domain value `encode`s to a Diesel `Insertable` row and every
//! row `decode`s back to the exact domain value. Decoding is **total** — malformed data
//! yields a [`RowError`], never a panic.
//!
//! Storage conventions (see `migrations/.../up.sql`):
//!   * ids (`u128` newtypes) ⇄ decimal-string `TEXT`
//!   * enums ⇄ `TEXT` discriminant tags
//!   * `BTreeSet<…>` ⇄ comma-delimited `TEXT` (empty set ⇄ empty string, sorted/stable)
//!   * `bool` ⇄ `INTEGER` 0/1
//!   * `Points` / `Date` / `Timestamp` ⇄ `BigInt`
//!   * audit columns (`created_by/at`, `updated_by/at`) live on the row structs only —
//!     they are NOT part of the domain types. `encode` takes an [`Audit`] to stamp them;
//!     `decode` ignores them so the domain stays pure.

use std::collections::BTreeSet;

use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use domain_core::contract::Currency;
use domain_core::contract::{
    Achievement, AchievementId, Assignment, Availability, Cadence, Category, ClaimId, CommandId,
    Completion, Criterion, Date, Event, ItemId, Points, Quest, QuestId, RedeemableItem, RequestId,
    Role, Scope, Schedule, StreakBasis, Timestamp, User, UserId, Weekday,
};

use crate::schema::{achievements, events, items, quests, users};

// ─── Errors ─────────────────────────────────────────────────────────────────

/// A malformed-row error. Decoding domain values from storage is total: anything that
/// cannot be reconstructed (unknown tag, non-numeric id, a `NULL` where a variant
/// requires a value, …) surfaces here instead of panicking.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowError {
    /// A `TEXT` id column did not parse as a `u128` decimal string.
    BadId { field: &'static str, value: String },
    /// An enum/discriminant `TEXT` column held an unknown tag.
    BadTag { field: &'static str, value: String },
    /// A `BigInt`/`Integer` column held an out-of-range value (e.g. a negative `Points`).
    BadInt { field: &'static str, value: i64 },
    /// A nullable column was `NULL` (or non-null) in a way the variant forbids.
    MissingField { kind: &'static str, field: &'static str },
    /// A `Weekday` token in a delimited set was not one of Mon..=Sun.
    BadWeekday { value: String },
}

impl std::fmt::Display for RowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RowError::BadId { field, value } => write!(f, "bad id in `{field}`: {value:?}"),
            RowError::BadTag { field, value } => write!(f, "bad tag in `{field}`: {value:?}"),
            RowError::BadInt { field, value } => write!(f, "bad int in `{field}`: {value}"),
            RowError::MissingField { kind, field } => {
                write!(f, "event `{kind}` is missing required column `{field}`")
            }
            RowError::BadWeekday { value } => write!(f, "bad weekday token: {value:?}"),
        }
    }
}

impl std::error::Error for RowError {}

type Result<T> = std::result::Result<T, RowError>;

// ─── Audit ──────────────────────────────────────────────────────────────────

/// Audit stamp applied to a definition / identity row at write time (ADR SQUIRE-A-0007).
/// `by` is the acting user (`None` = system/seed); `at` is the wall-clock timestamp.
/// The caller (T-0010's `apply`) supplies created/updated pairs; the domain types
/// carry none of this.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Audit {
    pub created_by: Option<UserId>,
    pub created_at: Timestamp,
    pub updated_by: Option<UserId>,
    pub updated_at: Timestamp,
}

// ─── Primitive helpers ──────────────────────────────────────────────────────

/// `u128` → decimal-string `TEXT`.
pub fn id_to_text(id: u128) -> String {
    id.to_string()
}

/// Decimal-string `TEXT` → `u128`.
pub fn text_to_id(field: &'static str, s: &str) -> Result<u128> {
    s.parse::<u128>()
        .map_err(|_| RowError::BadId { field, value: s.to_string() })
}

fn bool_to_int(b: bool) -> i32 {
    i32::from(b)
}

fn int_to_bool(n: i32) -> bool {
    n != 0
}

/// `Points` (u32) → `BigInt`.
fn points_to_i64(p: Points) -> i64 {
    i64::from(p)
}

/// `BigInt` → `Points` (u32). Negative / overflowing values are malformed.
fn i64_to_points(field: &'static str, n: i64) -> Result<Points> {
    Points::try_from(n).map_err(|_| RowError::BadInt { field, value: n })
}

fn date_to_i64(d: Date) -> i64 {
    i64::from(d.0)
}

fn i64_to_date(field: &'static str, n: i64) -> Result<Date> {
    i32::try_from(n)
        .map(Date)
        .map_err(|_| RowError::BadInt { field, value: n })
}

// ─── Role ⇄ tag ─────────────────────────────────────────────────────────────

fn role_to_tag(r: Role) -> &'static str {
    match r {
        Role::Knight => "Knight",
        Role::Squire => "Squire",
    }
}

fn tag_to_role(s: &str) -> Result<Role> {
    match s {
        "Knight" => Ok(Role::Knight),
        "Squire" => Ok(Role::Squire),
        _ => Err(RowError::BadTag { field: "role", value: s.to_string() }),
    }
}

// ─── Weekday set ⇄ comma-delimited TEXT ─────────────────────────────────────

fn weekday_to_tag(w: Weekday) -> &'static str {
    match w {
        Weekday::Mon => "Mon",
        Weekday::Tue => "Tue",
        Weekday::Wed => "Wed",
        Weekday::Thu => "Thu",
        Weekday::Fri => "Fri",
        Weekday::Sat => "Sat",
        Weekday::Sun => "Sun",
    }
}

fn tag_to_weekday(s: &str) -> Result<Weekday> {
    Ok(match s {
        "Mon" => Weekday::Mon,
        "Tue" => Weekday::Tue,
        "Wed" => Weekday::Wed,
        "Thu" => Weekday::Thu,
        "Fri" => Weekday::Fri,
        "Sat" => Weekday::Sat,
        "Sun" => Weekday::Sun,
        _ => return Err(RowError::BadWeekday { value: s.to_string() }),
    })
}

/// `BTreeSet<Weekday>` → comma-delimited (empty set ⇄ `""`). Ordering is the set's own
/// (Mon..Sun), so output is stable.
fn weekdays_to_text(days: &BTreeSet<Weekday>) -> String {
    days.iter()
        .map(|w| weekday_to_tag(*w))
        .collect::<Vec<_>>()
        .join(",")
}

fn text_to_weekdays(s: &str) -> Result<BTreeSet<Weekday>> {
    if s.is_empty() {
        return Ok(BTreeSet::new());
    }
    s.split(',').map(tag_to_weekday).collect()
}

/// `BTreeSet<UserId>` → comma-delimited decimal ids (empty set ⇄ `""`, sorted).
fn user_ids_to_text(ids: &BTreeSet<UserId>) -> String {
    ids.iter()
        .map(|u| id_to_text(u.0))
        .collect::<Vec<_>>()
        .join(",")
}

fn text_to_user_ids(field: &'static str, s: &str) -> Result<BTreeSet<UserId>> {
    if s.is_empty() {
        return Ok(BTreeSet::new());
    }
    s.split(',')
        .map(|tok| Ok(UserId(text_to_id(field, tok)?)))
        .collect()
}

// ─── users ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Insertable, Queryable, Selectable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct UserRow {
    pub id: String,
    pub role: String,
    pub display_name: String,
    pub active: i32,
    pub created_by: Option<String>,
    pub created_at: i64,
    pub updated_by: Option<String>,
    pub updated_at: i64,
}

impl UserRow {
    pub fn from_user(u: &User, audit: Audit) -> Self {
        UserRow {
            id: id_to_text(u.id.0),
            role: role_to_tag(u.role).to_string(),
            display_name: u.display_name.clone(),
            active: bool_to_int(u.active),
            created_by: audit.created_by.map(|c| id_to_text(c.0)),
            created_at: audit.created_at.0,
            updated_by: audit.updated_by.map(|c| id_to_text(c.0)),
            updated_at: audit.updated_at.0,
        }
    }

    pub fn to_user(&self) -> Result<User> {
        Ok(User {
            id: UserId(text_to_id("users.id", &self.id)?),
            role: tag_to_role(&self.role)?,
            display_name: self.display_name.clone(),
            active: int_to_bool(self.active),
        })
    }
}

// ─── quests ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Insertable, Queryable, Selectable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = quests)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct QuestRow {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub reward: i64,
    pub cadence_kind: String,
    pub cadence_due: Option<i64>,
    pub cadence_weekdays: Option<String>,
    pub cadence_n: Option<i64>,
    pub cadence_anchor: Option<i64>,
    pub assignment_kind: String,
    pub assignment_squires: Option<String>,
    pub completion: String,
    pub auto_approve: i32,
    pub repeatable_within_day: i32,
    pub icon: Option<String>,
    pub active: i32,
    pub created_by: Option<String>,
    pub created_at: i64,
    pub updated_by: Option<String>,
    pub updated_at: i64,
}

fn completion_to_tag(c: Completion) -> &'static str {
    match c {
        Completion::EachAssignee => "EachAssignee",
        Completion::Race => "Race",
    }
}

fn tag_to_completion(s: &str) -> Result<Completion> {
    match s {
        "EachAssignee" => Ok(Completion::EachAssignee),
        "Race" => Ok(Completion::Race),
        _ => Err(RowError::BadTag { field: "completion", value: s.to_string() }),
    }
}

impl QuestRow {
    pub fn from_quest(q: &Quest, audit: Audit) -> Self {
        // Cadence → (kind, due, weekdays, n, anchor)
        let (cadence_kind, cadence_due, cadence_weekdays, cadence_n, cadence_anchor) = match &q
            .cadence
        {
            Cadence::OneOff { due } => (
                "OneOff",
                due.map(date_to_i64),
                None,
                None,
                None,
            ),
            Cadence::Recurring(Schedule::Daily) => ("Daily", None, None, None, None),
            Cadence::Recurring(Schedule::Weekly { days }) => {
                ("Weekly", None, Some(weekdays_to_text(days)), None, None)
            }
            Cadence::Recurring(Schedule::EveryNDays { n, anchor }) => (
                "EveryNDays",
                None,
                None,
                Some(i64::from(*n)),
                Some(date_to_i64(*anchor)),
            ),
        };

        let (assignment_kind, assignment_squires) = match &q.assignment {
            Assignment::AllSquires => ("AllSquires", None),
            Assignment::Squires(ids) => ("Squires", Some(user_ids_to_text(ids))),
        };

        QuestRow {
            id: id_to_text(q.id.0),
            title: q.title.clone(),
            description: q.description.clone(),
            category: q.category.as_ref().map(|c| c.0.clone()),
            reward: points_to_i64(q.reward),
            cadence_kind: cadence_kind.to_string(),
            cadence_due,
            cadence_weekdays,
            cadence_n,
            cadence_anchor,
            assignment_kind: assignment_kind.to_string(),
            assignment_squires,
            completion: completion_to_tag(q.completion).to_string(),
            auto_approve: bool_to_int(q.auto_approve),
            repeatable_within_day: bool_to_int(q.repeatable_within_day),
            icon: q.icon.clone(),
            active: bool_to_int(q.active),
            created_by: audit.created_by.map(|c| id_to_text(c.0)),
            created_at: audit.created_at.0,
            updated_by: audit.updated_by.map(|c| id_to_text(c.0)),
            updated_at: audit.updated_at.0,
        }
    }

    pub fn to_quest(&self) -> Result<Quest> {
        let cadence = match self.cadence_kind.as_str() {
            "OneOff" => {
                let due = match self.cadence_due {
                    Some(n) => Some(i64_to_date("quests.cadence_due", n)?),
                    None => None,
                };
                Cadence::OneOff { due }
            }
            "Daily" => Cadence::Recurring(Schedule::Daily),
            "Weekly" => {
                let raw = self.cadence_weekdays.as_deref().ok_or(RowError::MissingField {
                    kind: "quest",
                    field: "cadence_weekdays",
                })?;
                Cadence::Recurring(Schedule::Weekly { days: text_to_weekdays(raw)? })
            }
            "EveryNDays" => {
                let n_raw = self.cadence_n.ok_or(RowError::MissingField {
                    kind: "quest",
                    field: "cadence_n",
                })?;
                let n = u16::try_from(n_raw)
                    .map_err(|_| RowError::BadInt { field: "quests.cadence_n", value: n_raw })?;
                let anchor_raw = self.cadence_anchor.ok_or(RowError::MissingField {
                    kind: "quest",
                    field: "cadence_anchor",
                })?;
                let anchor = i64_to_date("quests.cadence_anchor", anchor_raw)?;
                Cadence::Recurring(Schedule::EveryNDays { n, anchor })
            }
            other => {
                return Err(RowError::BadTag {
                    field: "cadence_kind",
                    value: other.to_string(),
                })
            }
        };

        let assignment = match self.assignment_kind.as_str() {
            "AllSquires" => Assignment::AllSquires,
            "Squires" => {
                let raw = self.assignment_squires.as_deref().ok_or(RowError::MissingField {
                    kind: "quest",
                    field: "assignment_squires",
                })?;
                Assignment::Squires(text_to_user_ids("quests.assignment_squires", raw)?)
            }
            other => {
                return Err(RowError::BadTag {
                    field: "assignment_kind",
                    value: other.to_string(),
                })
            }
        };

        Ok(Quest {
            id: QuestId(text_to_id("quests.id", &self.id)?),
            title: self.title.clone(),
            description: self.description.clone(),
            category: self.category.clone().map(Category),
            reward: i64_to_points("quests.reward", self.reward)?,
            cadence,
            assignment,
            completion: tag_to_completion(&self.completion)?,
            auto_approve: int_to_bool(self.auto_approve),
            repeatable_within_day: int_to_bool(self.repeatable_within_day),
            active: int_to_bool(self.active),
            icon: self.icon.clone(),
        })
    }
}

// ─── items ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Insertable, Queryable, Selectable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = items)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ItemRow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub cost: i64,
    pub gate: Option<String>,
    pub availability: String,
    pub icon: Option<String>,
    pub active: i32,
    pub created_by: Option<String>,
    pub created_at: i64,
    pub updated_by: Option<String>,
    pub updated_at: i64,
}

fn availability_to_tag(a: Availability) -> &'static str {
    match a {
        Availability::Once => "Once",
        Availability::Repeatable => "Repeatable",
    }
}

fn tag_to_availability(s: &str) -> Result<Availability> {
    match s {
        "Once" => Ok(Availability::Once),
        "Repeatable" => Ok(Availability::Repeatable),
        _ => Err(RowError::BadTag { field: "availability", value: s.to_string() }),
    }
}

impl ItemRow {
    pub fn from_item(it: &RedeemableItem, audit: Audit) -> Self {
        ItemRow {
            id: id_to_text(it.id.0),
            name: it.name.clone(),
            description: it.description.clone(),
            cost: points_to_i64(it.cost),
            gate: it.gate.map(|g| id_to_text(g.0)),
            availability: availability_to_tag(it.availability).to_string(),
            icon: it.icon.clone(),
            active: bool_to_int(it.active),
            created_by: audit.created_by.map(|c| id_to_text(c.0)),
            created_at: audit.created_at.0,
            updated_by: audit.updated_by.map(|c| id_to_text(c.0)),
            updated_at: audit.updated_at.0,
        }
    }

    pub fn to_item(&self) -> Result<RedeemableItem> {
        let gate = match &self.gate {
            Some(g) => Some(AchievementId(text_to_id("items.gate", g)?)),
            None => None,
        };
        Ok(RedeemableItem {
            id: ItemId(text_to_id("items.id", &self.id)?),
            name: self.name.clone(),
            description: self.description.clone(),
            cost: i64_to_points("items.cost", self.cost)?,
            gate,
            availability: tag_to_availability(&self.availability)?,
            active: int_to_bool(self.active),
            icon: self.icon.clone(),
        })
    }
}

// ─── achievements ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Insertable, Queryable, Selectable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = achievements)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct AchievementRow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub criterion_kind: String,
    pub scope_kind: Option<String>,
    pub scope_quest: Option<String>,
    pub scope_category: Option<String>,
    pub streak_length: Option<i64>,
    pub streak_basis: Option<String>,
    pub total_count: Option<i64>,
    pub points_total: Option<i64>,
    pub bonus_points: i64,
    pub active: i32,
    pub created_by: Option<String>,
    pub created_at: i64,
    pub updated_by: Option<String>,
    pub updated_at: i64,
}

fn streak_basis_to_tag(b: StreakBasis) -> &'static str {
    match b {
        StreakBasis::ScheduledOccurrences => "ScheduledOccurrences",
        StreakBasis::CalendarDays => "CalendarDays",
    }
}

fn tag_to_streak_basis(s: &str) -> Result<StreakBasis> {
    match s {
        "ScheduledOccurrences" => Ok(StreakBasis::ScheduledOccurrences),
        "CalendarDays" => Ok(StreakBasis::CalendarDays),
        _ => Err(RowError::BadTag { field: "streak_basis", value: s.to_string() }),
    }
}

/// `Scope` → `(scope_kind, scope_quest, scope_category)`.
fn scope_to_cols(scope: &Scope) -> (&'static str, Option<String>, Option<String>) {
    match scope {
        Scope::Quest(q) => ("Quest", Some(id_to_text(q.0)), None),
        Scope::Category(c) => ("Category", None, Some(c.0.clone())),
        Scope::Any => ("Any", None, None),
    }
}

fn cols_to_scope(
    kind: &str,
    quest: &Option<String>,
    category: &Option<String>,
) -> Result<Scope> {
    match kind {
        "Quest" => {
            let q = quest.as_deref().ok_or(RowError::MissingField {
                kind: "achievement",
                field: "scope_quest",
            })?;
            Ok(Scope::Quest(QuestId(text_to_id("achievements.scope_quest", q)?)))
        }
        "Category" => {
            let c = category.as_deref().ok_or(RowError::MissingField {
                kind: "achievement",
                field: "scope_category",
            })?;
            Ok(Scope::Category(Category(c.to_string())))
        }
        "Any" => Ok(Scope::Any),
        other => Err(RowError::BadTag { field: "scope_kind", value: other.to_string() }),
    }
}

impl AchievementRow {
    pub fn from_achievement(a: &Achievement, audit: Audit) -> Self {
        let mut row = AchievementRow {
            id: id_to_text(a.id.0),
            name: a.name.clone(),
            description: a.description.clone(),
            criterion_kind: String::new(),
            scope_kind: None,
            scope_quest: None,
            scope_category: None,
            streak_length: None,
            streak_basis: None,
            total_count: None,
            points_total: None,
            bonus_points: points_to_i64(a.bonus_points),
            active: bool_to_int(a.active),
            created_by: audit.created_by.map(|c| id_to_text(c.0)),
            created_at: audit.created_at.0,
            updated_by: audit.updated_by.map(|c| id_to_text(c.0)),
            updated_at: audit.updated_at.0,
        };

        match &a.criterion {
            Criterion::Streak { scope, length, basis } => {
                row.criterion_kind = "Streak".to_string();
                let (sk, sq, sc) = scope_to_cols(scope);
                row.scope_kind = Some(sk.to_string());
                row.scope_quest = sq;
                row.scope_category = sc;
                row.streak_length = Some(i64::from(*length));
                row.streak_basis = Some(streak_basis_to_tag(*basis).to_string());
            }
            Criterion::TotalCompletions { scope, count } => {
                row.criterion_kind = "TotalCompletions".to_string();
                let (sk, sq, sc) = scope_to_cols(scope);
                row.scope_kind = Some(sk.to_string());
                row.scope_quest = sq;
                row.scope_category = sc;
                row.total_count = Some(i64::from(*count));
            }
            Criterion::PointsEarned { total } => {
                row.criterion_kind = "PointsEarned".to_string();
                row.points_total = Some(points_to_i64(*total));
            }
        }

        row
    }

    pub fn to_achievement(&self) -> Result<Achievement> {
        let criterion = match self.criterion_kind.as_str() {
            "Streak" => {
                let scope_kind = self.scope_kind.as_deref().ok_or(RowError::MissingField {
                    kind: "achievement",
                    field: "scope_kind",
                })?;
                let scope = cols_to_scope(scope_kind, &self.scope_quest, &self.scope_category)?;
                let length_raw = self.streak_length.ok_or(RowError::MissingField {
                    kind: "achievement",
                    field: "streak_length",
                })?;
                let length = u32::try_from(length_raw).map_err(|_| RowError::BadInt {
                    field: "achievements.streak_length",
                    value: length_raw,
                })?;
                let basis_raw = self.streak_basis.as_deref().ok_or(RowError::MissingField {
                    kind: "achievement",
                    field: "streak_basis",
                })?;
                Criterion::Streak { scope, length, basis: tag_to_streak_basis(basis_raw)? }
            }
            "TotalCompletions" => {
                let scope_kind = self.scope_kind.as_deref().ok_or(RowError::MissingField {
                    kind: "achievement",
                    field: "scope_kind",
                })?;
                let scope = cols_to_scope(scope_kind, &self.scope_quest, &self.scope_category)?;
                let count_raw = self.total_count.ok_or(RowError::MissingField {
                    kind: "achievement",
                    field: "total_count",
                })?;
                let count = u32::try_from(count_raw).map_err(|_| RowError::BadInt {
                    field: "achievements.total_count",
                    value: count_raw,
                })?;
                Criterion::TotalCompletions { scope, count }
            }
            "PointsEarned" => {
                let total_raw = self.points_total.ok_or(RowError::MissingField {
                    kind: "achievement",
                    field: "points_total",
                })?;
                Criterion::PointsEarned {
                    total: i64_to_points("achievements.points_total", total_raw)?,
                }
            }
            other => {
                return Err(RowError::BadTag {
                    field: "criterion_kind",
                    value: other.to_string(),
                })
            }
        };

        Ok(Achievement {
            id: AchievementId(text_to_id("achievements.id", &self.id)?),
            name: self.name.clone(),
            description: self.description.clone(),
            criterion,
            bonus_points: i64_to_points("achievements.bonus_points", self.bonus_points)?,
            active: int_to_bool(self.active),
        })
    }
}

// ─── events ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Insertable, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = events)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct EventRow {
    pub seq: i64,
    pub kind: String,
    pub squire: String,
    pub actor: Option<String>,
    pub at: i64,
    pub claim_id: Option<String>,
    pub quest_id: Option<String>,
    pub on_date: Option<i64>,
    pub points: Option<i64>,
    pub amount: Option<i64>,
    pub reason: Option<String>,
    pub request_id: Option<String>,
    pub command_id: Option<String>,
    pub item_id: Option<String>,
    pub achievement_id: Option<String>,
    pub currency: Option<String>, // SQUIRE-A-0013: the Adjusted event's currency tag (NULL ⇒ Coins)
}

/// All columns default to `None`/empty; each variant fills only its own.
fn empty_event_row(seq: i64, kind: &str, squire: UserId, at: Timestamp) -> EventRow {
    EventRow {
        seq,
        kind: kind.to_string(),
        squire: id_to_text(squire.0),
        actor: None,
        at: at.0,
        claim_id: None,
        quest_id: None,
        on_date: None,
        points: None,
        amount: None,
        reason: None,
        request_id: None,
        command_id: None,
        item_id: None,
        achievement_id: None,
        currency: None,
    }
}

fn opt_user_to_text(u: Option<UserId>) -> Option<String> {
    u.map(|u| id_to_text(u.0))
}

impl EventRow {
    /// Encode an [`Event`] to its row. `seq` is the app-assigned order key (supplied by
    /// the caller — `apply` sets it to `max(seq)+1`).
    pub fn from_event(seq: i64, ev: &Event) -> Self {
        match ev {
            Event::CompletionClaimed { claim_id, squire, quest_id, on, at } => {
                let mut r = empty_event_row(seq, "CompletionClaimed", *squire, *at);
                r.claim_id = Some(id_to_text(claim_id.0));
                r.quest_id = Some(id_to_text(quest_id.0));
                r.on_date = Some(date_to_i64(*on));
                r
            }
            Event::CompletionApproved { claim_id, squire, actor, points, at } => {
                let mut r = empty_event_row(seq, "CompletionApproved", *squire, *at);
                r.claim_id = Some(id_to_text(claim_id.0));
                r.actor = opt_user_to_text(*actor);
                r.points = Some(points_to_i64(*points));
                r
            }
            Event::CompletionRejected { claim_id, squire, actor, reason, at } => {
                let mut r = empty_event_row(seq, "CompletionRejected", *squire, *at);
                r.claim_id = Some(id_to_text(claim_id.0));
                r.actor = opt_user_to_text(*actor);
                r.reason = reason.clone();
                r
            }
            Event::ItemRedeemed { request_id, command_id, squire, actor, item_id, cost, at } => {
                let mut r = empty_event_row(seq, "ItemRedeemed", *squire, *at);
                r.request_id = request_id.map(|x| id_to_text(x.0));
                r.command_id = command_id.map(|x| id_to_text(x.0));
                r.actor = opt_user_to_text(*actor);
                r.item_id = Some(id_to_text(item_id.0));
                r.points = Some(points_to_i64(*cost));
                r
            }
            Event::AchievementUnlocked { squire, id, bonus, at } => {
                let mut r = empty_event_row(seq, "AchievementUnlocked", *squire, *at);
                r.achievement_id = Some(id_to_text(id.0));
                r.points = Some(points_to_i64(*bonus));
                r
            }
            Event::Adjusted { command_id, squire, actor, currency, amount, reason, at } => {
                let mut r = empty_event_row(seq, "Adjusted", *squire, *at);
                r.command_id = Some(id_to_text(command_id.0));
                r.actor = opt_user_to_text(*actor);
                r.currency = Some(currency.tag().to_string());
                r.amount = Some(*amount);
                r.reason = Some(reason.clone());
                r
            }
            Event::RedemptionRequested { request_id, squire, item_id, at } => {
                let mut r = empty_event_row(seq, "RedemptionRequested", *squire, *at);
                r.request_id = Some(id_to_text(request_id.0));
                r.item_id = Some(id_to_text(item_id.0));
                r
            }
            Event::RedemptionRejected { request_id, squire, actor, reason, at } => {
                let mut r = empty_event_row(seq, "RedemptionRejected", *squire, *at);
                r.request_id = Some(id_to_text(request_id.0));
                r.actor = opt_user_to_text(*actor);
                r.reason = reason.clone();
                r
            }
        }
    }

    /// Decode this row back into an [`Event`]. The `seq` is storage-only ordering and is
    /// not carried by the domain type, so it is dropped here.
    pub fn to_event(&self) -> Result<Event> {
        let squire = UserId(text_to_id("events.squire", &self.squire)?);
        let at = Timestamp(self.at);
        let actor = self.opt_actor()?;

        Ok(match self.kind.as_str() {
            "CompletionClaimed" => Event::CompletionClaimed {
                claim_id: ClaimId(self.req_id("claim_id", &self.claim_id)?),
                squire,
                quest_id: QuestId(self.req_id("quest_id", &self.quest_id)?),
                on: i64_to_date("events.on_date", self.req_int("on_date", self.on_date)?)?,
                at,
            },
            "CompletionApproved" => Event::CompletionApproved {
                claim_id: ClaimId(self.req_id("claim_id", &self.claim_id)?),
                squire,
                actor,
                points: i64_to_points("events.points", self.req_int("points", self.points)?)?,
                at,
            },
            "CompletionRejected" => Event::CompletionRejected {
                claim_id: ClaimId(self.req_id("claim_id", &self.claim_id)?),
                squire,
                actor,
                reason: self.reason.clone(),
                at,
            },
            "ItemRedeemed" => Event::ItemRedeemed {
                request_id: self.opt_id("request_id", &self.request_id)?.map(RequestId),
                command_id: self.opt_id("command_id", &self.command_id)?.map(CommandId),
                squire,
                actor,
                item_id: ItemId(self.req_id("item_id", &self.item_id)?),
                cost: i64_to_points("events.points", self.req_int("points", self.points)?)?,
                at,
            },
            "AchievementUnlocked" => Event::AchievementUnlocked {
                squire,
                id: AchievementId(self.req_id("achievement_id", &self.achievement_id)?),
                bonus: i64_to_points("events.points", self.req_int("points", self.points)?)?,
                at,
            },
            "Adjusted" => Event::Adjusted {
                command_id: CommandId(self.req_id("command_id", &self.command_id)?),
                squire,
                actor,
                currency: self
                    .currency
                    .as_deref()
                    .and_then(Currency::from_tag)
                    .unwrap_or(Currency::Coins), // back-compat: migrated `PointsAdjusted`/NULL ⇒ Coins
                amount: self.req_int("amount", self.amount)?,
                reason: self.reason.clone().ok_or(RowError::MissingField {
                    kind: "Adjusted",
                    field: "reason",
                })?,
                at,
            },
            "RedemptionRequested" => Event::RedemptionRequested {
                request_id: RequestId(self.req_id("request_id", &self.request_id)?),
                squire,
                item_id: ItemId(self.req_id("item_id", &self.item_id)?),
                at,
            },
            "RedemptionRejected" => Event::RedemptionRejected {
                request_id: RequestId(self.req_id("request_id", &self.request_id)?),
                squire,
                actor,
                reason: self.reason.clone(),
                at,
            },
            other => {
                return Err(RowError::BadTag { field: "events.kind", value: other.to_string() })
            }
        })
    }

    fn opt_actor(&self) -> Result<Option<UserId>> {
        Ok(self.opt_id("actor", &self.actor)?.map(UserId))
    }

    fn opt_id(&self, field: &'static str, col: &Option<String>) -> Result<Option<u128>> {
        match col {
            Some(s) => Ok(Some(text_to_id(self.qualified(field), s)?)),
            None => Ok(None),
        }
    }

    fn req_id(&self, field: &'static str, col: &Option<String>) -> Result<u128> {
        let s = col.as_deref().ok_or(RowError::MissingField {
            kind: leak_kind(&self.kind),
            field,
        })?;
        text_to_id(self.qualified(field), s)
    }

    fn req_int(&self, field: &'static str, col: Option<i64>) -> Result<i64> {
        col.ok_or(RowError::MissingField { kind: leak_kind(&self.kind), field })
    }

    /// Produce a `'static`-ish qualified field name for error reporting. The set of
    /// columns is fixed, so map to literals.
    fn qualified(&self, field: &'static str) -> &'static str {
        match field {
            "actor" => "events.actor",
            "claim_id" => "events.claim_id",
            "quest_id" => "events.quest_id",
            "request_id" => "events.request_id",
            "command_id" => "events.command_id",
            "item_id" => "events.item_id",
            "achievement_id" => "events.achievement_id",
            other => other,
        }
    }
}

/// Map a runtime event-kind string to a `'static` tag for [`RowError::MissingField`].
/// Falls back to a generic literal for an unknown kind (which `to_event` reports
/// separately as `BadTag`).
fn leak_kind(kind: &str) -> &'static str {
    match kind {
        "CompletionClaimed" => "CompletionClaimed",
        "CompletionApproved" => "CompletionApproved",
        "CompletionRejected" => "CompletionRejected",
        "ItemRedeemed" => "ItemRedeemed",
        "AchievementUnlocked" => "AchievementUnlocked",
        "Adjusted" => "Adjusted",
        "RedemptionRequested" => "RedemptionRequested",
        "RedemptionRejected" => "RedemptionRejected",
        _ => "event",
    }
}
