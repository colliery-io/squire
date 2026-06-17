//! Connection / backend abstraction.
//!
//! The same repo code targets SQLite today and Postgres later. We use Diesel 2.2's
//! `#[derive(diesel::MultiConnection)]` over an enum: the `Sqlite` arm is always
//! present; the `Pg` arm only compiles when the `postgres` feature is on (which pulls
//! diesel's postgres backend and requires a system libpq at link time). The DEFAULT
//! build is therefore SQLite-only and needs no libpq.

use diesel::sqlite::SqliteConnection;

#[cfg(feature = "postgres")]
use diesel::pg::PgConnection;

/// A backend-agnostic connection. `snapshot`/`apply` in later tasks operate against
/// this so they stay backend-portable.
#[derive(diesel::MultiConnection)]
pub enum AnyConnection {
    Sqlite(SqliteConnection),
    #[cfg(feature = "postgres")]
    Pg(PgConnection),
}
