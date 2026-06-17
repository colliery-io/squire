//! Per-tenant single-file backup: export + import/restore (SQUIRE-T-0012).
//!
//! GOAL: capture the ENTIRE household store — users + definitions (quests/items/
//! achievements) + the complete event log, **including the audit columns
//! (`created_by/at`, `updated_by/at`) and the append-only `seq`** — into ONE file, and
//! round-trip it back into a fresh, equal store.
//!
//! ## Chosen format
//!
//! A **backend-portable serialized dump**: the five row structs from [`crate::rows`] are
//! serialized verbatim to a single JSON document ([`Dump`]). This is portable because it
//! captures the store at the *row* layer — the same shape Diesel reads/writes on both
//! SQLite and Postgres — rather than a backend-specific binary file (an `.sqlite` file or a
//! `pg_dump`). A dump produced from a SQLite tenant imports cleanly into a Postgres tenant
//! and vice-versa. Because the row structs carry every column (including audit + `seq`),
//! nothing is lost: this is NOT a domain `Snapshot` (which deliberately drops audit/seq) but
//! the full storage row.
//!
//! ## Consistency / no torn export
//!
//! [`export`] reads all five tables inside a single READ transaction, so the dump is a
//! coherent point-in-time snapshot — it can never capture the store mid-`apply` (half of an
//! event batch written, the other half not). The store is single-writer anyway, but the
//! read-txn makes the "coherent snapshot" guarantee explicit and backend-enforced.
//!
//! ## Import is a verbatim bulk load
//!
//! [`import`] INSERTs every row exactly as serialized into a FRESH (already-migrated, empty)
//! store, inside one transaction. It does NOT go through `apply`, so audit columns and `seq`
//! are restored as-is rather than re-stamped — the restored store is byte-for-byte equal to
//! the original.

use std::path::Path;

use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::rows::{AchievementRow, EventRow, ItemRow, QuestRow, UserRow};
use crate::schema::{achievements, events, items, quests, users};
use crate::AnyConnection;

/// The complete, portable dump of a single household store: every row of every table,
/// carrying audit columns and `seq` verbatim. Events are stored in `seq` order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dump {
    pub users: Vec<UserRow>,
    pub quests: Vec<QuestRow>,
    pub items: Vec<ItemRow>,
    pub achievements: Vec<AchievementRow>,
    pub events: Vec<EventRow>,
}

/// A backup (export / import) failure.
#[derive(Debug)]
pub enum BackupError {
    /// A database / query / transaction error.
    Db(diesel::result::Error),
    /// A filesystem read/write error for the dump file.
    Io(std::io::Error),
    /// (De)serialization of the JSON dump failed.
    Serde(serde_json::Error),
}

impl std::fmt::Display for BackupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackupError::Db(e) => write!(f, "backup database error: {e}"),
            BackupError::Io(e) => write!(f, "backup file error: {e}"),
            BackupError::Serde(e) => write!(f, "backup (de)serialization error: {e}"),
        }
    }
}

impl std::error::Error for BackupError {}

impl From<diesel::result::Error> for BackupError {
    fn from(e: diesel::result::Error) -> Self {
        BackupError::Db(e)
    }
}

impl From<std::io::Error> for BackupError {
    fn from(e: std::io::Error) -> Self {
        BackupError::Io(e)
    }
}

impl From<serde_json::Error> for BackupError {
    fn from(e: serde_json::Error) -> Self {
        BackupError::Serde(e)
    }
}

/// Read ALL rows from all five tables into a [`Dump`] (events ordered by `seq`) and write it
/// to ONE file at `path` as pretty JSON.
///
/// The reads run inside a single read transaction so the export is a coherent, non-torn
/// point-in-time snapshot of the whole store (see module docs).
pub fn export(conn: &mut AnyConnection, path: impl AsRef<Path>) -> Result<(), BackupError> {
    let dump = conn.transaction::<Dump, diesel::result::Error, _>(read_dump)?;
    let json = serde_json::to_string_pretty(&dump)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Read every row of every table into a [`Dump`]. Called inside a transaction by [`export`].
fn read_dump(conn: &mut AnyConnection) -> Result<Dump, diesel::result::Error> {
    Ok(Dump {
        users: users::table
            .select(UserRow::as_select())
            .order(users::id.asc())
            .load(conn)?,
        quests: quests::table
            .select(QuestRow::as_select())
            .order(quests::id.asc())
            .load(conn)?,
        items: items::table
            .select(ItemRow::as_select())
            .order(items::id.asc())
            .load(conn)?,
        achievements: achievements::table
            .select(AchievementRow::as_select())
            .order(achievements::id.asc())
            .load(conn)?,
        events: events::table
            .select(EventRow::as_select())
            .order(events::seq.asc())
            .load(conn)?,
    })
}

/// Read the JSON [`Dump`] at `path` and INSERT every row verbatim into a FRESH (already
/// migrated, empty) store, preserving `seq`, audit columns, and all values exactly.
///
/// This is a bulk load, NOT an `apply`, so audit/seq are restored as-is (not re-stamped).
/// Everything runs inside a single transaction: a partial restore is impossible.
pub fn import(conn: &mut AnyConnection, path: impl AsRef<Path>) -> Result<(), BackupError> {
    let json = std::fs::read_to_string(path)?;
    let dump: Dump = serde_json::from_str(&json)?;
    conn.transaction::<(), diesel::result::Error, _>(|conn| insert_dump(conn, &dump))?;
    Ok(())
}

/// Insert every row of `dump` verbatim. Called inside a transaction by [`import`].
///
/// Diesel's erased `MultiConnection` cannot type-check a multi-row `BatchInsert`, so — like
/// the upserts in `lib.rs` — each insert dispatches on the enum and runs against the CONCRETE
/// connection (`SqliteConnection` / `PgConnection`), both of which support batch insert. The
/// `$rows` slice is identical per backend; the macro avoids hand-duplicating it.
fn insert_dump(conn: &mut AnyConnection, dump: &Dump) -> Result<(), diesel::result::Error> {
    macro_rules! insert_all {
        ($table:path, $rows:expr) => {{
            match conn {
                AnyConnection::Sqlite(c) => {
                    diesel::insert_into($table).values($rows).execute(c)?;
                }
                #[cfg(feature = "postgres")]
                AnyConnection::Pg(c) => {
                    diesel::insert_into($table).values($rows).execute(c)?;
                }
            }
        }};
    }

    insert_all!(users::table, &dump.users);
    insert_all!(quests::table, &dump.quests);
    insert_all!(items::table, &dump.items);
    insert_all!(achievements::table, &dump.achievements);
    // Events carry an explicit `seq` PRIMARY KEY value, inserted verbatim so the append-only
    // ordering is preserved exactly (not re-generated).
    insert_all!(events::table, &dump.events);
    Ok(())
}
