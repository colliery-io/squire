//! Squire `store` — Diesel-backed persistence for the domain core.
//!
//! Task SQUIRE-T-0010: the real [`Repository`] (`snapshot` + atomic `apply`) and a real
//! [`Clock`], implemented over [`AnyConnection`] so the SAME code runs on SQLite and
//! Postgres. All row⇄domain conversion goes through the T-0009 mapping in [`rows`].
//!
//! Default build is SQLite-only (bundled SQLite, no system libpq). Postgres is an
//! opt-in `postgres` cargo feature.

pub mod backup;
pub mod conn;
pub mod migrations;
pub mod rows;
pub mod schema;
pub mod settings;
pub mod tenant;

pub use conn::AnyConnection;
pub use migrations::{run_migrations, MIGRATIONS};
pub use rows::{AchievementRow, Audit, EventRow, ItemRow, QuestRow, RowError, UserRow};
pub use settings::{date_in_zone, live_config, valid_timezone, ConfigView, LiveConfig, LocalClock};

/// Postgres-backend helpers, compiled only under the `postgres` feature (which links libpq).
/// Used by the dual-backend integration tests run against the docker-compose Postgres service
/// (ADR SQUIRE-A-0003 / NFR-2.6). All diesel-Pg usage is kept inside this module so the
/// integration tests need no diesel dev-dependency.
#[cfg(feature = "postgres")]
pub mod pg {
    use diesel::connection::SimpleConnection;
    use diesel::pg::PgConnection;
    use diesel::prelude::*;

    /// Connect to `url`, reset the `public` schema to a clean slate (so the test is idempotent
    /// across runs), apply the embedded migrations, and verify every table exists — i.e. the
    /// portable DDL migrates on Postgres exactly as on SQLite.
    pub fn provision_clean(url: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = PgConnection::establish(url)?;
        conn.batch_execute("DROP SCHEMA IF EXISTS public CASCADE; CREATE SCHEMA public;")?;
        crate::run_migrations(&mut conn)?;
        conn.batch_execute(
            "SELECT 1 FROM users LIMIT 0; SELECT 1 FROM quests LIMIT 0; \
             SELECT 1 FROM items LIMIT 0; SELECT 1 FROM achievements LIMIT 0; \
             SELECT 1 FROM events LIMIT 0; SELECT 1 FROM credentials LIMIT 0;",
        )?;
        Ok(())
    }
}

use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use diesel::sqlite::SqliteConnection;
use diesel::Connection;
use diesel_migrations::MigrationHarness;

use domain_core::contract::{
    config_keys, Achievement, AchievementId, Change, Clock, Date, Event, HouseholdConfig, ItemId,
    Quest, QuestId, RedeemableItem, RepoError, Repository, Role, Snapshot, Timestamp, User, UserId,
};

use crate::rows::id_to_text;
use crate::schema::{
    achievements, config, credentials, events, items, pairing_codes, quests, users,
};

// ─── error mapping ───────────────────────────────────────────────────────────

/// Map a Diesel error to the domain [`RepoError`].
///
/// A uniqueness / serialization / check-constraint conflict is a [`RepoError::Conflict`]
/// (the single-writer lost a race or violated an invariant); everything else
/// (connection / IO / backend) becomes [`RepoError::Io`].
fn map_err(e: DieselError) -> RepoError {
    match e {
        DieselError::DatabaseError(
            DatabaseErrorKind::UniqueViolation
            | DatabaseErrorKind::SerializationFailure
            | DatabaseErrorKind::CheckViolation
            | DatabaseErrorKind::ForeignKeyViolation,
            _,
        ) => RepoError::Conflict,
        other => RepoError::Io(format!("{other}")),
    }
}

/// A [`RowError`] surfacing during decode is treated as corrupt storage / IO.
fn map_row_err(e: RowError) -> RepoError {
    RepoError::Io(format!("{e}"))
}

/// Transaction-internal error: carries a [`RepoError`] while satisfying Diesel's
/// `transaction` requirement that the error implement `From<diesel::result::Error>`
/// (`RepoError` is foreign, so we cannot impl that directly on it).
struct TxnError(RepoError);

impl From<DieselError> for TxnError {
    fn from(e: DieselError) -> Self {
        TxnError(map_err(e))
    }
}

impl From<RepoError> for TxnError {
    fn from(e: RepoError) -> Self {
        TxnError(e)
    }
}

// ─── upsert dispatch macro ─────────────────────────────────────────────────────
//
// Diesel's `MultiConnection` backend reports `OnConflictClause = DoesNotSupportOnConflictClause`
// — i.e. the *erased* `AnyConnection` cannot type-check `on_conflict().do_update()`. So each
// upsert dispatches on the enum and runs the typed query against the CONCRETE connection
// (`SqliteConnection` / `PgConnection`), both of which support the Pg-style upsert in Diesel 2.
// The `$set` tuple is identical per backend; the macro avoids hand-duplicating it.
//
// For the definition/identity upserts, `created_*` are NOT in `$set`, so an existing
// (conflicting) row keeps its original `created_by`/`created_at`; only a brand-new INSERT picks
// them up from the row values. `updated_*` are always in `$set`, so every write stamps the last
// editor + time. (The `credentials` upsert has no audit columns at all.)
//
// Defined here (ahead of the first use in `Store::set_credential`) so it is in textual scope for
// the whole module.
macro_rules! run_upsert {
    ($conn:expr, $table:path, $id:path, $row:expr, $set:expr) => {{
        let row = $row;
        match $conn {
            AnyConnection::Sqlite(c) => diesel::insert_into($table)
                .values(&row)
                .on_conflict($id)
                .do_update()
                .set($set(&row))
                .execute(c),
            #[cfg(feature = "postgres")]
            AnyConnection::Pg(c) => diesel::insert_into($table)
                .values(&row)
                .on_conflict($id)
                .do_update()
                .set($set(&row))
                .execute(c),
        }
        .map_err(map_err)?;
    }};
}

// ─── the generic store ───────────────────────────────────────────────────────

/// Backend-portable [`Repository`] over an [`AnyConnection`] plus an injected [`Clock`].
///
/// The same `snapshot` / `apply` bodies run on SQLite and Postgres (the connection is the
/// `MultiConnection` enum). The clock is injected so `apply`'s audit stamping is
/// deterministic and testable (see [`FixedClock`]).
pub struct Store<C: Clock> {
    // `RefCell` because `Repository::snapshot(&self)` is read-only at the trait level, but
    // Diesel queries need `&mut Connection`. The store is the single writer, so there is no
    // concurrent borrow to fear; the `RefCell` makes the interior mutability sound.
    conn: std::cell::RefCell<AnyConnection>,
    clock: C,
}

impl<C: Clock> Store<C> {
    /// Wrap an already-migrated connection with a clock.
    pub fn new(conn: AnyConnection, clock: C) -> Self {
        Self {
            conn: std::cell::RefCell::new(conn),
            clock,
        }
    }

    /// Borrow the clock.
    pub fn clock(&self) -> &C {
        &self.clock
    }

    /// Mutably borrow the clock (tests advance a [`FixedClock`] between batches).
    pub fn clock_mut(&mut self) -> &mut C {
        &mut self.clock
    }

    /// Mutably borrow the underlying connection (advanced / test use — e.g. reading the
    /// audit columns that are intentionally absent from the [`Snapshot`]).
    pub fn connection(&mut self) -> std::cell::RefMut<'_, AnyConnection> {
        self.conn.borrow_mut()
    }

    /// Export this household store to ONE file at `path` (the full backend-portable dump:
    /// users + definitions + complete event log, incl. audit columns and `seq`). See
    /// [`crate::backup::export`].
    pub fn export(&self, path: impl AsRef<std::path::Path>) -> Result<(), backup::BackupError> {
        backup::export(&mut self.conn.borrow_mut(), path)
    }

    /// Raw per-item event log (NFR-11): every event touching `item`, in `seq` order, decoded
    /// to domain [`Event`]s. These are exactly the redemption lifecycle events that carry the
    /// item — `RedemptionRequested` / `RedemptionRejected` / `ItemRedeemed` — i.e. the trail
    /// that explains how an item's redemptions (and their balance impact) came to be.
    ///
    /// Filtered on the `item_id` column at the database, so it scales to a long log without
    /// materialising the whole snapshot.
    pub fn raw_log_for_item(&self, item: ItemId) -> Vec<Event> {
        let mut conn = self.conn.borrow_mut();
        let rows: Vec<EventRow> = events::table
            .filter(events::item_id.eq(id_to_text(item.0)))
            .select(EventRow::as_select())
            .order(events::seq.asc())
            .load(&mut *conn)
            .expect("raw_log_for_item: query failed (corrupt store)");
        rows.iter()
            .map(|r| r.to_event())
            .collect::<Result<Vec<_>, _>>()
            .expect("raw_log_for_item: decode failed (corrupt store)")
    }

    /// Raw per-quest event log (NFR-11): every event touching `quest`, in `seq` order, decoded
    /// to domain [`Event`]s — the trail that explains how a Squire's streak / completion history
    /// for that quest was reached.
    ///
    /// Two steps: (1) the `CompletionClaimed` events carry `quest_id` directly; (2) the
    /// approve/reject events (`CompletionApproved` / `CompletionRejected`) carry only a
    /// `claim_id`, so we first resolve the set of claim ids for this quest, then pull the
    /// events whose `claim_id` is in that set. The union is returned in `seq` order.
    pub fn raw_log_for_quest(&self, quest: QuestId) -> Vec<Event> {
        let mut conn = self.conn.borrow_mut();

        // (1) claim ids for this quest (from its CompletionClaimed events).
        let claim_ids: Vec<String> = events::table
            .filter(events::quest_id.eq(id_to_text(quest.0)))
            .filter(events::claim_id.is_not_null())
            .select(events::claim_id)
            .load::<Option<String>>(&mut *conn)
            .expect("raw_log_for_quest: claim-id query failed (corrupt store)")
            .into_iter()
            .flatten()
            .collect();

        // (2) all events that either name the quest directly OR resolve to one of its claims,
        //     ordered by seq. A single query with the two predicates OR'd keeps it one pass.
        let rows: Vec<EventRow> = events::table
            .filter(
                events::quest_id
                    .eq(id_to_text(quest.0))
                    .or(events::claim_id.eq_any(claim_ids)),
            )
            .select(EventRow::as_select())
            .order(events::seq.asc())
            .load(&mut *conn)
            .expect("raw_log_for_quest: query failed (corrupt store)");
        rows.iter()
            .map(|r| r.to_event())
            .collect::<Result<Vec<_>, _>>()
            .expect("raw_log_for_quest: decode failed (corrupt store)")
    }

    /// Recent **household-wide** event trail (SQUIRE-T-0112): the most recent `limit` events across
    /// the whole household, **newest first**, decoded to domain [`Event`]s — the audit/history feed
    /// for the Knight phone (parity with the Keep's per-entity inspector, but household-wide).
    /// Ordered + limited at the database so a long log never materialises the whole snapshot.
    pub fn recent_events(&self, limit: i64) -> Vec<Event> {
        let mut conn = self.conn.borrow_mut();
        let rows: Vec<EventRow> = events::table
            .select(EventRow::as_select())
            .order(events::seq.desc())
            .limit(limit)
            .load(&mut *conn)
            .expect("recent_events: query failed (corrupt store)");
        rows.iter()
            .map(|r| r.to_event())
            .collect::<Result<Vec<_>, _>>()
            .expect("recent_events: decode failed (corrupt store)")
    }

    /// Upsert a member's hashed secret into THIS tenant's `credentials` table (REQ-1.6).
    ///
    /// Credentials live inside the tenant schema, isolated exactly like the rest of the
    /// household's data. This is NOT a domain [`Change`] — it bypasses `apply` (no audit
    /// stamping, no event) and writes the row directly. `secret_hash` is an Argon2id PHC
    /// string; the plaintext is never stored. Upsert by `user_id` so re-setting a secret
    /// replaces the existing hash.
    pub fn set_credential(&self, user: UserId, secret_hash: &str) -> Result<(), RepoError> {
        let mut conn = self.conn.borrow_mut();
        let row = CredentialRow {
            user_id: id_to_text(user.0),
            secret_hash: secret_hash.to_string(),
        };
        run_upsert!(
            &mut *conn,
            credentials::table,
            credentials::user_id,
            row,
            |row: &CredentialRow| (credentials::secret_hash.eq(row.secret_hash.clone()),)
        );
        Ok(())
    }

    /// Read a member's stored secret hash from this tenant's `credentials` table, or `None`
    /// if no credential is set for `user`.
    pub fn credential(&self, user: UserId) -> Option<String> {
        let mut conn = self.conn.borrow_mut();
        credentials::table
            .filter(credentials::user_id.eq(id_to_text(user.0)))
            .select(credentials::secret_hash)
            .first::<String>(&mut *conn)
            .optional()
            .expect("credential: query failed (corrupt store)")
    }

    /// Insert a one-time pairing code into THIS tenant's `pairing_codes` table (ADR SQUIRE-A-0010).
    ///
    /// Like [`set_credential`](Self::set_credential), this is auth material written directly by the
    /// identity layer (NOT a domain [`Change`] — bypasses `apply`, no audit, not in the event log).
    /// `code_hash` is `sha256(code)` hex; the plaintext code is never stored. `expires_at` is unix
    /// millis. The code is consumed (and deleted) via [`take_pairing_code`](Self::take_pairing_code).
    pub fn insert_pairing_code(
        &self,
        code_hash: &str,
        user: UserId,
        role: Role,
        expires_at: i64,
    ) -> Result<(), RepoError> {
        let mut conn = self.conn.borrow_mut();
        let row = PairingCodeRow {
            code_hash: code_hash.to_string(),
            user_id: id_to_text(user.0),
            role: match role {
                Role::Knight => "Knight",
                Role::Squire => "Squire",
            }
            .to_string(),
            expires_at,
        };
        diesel::insert_into(pairing_codes::table)
            .values(&row)
            .execute(&mut *conn)
            .map_err(map_err)?;
        Ok(())
    }

    /// Look up a pairing code by its `sha256(code)` hash and **delete it** (single-use), returning
    /// `(target_user, role, expires_at)` if present. Returns `None` for an unknown code. The caller
    /// (identity) enforces the expiry against the clock; the row is removed on access either way, so
    /// a presented code is always spent. Atomic with respect to other writers because the caller
    /// holds the single-writer store lock for the whole consume.
    pub fn take_pairing_code(&self, code_hash: &str) -> Option<(UserId, Role, i64)> {
        let mut conn = self.conn.borrow_mut();
        let row: PairingCodeRow = pairing_codes::table
            .filter(pairing_codes::code_hash.eq(code_hash))
            .first::<PairingCodeRow>(&mut *conn)
            .optional()
            .expect("take_pairing_code: query failed (corrupt store)")?;
        diesel::delete(pairing_codes::table.filter(pairing_codes::code_hash.eq(code_hash)))
            .execute(&mut *conn)
            .expect("take_pairing_code: delete failed (corrupt store)");
        let user = UserId(row.user_id.parse::<u128>().ok()?);
        let role = match row.role.as_str() {
            "Knight" => Role::Knight,
            "Squire" => Role::Squire,
            _ => return None,
        };
        Some((user, role, row.expires_at))
    }

    /// Read a single household-config setting (ADR SQUIRE-A-0011) from THIS tenant's `config`
    /// table by `key`, or `None` if unset.
    pub fn get_setting(&self, key: &str) -> Option<String> {
        let mut conn = self.conn.borrow_mut();
        config::table
            .filter(config::key.eq(key))
            .select(config::value)
            .first::<String>(&mut *conn)
            .optional()
            .expect("get_setting: query failed (corrupt store)")
    }

    /// Upsert a household-config setting (ADR SQUIRE-A-0011): write `value` for `key`, stamping
    /// `updated_by` (the acting Knight; `None` for system/seed writes) and `updated_at` from the
    /// clock. Settings material written directly — NOT a domain [`Change`] (no event), but
    /// audit-stamped per A-0007. Upsert by `key`, so re-setting replaces the value.
    pub fn set_setting(&self, key: &str, value: &str, by: Option<UserId>) -> Result<(), RepoError> {
        let mut conn = self.conn.borrow_mut();
        let row = ConfigRow {
            key: key.to_string(),
            value: value.to_string(),
            updated_by: by.map(|u| id_to_text(u.0)),
            updated_at: self.clock.now().0,
        };
        run_upsert!(
            &mut *conn,
            config::table,
            config::key,
            row,
            |row: &ConfigRow| (
                config::value.eq(row.value.clone()),
                config::updated_by.eq(row.updated_by.clone()),
                config::updated_at.eq(row.updated_at),
            )
        );
        Ok(())
    }

    /// Assemble the typed [`HouseholdConfig`] view from the `config` rows, defaulting any absent or
    /// blank keys (ADR SQUIRE-A-0011). Never fails — a missing setting falls back to its default.
    pub fn load_config(&self) -> HouseholdConfig {
        let mut cfg = HouseholdConfig::default();
        if let Some(tz) = self.get_setting(config_keys::TIMEZONE) {
            if !tz.is_empty() {
                cfg.timezone = tz;
            }
        }
        // An unparseable or out-of-range stored value falls back to the default, so the view never
        // fails to assemble (ADR SQUIRE-A-0011).
        let minute = |key: &str| {
            self.get_setting(key)
                .and_then(|v| v.trim().parse::<u16>().ok())
                .filter(|m| domain_core::contract::valid_minute_of_day(*m))
        };
        if let Some(m) = minute(config_keys::NOTIFY_WAKE_FROM) {
            cfg.notify_wake_from = m;
        }
        if let Some(m) = minute(config_keys::NOTIFY_WAKE_TO) {
            cfg.notify_wake_to = m;
        }
        cfg
    }
}

/// A `credentials` row: a member's hashed secret keyed by `UserId` (no audit columns — this
/// table is written directly by the identity layer, not through `apply`).
#[derive(Debug, Clone, Insertable, Queryable, Selectable)]
#[diesel(table_name = credentials)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
struct CredentialRow {
    user_id: String,
    secret_hash: String,
}

/// A `pairing_codes` row: a one-time device-pairing code (hashed) keyed by `code_hash`, with its
/// target member, role, and expiry. Auth material written directly by the identity layer (ADR
/// SQUIRE-A-0010); no audit columns, not in the event log.
#[derive(Debug, Clone, Insertable, Queryable, Selectable)]
#[diesel(table_name = pairing_codes)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
struct PairingCodeRow {
    code_hash: String,
    user_id: String,
    role: String,
    expires_at: i64,
}

/// A `config` row: one household setting (ADR SQUIRE-A-0011) keyed by `key`, with the acting Knight
/// + time stamped on write (A-0007). Settings material written directly, not through `apply`.
#[derive(Debug, Clone, Insertable, Queryable, Selectable)]
#[diesel(table_name = config)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
struct ConfigRow {
    key: String,
    value: String,
    updated_by: Option<String>,
    updated_at: i64,
}

impl<C: Clock> Repository for Store<C> {
    fn snapshot(&self) -> Snapshot {
        // Decode failures are an IO/corruption bug, not normal control flow, so panic to
        // match the trait's infallible signature.
        load_snapshot(&mut self.conn.borrow_mut())
            .expect("snapshot: read or decode failed (corrupt store)")
    }

    fn apply(&mut self, by: Option<UserId>, changes: &[Change]) -> Result<(), RepoError> {
        let now = self.clock.now();
        self.conn
            .get_mut()
            .transaction::<(), TxnError, _>(|conn| {
                apply_changes(conn, by, now, changes).map_err(TxnError)
            })
            .map_err(|TxnError(e)| e)
    }
}

/// Load the full [`Snapshot`]: every row of each table, ordered by its key (events by
/// `seq` ascending), decoded back into the domain type. Audit columns are read but dropped.
fn load_snapshot(conn: &mut AnyConnection) -> Result<Snapshot, RepoError> {
    let user_rows: Vec<UserRow> = users::table
        .select(UserRow::as_select())
        .order(users::id.asc())
        .load(conn)
        .map_err(map_err)?;
    let quest_rows: Vec<QuestRow> = quests::table
        .select(QuestRow::as_select())
        .order(quests::id.asc())
        .load(conn)
        .map_err(map_err)?;
    let item_rows: Vec<ItemRow> = items::table
        .select(ItemRow::as_select())
        .order(items::id.asc())
        .load(conn)
        .map_err(map_err)?;
    let ach_rows: Vec<AchievementRow> = achievements::table
        .select(AchievementRow::as_select())
        .order(achievements::id.asc())
        .load(conn)
        .map_err(map_err)?;
    let event_rows: Vec<EventRow> = events::table
        .select(EventRow::as_select())
        .order(events::seq.asc())
        .load(conn)
        .map_err(map_err)?;

    let users = user_rows
        .iter()
        .map(|r| r.to_user())
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_row_err)?;
    let quests = quest_rows
        .iter()
        .map(|r| r.to_quest())
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_row_err)?;
    let items = item_rows
        .iter()
        .map(|r| r.to_item())
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_row_err)?;
    let achievements = ach_rows
        .iter()
        .map(|r| r.to_achievement())
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_row_err)?;
    let events = event_rows
        .iter()
        .map(|r| r.to_event())
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_row_err)?;

    Ok(Snapshot {
        users,
        quests,
        items,
        achievements,
        events,
    })
}

/// Apply a batch of [`Change`]s inside the caller's transaction. Single-writer + in-txn, so
/// `seq` is assigned without a race: compute `max(seq)` once, increment per appended event.
fn apply_changes(
    conn: &mut AnyConnection,
    by: Option<UserId>,
    now: Timestamp,
    changes: &[Change],
) -> Result<(), RepoError> {
    // One read of the high-water mark; events are append-only so it only grows as we go.
    let mut next_seq: i64 = events::table
        .select(diesel::dsl::max(events::seq))
        .first::<Option<i64>>(conn)
        .map_err(map_err)?
        .unwrap_or(0)
        + 1;

    for change in changes {
        match change {
            Change::Append(ev) => {
                append_event(conn, next_seq, ev)?;
                next_seq += 1;
            }
            Change::PutUser(u) => upsert_user(conn, by, now, u)?,
            Change::PutQuest(q) => upsert_quest(conn, by, now, q)?,
            Change::PutItem(it) => upsert_item(conn, by, now, it)?,
            Change::PutAchievement(a) => upsert_achievement(conn, by, now, a)?,
            Change::SetUserActive(id, active) => set_user_active(conn, by, now, *id, *active)?,
            Change::SetQuestActive(id, active) => set_quest_active(conn, by, now, *id, *active)?,
            Change::SetItemActive(id, active) => set_item_active(conn, by, now, *id, *active)?,
            Change::SetAchievementActive(id, active) => {
                set_achievement_active(conn, by, now, *id, *active)?
            }
        }
    }
    Ok(())
}

/// Build the audit stamp for an upsert. On a fresh insert all four columns are set; on
/// conflict the `.set(...)` clauses below deliberately omit `created_*`, so those keep
/// their existing values and only `updated_*` move.
fn fresh_audit(by: Option<UserId>, now: Timestamp) -> Audit {
    Audit {
        created_by: by,
        created_at: now,
        updated_by: by,
        updated_at: now,
    }
}

fn append_event(conn: &mut AnyConnection, seq: i64, ev: &Event) -> Result<(), RepoError> {
    let row = EventRow::from_event(seq, ev);
    diesel::insert_into(events::table)
        .values(&row)
        .execute(conn)
        .map_err(map_err)?;
    Ok(())
}

// ─── upserts ──────────────────────────────────────────────────────────────────
//
// Each upsert preserves `created_*` on update by listing exactly the mutable columns plus
// `updated_by`/`updated_at` in `.set(...)`. The `created_by`/`created_at` columns are NOT
// in the set, so a conflicting (existing) row keeps whatever it was first inserted with;
// only a brand-new row gets `created_* = (by, now)` from the inserted row values. This is
// portable across SQLite and Postgres in Diesel 2 (`on_conflict(id).do_update()`).

fn upsert_user(
    conn: &mut AnyConnection,
    by: Option<UserId>,
    now: Timestamp,
    u: &User,
) -> Result<(), RepoError> {
    run_upsert!(
        conn,
        users::table,
        users::id,
        UserRow::from_user(u, fresh_audit(by, now)),
        |row: &UserRow| (
            users::role.eq(row.role.clone()),
            users::display_name.eq(row.display_name.clone()),
            users::active.eq(row.active),
            users::updated_by.eq(row.updated_by.clone()),
            users::updated_at.eq(row.updated_at),
        )
    );
    Ok(())
}

fn upsert_quest(
    conn: &mut AnyConnection,
    by: Option<UserId>,
    now: Timestamp,
    q: &Quest,
) -> Result<(), RepoError> {
    run_upsert!(
        conn,
        quests::table,
        quests::id,
        QuestRow::from_quest(q, fresh_audit(by, now)),
        |row: &QuestRow| (
            quests::title.eq(row.title.clone()),
            quests::description.eq(row.description.clone()),
            quests::category.eq(row.category.clone()),
            quests::reward.eq(row.reward),
            quests::cadence_kind.eq(row.cadence_kind.clone()),
            quests::cadence_due.eq(row.cadence_due),
            quests::cadence_weekdays.eq(row.cadence_weekdays.clone()),
            quests::cadence_n.eq(row.cadence_n),
            quests::cadence_anchor.eq(row.cadence_anchor),
            quests::assignment_kind.eq(row.assignment_kind.clone()),
            quests::assignment_squires.eq(row.assignment_squires.clone()),
            quests::completion.eq(row.completion.clone()),
            quests::auto_approve.eq(row.auto_approve),
            quests::repeatable_within_day.eq(row.repeatable_within_day),
            quests::icon.eq(row.icon.clone()),
            quests::active.eq(row.active),
            quests::updated_by.eq(row.updated_by.clone()),
            quests::updated_at.eq(row.updated_at),
        )
    );
    Ok(())
}

fn upsert_item(
    conn: &mut AnyConnection,
    by: Option<UserId>,
    now: Timestamp,
    it: &RedeemableItem,
) -> Result<(), RepoError> {
    run_upsert!(
        conn,
        items::table,
        items::id,
        ItemRow::from_item(it, fresh_audit(by, now)),
        |row: &ItemRow| (
            items::name.eq(row.name.clone()),
            items::description.eq(row.description.clone()),
            items::cost.eq(row.cost),
            items::gate.eq(row.gate.clone()),
            items::availability.eq(row.availability.clone()),
            items::icon.eq(row.icon.clone()),
            items::active.eq(row.active),
            items::updated_by.eq(row.updated_by.clone()),
            items::updated_at.eq(row.updated_at),
        )
    );
    Ok(())
}

fn upsert_achievement(
    conn: &mut AnyConnection,
    by: Option<UserId>,
    now: Timestamp,
    a: &Achievement,
) -> Result<(), RepoError> {
    run_upsert!(
        conn,
        achievements::table,
        achievements::id,
        AchievementRow::from_achievement(a, fresh_audit(by, now)),
        |row: &AchievementRow| (
            achievements::name.eq(row.name.clone()),
            achievements::description.eq(row.description.clone()),
            achievements::criterion_kind.eq(row.criterion_kind.clone()),
            achievements::scope_kind.eq(row.scope_kind.clone()),
            achievements::scope_quest.eq(row.scope_quest.clone()),
            achievements::scope_category.eq(row.scope_category.clone()),
            achievements::streak_length.eq(row.streak_length),
            achievements::streak_basis.eq(row.streak_basis.clone()),
            achievements::total_count.eq(row.total_count),
            achievements::points_total.eq(row.points_total),
            achievements::bonus_points.eq(row.bonus_points),
            achievements::active.eq(row.active),
            achievements::updated_by.eq(row.updated_by.clone()),
            achievements::updated_at.eq(row.updated_at),
        )
    );
    Ok(())
}

// ─── set-active ───────────────────────────────────────────────────────────────
//
// Update `active` + the `updated_*` audit on an existing row. Erroring on a missing row
// is acceptable: the engine only emits these for defs it already knows exist.

fn updated_by_text(by: Option<UserId>) -> Option<String> {
    by.map(|u| id_to_text(u.0))
}

/// A successful `UPDATE` that matched zero rows means the target id was missing — surface a
/// [`RepoError::Conflict`] so the whole batch rolls back.
fn require_one(n: usize) -> Result<(), RepoError> {
    if n == 0 {
        Err(RepoError::Conflict)
    } else {
        Ok(())
    }
}

fn set_user_active(
    conn: &mut AnyConnection,
    by: Option<UserId>,
    now: Timestamp,
    id: UserId,
    active: bool,
) -> Result<(), RepoError> {
    let n = diesel::update(users::table.filter(users::id.eq(id_to_text(id.0))))
        .set((
            users::active.eq(i32::from(active)),
            users::updated_by.eq(updated_by_text(by)),
            users::updated_at.eq(now.0),
        ))
        .execute(conn)
        .map_err(map_err)?;
    require_one(n)
}

fn set_quest_active(
    conn: &mut AnyConnection,
    by: Option<UserId>,
    now: Timestamp,
    id: QuestId,
    active: bool,
) -> Result<(), RepoError> {
    let n = diesel::update(quests::table.filter(quests::id.eq(id_to_text(id.0))))
        .set((
            quests::active.eq(i32::from(active)),
            quests::updated_by.eq(updated_by_text(by)),
            quests::updated_at.eq(now.0),
        ))
        .execute(conn)
        .map_err(map_err)?;
    require_one(n)
}

fn set_item_active(
    conn: &mut AnyConnection,
    by: Option<UserId>,
    now: Timestamp,
    id: ItemId,
    active: bool,
) -> Result<(), RepoError> {
    let n = diesel::update(items::table.filter(items::id.eq(id_to_text(id.0))))
        .set((
            items::active.eq(i32::from(active)),
            items::updated_by.eq(updated_by_text(by)),
            items::updated_at.eq(now.0),
        ))
        .execute(conn)
        .map_err(map_err)?;
    require_one(n)
}

fn set_achievement_active(
    conn: &mut AnyConnection,
    by: Option<UserId>,
    now: Timestamp,
    id: AchievementId,
    active: bool,
) -> Result<(), RepoError> {
    let n = diesel::update(achievements::table.filter(achievements::id.eq(id_to_text(id.0))))
        .set((
            achievements::active.eq(i32::from(active)),
            achievements::updated_by.eq(updated_by_text(by)),
            achievements::updated_at.eq(now.0),
        ))
        .execute(conn)
        .map_err(map_err)?;
    require_one(n)
}

// ─── audit accessors (test / introspection support) ───────────────────────────
//
// The `created_*` / `updated_*` columns are deliberately NOT part of the `Snapshot`
// (the domain stays pure). These helpers read them back for audit-stamping tests and
// any future introspection. `by` columns come back as `Option<UserId>` (NULL = system).

/// The audit stamp on a definition / identity row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuditCols {
    pub created_by: Option<UserId>,
    pub created_at: Timestamp,
    pub updated_by: Option<UserId>,
    pub updated_at: Timestamp,
}

fn audit_from_parts(
    created_by: Option<String>,
    created_at: i64,
    updated_by: Option<String>,
    updated_at: i64,
) -> Result<AuditCols, RepoError> {
    let parse = |s: Option<String>| -> Result<Option<UserId>, RepoError> {
        match s {
            Some(s) => Ok(Some(UserId(
                crate::rows::text_to_id("audit", &s).map_err(map_row_err)?,
            ))),
            None => Ok(None),
        }
    };
    Ok(AuditCols {
        created_by: parse(created_by)?,
        created_at: Timestamp(created_at),
        updated_by: parse(updated_by)?,
        updated_at: Timestamp(updated_at),
    })
}

/// Read the audit columns of the `quests` row with this id (`None` if absent).
pub fn quest_audit(conn: &mut AnyConnection, id: QuestId) -> Result<Option<AuditCols>, RepoError> {
    let row: Option<(Option<String>, i64, Option<String>, i64)> = quests::table
        .filter(quests::id.eq(id_to_text(id.0)))
        .select((
            quests::created_by,
            quests::created_at,
            quests::updated_by,
            quests::updated_at,
        ))
        .first(conn)
        .optional()
        .map_err(map_err)?;
    row.map(|(cb, ca, ub, ua)| audit_from_parts(cb, ca, ub, ua))
        .transpose()
}

/// Read the audit columns of the `users` row with this id (`None` if absent).
pub fn user_audit(conn: &mut AnyConnection, id: UserId) -> Result<Option<AuditCols>, RepoError> {
    let row: Option<(Option<String>, i64, Option<String>, i64)> = users::table
        .filter(users::id.eq(id_to_text(id.0)))
        .select((
            users::created_by,
            users::created_at,
            users::updated_by,
            users::updated_at,
        ))
        .first(conn)
        .optional()
        .map_err(map_err)?;
    row.map(|(cb, ca, ub, ua)| audit_from_parts(cb, ca, ub, ua))
        .transpose()
}

/// Read the audit columns of the `items` row with this id (`None` if absent).
pub fn item_audit(conn: &mut AnyConnection, id: ItemId) -> Result<Option<AuditCols>, RepoError> {
    let row: Option<(Option<String>, i64, Option<String>, i64)> = items::table
        .filter(items::id.eq(id_to_text(id.0)))
        .select((
            items::created_by,
            items::created_at,
            items::updated_by,
            items::updated_at,
        ))
        .first(conn)
        .optional()
        .map_err(map_err)?;
    row.map(|(cb, ca, ub, ua)| audit_from_parts(cb, ca, ub, ua))
        .transpose()
}

/// Read the audit columns of the `achievements` row with this id (`None` if absent).
pub fn achievement_audit(
    conn: &mut AnyConnection,
    id: AchievementId,
) -> Result<Option<AuditCols>, RepoError> {
    let row: Option<(Option<String>, i64, Option<String>, i64)> = achievements::table
        .filter(achievements::id.eq(id_to_text(id.0)))
        .select((
            achievements::created_by,
            achievements::created_at,
            achievements::updated_by,
            achievements::updated_at,
        ))
        .first(conn)
        .optional()
        .map_err(map_err)?;
    row.map(|(cb, ca, ub, ua)| audit_from_parts(cb, ca, ub, ua))
        .transpose()
}

// ─── SQLite convenience store ─────────────────────────────────────────────────

/// A SQLite-backed store with the real [`SystemClock`]. Convenience wrapper around the
/// generic [`Store`] for the common single-backend case; [`SqliteStore::open`] applies
/// pending migrations on open.
pub struct SqliteStore {
    inner: Store<SystemClock>,
}

impl SqliteStore {
    /// Open (or create) a SQLite database at `database_url`, apply pending migrations, and
    /// wrap it with the real [`SystemClock`]. `database_url` may be a file path or
    /// `":memory:"`.
    pub fn open(database_url: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = SqliteConnection::establish(database_url)?;
        conn.run_pending_migrations(MIGRATIONS)?;
        Ok(Self {
            inner: Store::new(AnyConnection::Sqlite(conn), SystemClock),
        })
    }

    /// Borrow the underlying connection (for later tasks / advanced use).
    pub fn connection(&mut self) -> std::cell::RefMut<'_, AnyConnection> {
        self.inner.connection()
    }
}

impl Repository for SqliteStore {
    fn snapshot(&self) -> Snapshot {
        self.inner.snapshot()
    }

    fn apply(&mut self, by: Option<UserId>, changes: &[Change]) -> Result<(), RepoError> {
        self.inner.apply(by, changes)
    }
}

// ─── clocks ───────────────────────────────────────────────────────────────────

/// Day offset that re-aligns the unix epoch (Thursday 1970-01-01) onto the domain's
/// `Date(0) == Monday` convention (see `domain_core::common::weekday_of`).
///
/// `days_since_unix_epoch` would make `Date(0)` a **Thursday**, which breaks Weekly
/// schedules. We instead count days since **Monday 1969-12-29**, i.e. add 3 to the unix
/// day-count, so `Date(0)` is a Monday and `weekday_of` lines up with the real calendar.
pub(crate) const UNIX_TO_MONDAY_EPOCH_OFFSET: i64 = 3;

const MILLIS_PER_DAY: i64 = 86_400_000;

/// Current unix time in **milliseconds** (UTC), clamped to 0 before the epoch on a misconfigured
/// host. Shared by [`SystemClock`] and [`crate::settings::LocalClock`] so `now()` is defined once.
pub(crate) fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Real wall-clock [`Clock`], **UTC**. `now()` is unix millis; `today()` is the Monday-aligned
/// day-count in UTC. The household-local clock is [`crate::settings::LocalClock`] (ADR A-0011).
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

/// Convert unix millis to the domain's Monday-aligned `Date` (UTC). Shared by
/// [`SystemClock`] and [`FixedClock`] so the alignment convention lives in one place.
pub(crate) fn date_from_unix_millis(millis: i64) -> Date {
    // Floor-divide so negative (pre-epoch) instants still land on the right calendar day.
    let days = millis.div_euclid(MILLIS_PER_DAY);
    Date((days + UNIX_TO_MONDAY_EPOCH_OFFSET) as i32)
}

impl Clock for SystemClock {
    fn today(&self) -> Date {
        date_from_unix_millis(self.now().0)
    }

    fn now(&self) -> Timestamp {
        Timestamp(now_millis())
    }
}

/// Deterministic test clock with settable `today`/`now`. Lets `apply`'s audit stamping
/// and any clock-driven logic be exercised without touching the wall clock.
#[derive(Clone, Copy, Debug)]
pub struct FixedClock {
    pub today: Date,
    pub now: Timestamp,
}

impl FixedClock {
    /// A clock fixed at `now` (unix millis); `today` is derived via the same Monday-aligned
    /// rule [`SystemClock`] uses.
    pub fn at(now: Timestamp) -> Self {
        Self {
            today: date_from_unix_millis(now.0),
            now,
        }
    }

    /// A clock with explicitly chosen `today` and `now`.
    pub fn new(today: Date, now: Timestamp) -> Self {
        Self { today, now }
    }
}

impl Clock for FixedClock {
    fn today(&self) -> Date {
        self.today
    }

    fn now(&self) -> Timestamp {
        self.now
    }
}
