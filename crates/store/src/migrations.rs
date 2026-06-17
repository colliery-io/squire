//! Embedded migration runner.

use diesel::Connection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

/// The migrations embedded from `crates/store/migrations/` at compile time.
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

/// Run any pending embedded migrations against `conn`.
///
/// Works for any Diesel connection that implements `MigrationHarness` for its backend
/// (SQLite today; Postgres under the `postgres` feature).
pub fn run_migrations<C>(conn: &mut C) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    C: Connection + MigrationHarness<C::Backend>,
{
    conn.run_pending_migrations(MIGRATIONS)?;
    Ok(())
}
