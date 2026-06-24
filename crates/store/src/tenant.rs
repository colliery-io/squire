//! Schema-per-tenant provisioning + connection selection (ADR SQUIRE-A-0002).
//!
//! Each household (tenant) is **fully isolated** — its own SQLite file (local deployment)
//! or its own Postgres schema (hosted deployment). There is NO `tenant_id` discriminator
//! column anywhere: isolation is *structural* (a whole separate file / schema), not a
//! filtered shared table.
//!
//! This module provides the low-level provision / open / deprovision primitives keyed by an
//! **already-resolved** household handle. The household-handle → tenant *registry/routing*
//! (deciding which handle a given request belongs to) is OWNED BY SQUIRE-S-0007 and is out of
//! scope here.
//!
//! The [`Store`]/`Repository` code from T-0010 is reused unchanged: by the time a [`Store`] is
//! handed back, its connection is already tenant-scoped (a SQLite file for that handle, or a
//! Postgres connection whose `search_path` is pinned to that handle's schema), so the same
//! `snapshot`/`apply` bodies operate on exactly one tenant's data.
//!
//! ## Handle sanitization
//!
//! A handle is used to derive a filesystem path (SQLite) or a SQL schema name (Postgres), so
//! it MUST be constrained to avoid path traversal and SQL injection. The rule, enforced by
//! [`sanitize_handle`]:
//!
//! * allowed characters are exactly `[a-z0-9_]` (lowercase ASCII letters, digits, underscore);
//! * the handle must be non-empty and at most 48 characters;
//! * any other character (uppercase, `.`, `/`, `-`, whitespace, quotes, …) is **rejected**
//!   (not escaped) with [`ProvisionError::InvalidHandle`].
//!
//! Because only `[a-z0-9_]` survives, the Postgres schema name `t_<handle>` and the SQLite
//! file name `<handle>.sqlite` are always safe to interpolate. The `t_` prefix also keeps
//! tenant schemas clear of the reserved `public` schema and any pg_catalog names.

use std::path::PathBuf;

#[cfg(feature = "postgres")]
use diesel::connection::SimpleConnection; // batch_execute, only used for PG schema setup
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use domain_core::contract::Clock;

use crate::conn::AnyConnection;
use crate::migrations::run_migrations;
use crate::Store;

#[cfg(feature = "postgres")]
use diesel::pg::PgConnection;

/// Backend selected once at startup. Determines whether tenants are isolated as SQLite files
/// (local) or Postgres schemas (hosted).
#[derive(Clone, Debug)]
pub enum Backend {
    /// SQLite: each household handle maps to the file `<dir>/<handle>.sqlite`.
    Sqlite {
        /// Base directory holding one SQLite file per household.
        dir: PathBuf,
    },
    /// Postgres: each household handle maps to a schema named `t_<sanitized_handle>` inside
    /// the database addressed by `base_url`.
    #[cfg(feature = "postgres")]
    Postgres {
        /// Base Postgres connection URL (database is shared; tenants are separate schemas).
        base_url: String,
    },
}

/// Failure provisioning, opening, or deprovisioning a tenant.
#[derive(Debug)]
pub enum ProvisionError {
    /// The handle contained characters outside `[a-z0-9_]`, was empty, or was too long.
    InvalidHandle(String),
    /// A filesystem error (SQLite backend): create/delete of the tenant file failed.
    Io(std::io::Error),
    /// A database/connection/migration error.
    Backend(String),
}

impl std::fmt::Display for ProvisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProvisionError::InvalidHandle(h) => {
                write!(
                    f,
                    "invalid household handle {h:?}: only [a-z0-9_], 1..=48 chars allowed"
                )
            }
            ProvisionError::Io(e) => write!(f, "tenant filesystem error: {e}"),
            ProvisionError::Backend(e) => write!(f, "tenant backend error: {e}"),
        }
    }
}

impl std::error::Error for ProvisionError {}

impl From<std::io::Error> for ProvisionError {
    fn from(e: std::io::Error) -> Self {
        ProvisionError::Io(e)
    }
}

/// Max handle length. Postgres identifiers are capped at 63 bytes; `t_` + 48 leaves headroom.
const MAX_HANDLE_LEN: usize = 48;

/// Validate a household handle and return it (borrowed back) if it is safe to use as a
/// schema / file-name component. See the module docs for the rule. Rejects (does not escape)
/// anything outside `[a-z0-9_]`.
pub fn sanitize_handle(handle: &str) -> Result<&str, ProvisionError> {
    let ok = !handle.is_empty()
        && handle.len() <= MAX_HANDLE_LEN
        && handle
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
    if ok {
        Ok(handle)
    } else {
        Err(ProvisionError::InvalidHandle(handle.to_string()))
    }
}

/// Holds the chosen [`Backend`] and provisions / opens / deprovisions tenants against it.
#[derive(Clone, Debug)]
pub struct Provisioner {
    backend: Backend,
}

impl Provisioner {
    /// Build a provisioner over the chosen backend.
    pub fn new(backend: Backend) -> Self {
        Self { backend }
    }

    /// Borrow the backend.
    pub fn backend(&self) -> &Backend {
        &self.backend
    }

    /// Create + migrate the tenant for `handle`.
    ///
    /// * SQLite: create the file `<dir>/<handle>.sqlite` (if absent) and run migrations.
    /// * Postgres: `CREATE SCHEMA IF NOT EXISTS t_<handle>`, pin `search_path` to it, run
    ///   migrations.
    ///
    /// Idempotent-ish: re-provisioning an existing tenant is a no-op for already-applied
    /// migrations (the migration harness skips them), so calling `provision` twice is safe.
    pub fn provision(&self, handle: &str) -> Result<(), ProvisionError> {
        let handle = sanitize_handle(handle)?;
        match &self.backend {
            Backend::Sqlite { dir } => {
                std::fs::create_dir_all(dir)?;
                let path = sqlite_path(dir, handle);
                let url = path_str(&path)?;
                let mut conn = SqliteConnection::establish(url)
                    .map_err(|e| ProvisionError::Backend(e.to_string()))?;
                run_migrations(&mut conn).map_err(|e| ProvisionError::Backend(e.to_string()))?;
                Ok(())
            }
            #[cfg(feature = "postgres")]
            Backend::Postgres { base_url } => {
                let schema = schema_name(handle);
                let mut conn = PgConnection::establish(base_url)
                    .map_err(|e| ProvisionError::Backend(e.to_string()))?;
                // `schema` is `t_<[a-z0-9_]+>` — safe to interpolate (no escaping needed).
                conn.batch_execute(&format!(
                    "CREATE SCHEMA IF NOT EXISTS {schema}; SET search_path TO {schema};"
                ))
                .map_err(|e| ProvisionError::Backend(e.to_string()))?;
                run_migrations(&mut conn).map_err(|e| ProvisionError::Backend(e.to_string()))?;
                Ok(())
            }
        }
    }

    /// Open a tenant-scoped [`Store`] for `handle`, wrapping an already tenant-scoped
    /// connection so the unchanged T-0010 `snapshot`/`apply` operate on exactly one tenant.
    ///
    /// * SQLite: open the tenant's file (also runs migrations, so `open` works even if the
    ///   caller skipped `provision`).
    /// * Postgres: establish a fresh `PgConnection` and immediately `SET search_path TO
    ///   t_<handle>`, so every query resolves to that schema's tables.
    pub fn open<C: Clock>(&self, handle: &str, clock: C) -> Result<Store<C>, ProvisionError> {
        let handle = sanitize_handle(handle)?;
        let conn = self.open_conn(handle)?;
        Ok(Store::new(conn, clock))
    }

    /// Open a tenant-scoped raw [`AnyConnection`] (advanced use; [`open`](Self::open) wraps
    /// this in a [`Store`]).
    pub fn open_conn(&self, handle: &str) -> Result<AnyConnection, ProvisionError> {
        let handle = sanitize_handle(handle)?;
        match &self.backend {
            Backend::Sqlite { dir } => {
                std::fs::create_dir_all(dir)?;
                let path = sqlite_path(dir, handle);
                let url = path_str(&path)?;
                let mut conn = SqliteConnection::establish(url)
                    .map_err(|e| ProvisionError::Backend(e.to_string()))?;
                run_migrations(&mut conn).map_err(|e| ProvisionError::Backend(e.to_string()))?;
                Ok(AnyConnection::Sqlite(conn))
            }
            #[cfg(feature = "postgres")]
            Backend::Postgres { base_url } => {
                let schema = schema_name(handle);
                let mut conn = PgConnection::establish(base_url)
                    .map_err(|e| ProvisionError::Backend(e.to_string()))?;
                conn.batch_execute(&format!("SET search_path TO {schema};"))
                    .map_err(|e| ProvisionError::Backend(e.to_string()))?;
                Ok(AnyConnection::Pg(conn))
            }
        }
    }

    /// Restore a per-tenant backup dump at `path` into the tenant for `handle`.
    ///
    /// The tenant must already be provisioned (migrated) and **empty** — `import` is a
    /// verbatim bulk load that preserves `seq` and audit columns as-is (see
    /// [`crate::backup::import`]). Returns the opened, restored [`AnyConnection`].
    pub fn import(
        &self,
        handle: &str,
        path: impl AsRef<std::path::Path>,
    ) -> Result<AnyConnection, ProvisionError> {
        let handle = sanitize_handle(handle)?;
        let mut conn = self.open_conn(handle)?;
        crate::backup::import(&mut conn, path)
            .map_err(|e| ProvisionError::Backend(e.to_string()))?;
        Ok(conn)
    }

    /// Remove the tenant for `handle`.
    ///
    /// * SQLite: delete the file `<dir>/<handle>.sqlite`. **No-op on a missing file** (a
    ///   `NotFound` is swallowed) so deprovision is idempotent.
    /// * Postgres: `DROP SCHEMA t_<handle> CASCADE`. Uses `IF EXISTS`, so dropping a missing
    ///   schema is a **no-op** (idempotent).
    pub fn deprovision(&self, handle: &str) -> Result<(), ProvisionError> {
        let handle = sanitize_handle(handle)?;
        match &self.backend {
            Backend::Sqlite { dir } => {
                let path = sqlite_path(dir, handle);
                match std::fs::remove_file(&path) {
                    Ok(()) => Ok(()),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                    Err(e) => Err(ProvisionError::Io(e)),
                }
            }
            #[cfg(feature = "postgres")]
            Backend::Postgres { base_url } => {
                let schema = schema_name(handle);
                let mut conn = PgConnection::establish(base_url)
                    .map_err(|e| ProvisionError::Backend(e.to_string()))?;
                conn.batch_execute(&format!("DROP SCHEMA IF EXISTS {schema} CASCADE;"))
                    .map_err(|e| ProvisionError::Backend(e.to_string()))?;
                Ok(())
            }
        }
    }
}

/// The SQLite file path for a (already-sanitized) handle under `dir`.
fn sqlite_path(dir: &std::path::Path, handle: &str) -> PathBuf {
    dir.join(format!("{handle}.sqlite"))
}

/// The Postgres schema name for a (already-sanitized) handle.
#[cfg(feature = "postgres")]
fn schema_name(handle: &str) -> String {
    format!("t_{handle}")
}

/// Borrow a path as a UTF-8 connection string (paths from a sanitized handle + valid base dir
/// are expected to be UTF-8).
fn path_str(path: &std::path::Path) -> Result<&str, ProvisionError> {
    path.to_str().ok_or_else(|| {
        ProvisionError::Backend(format!("non-UTF-8 tenant path: {}", path.display()))
    })
}
