//! Squire `store` — Diesel-backed persistence for the domain core.
//!
//! Task SQUIRE-T-0008: SCAFFOLD ONLY. This crate stands up the schema, migrations, a
//! backend abstraction, and a migration runner. The real `Repository::snapshot` /
//! `Repository::apply` bodies arrive in later tasks; here they are stubs.
//!
//! Default build is SQLite-only (bundled SQLite, no system libpq). Postgres is an
//! opt-in `postgres` cargo feature.

pub mod conn;
pub mod migrations;
pub mod schema;

pub use conn::AnyConnection;
pub use migrations::{run_migrations, MIGRATIONS};

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
             SELECT 1 FROM events LIMIT 0;",
        )?;
        Ok(())
    }
}

use diesel::sqlite::SqliteConnection;
use diesel::Connection;
use diesel_migrations::MigrationHarness;

use domain_core::contract::{
    Change, Clock, Date, Repository, RepoError, Snapshot, Timestamp, UserId,
};

/// A SQLite-backed store. Holds an open connection; migrations should be run via
/// [`SqliteStore::open`] (which applies pending migrations) before use.
pub struct SqliteStore {
    conn: SqliteConnection,
}

impl SqliteStore {
    /// Open (or create) a SQLite database at `database_url` and apply pending
    /// migrations. `database_url` may be a file path or `":memory:"`.
    pub fn open(
        database_url: &str,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = SqliteConnection::establish(database_url)?;
        conn.run_pending_migrations(MIGRATIONS)?;
        Ok(Self { conn })
    }

    /// Borrow the underlying connection (for later tasks / advanced use).
    pub fn connection(&mut self) -> &mut SqliteConnection {
        &mut self.conn
    }
}

impl Repository for SqliteStore {
    fn snapshot(&self) -> Snapshot {
        // Implemented in a later task (read-side hydration).
        todo!("SqliteStore::snapshot — implemented in a later task")
    }

    fn apply(&mut self, _by: Option<UserId>, _changes: &[Change]) -> Result<(), RepoError> {
        // Implemented in a later task (single-writer change application).
        todo!("SqliteStore::apply — implemented in a later task")
    }
}

/// A trivial system clock placeholder. The domain core carries no time dependency, so
/// the concrete clock lives outside it. This stub will be fleshed out (real wall-clock /
/// timezone handling) in a later task.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn today(&self) -> Date {
        // Implemented in a later task.
        todo!("SystemClock::today — implemented in a later task")
    }

    fn now(&self) -> Timestamp {
        // Implemented in a later task.
        todo!("SystemClock::now — implemented in a later task")
    }
}
