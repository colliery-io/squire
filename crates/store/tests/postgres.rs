//! Dual-backend proof: the portable migrations apply on **Postgres**, not just SQLite
//! (ADR SQUIRE-A-0003, NFR-2.5/2.6). Compiled only with `--features postgres` (links libpq)
//! and run against the docker-compose `postgres` service:
//!
//!   docker compose up -d postgres
//!   DATABASE_URL=postgres://squire:squire@localhost:55432/squire_test \
//!     PQ_LIB_DIR=$(pg_config --libdir) \
//!     DYLD_FALLBACK_LIBRARY_PATH=$(pg_config --libdir) \
//!     cargo test -p store --features postgres
//!
//! Skipped (not failed) when `DATABASE_URL` is unset, so it never blocks an offline run.
#![cfg(feature = "postgres")]

#[test]
fn migrations_apply_on_postgres() {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping postgres test: DATABASE_URL not set");
        return;
    };
    store::pg::provision_clean(&url)
        .expect("the portable migrations should apply cleanly on Postgres");
}
