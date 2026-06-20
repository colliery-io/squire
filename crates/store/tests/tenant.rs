//! Schema-per-tenant provisioning + isolation tests (ADR SQUIRE-A-0002, task SQUIRE-T-0011).
//!
//! Runs on BOTH backends via a parametrized harness: SQLite always (a `tempfile::TempDir` as
//! the base `dir`); Postgres under `--features postgres` + `DATABASE_URL`, serialized behind a
//! `Mutex` (cargo runs test fns concurrently and they share the one database) and cleaned up
//! after each scenario.

use domain_core::contract::{Change, Currency, Event, Repository, Role, Timestamp, User, UserId};

use store::tenant::{Backend, ProvisionError, Provisioner};
use store::SystemClock;

// ─── backend harness ───────────────────────────────────────────────────────────

/// Hands `test` a freshly-built [`Provisioner`] for each available backend, plus a list of
/// handles to clean up afterwards (Postgres schemas survive process exit; SQLite files live in
/// a `TempDir` that auto-deletes). SQLite always; Postgres only under the feature + env.
#[cfg_attr(not(feature = "postgres"), allow(unused_variables))]
fn each_backend(handles: &[&str], test: impl Fn(&Provisioner)) {
    // SQLite — always. Base dir is a TempDir that drops (and deletes) at scope end.
    {
        let dir = tempfile::tempdir().expect("tempdir");
        let prov = Provisioner::new(Backend::Sqlite { dir: dir.path().to_path_buf() });
        test(&prov);
    }

    // Postgres — opt-in, serialized behind a Mutex (shared database).
    #[cfg(feature = "postgres")]
    {
        use std::sync::Mutex;
        static PG_LOCK: Mutex<()> = Mutex::new(());

        if let Ok(url) = std::env::var("DATABASE_URL") {
            let _guard = PG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            let prov = Provisioner::new(Backend::Postgres { base_url: url });
            // Pre-clean any leftover schemas from a previous aborted run.
            for h in handles {
                prov.deprovision(h).expect("pre-clean schema");
            }
            test(&prov);
            // Post-clean.
            for h in handles {
                prov.deprovision(h).expect("post-clean schema");
            }
        } else {
            eprintln!("skipping postgres tenant test: DATABASE_URL not set");
        }
    }
}

// ─── sample data, parametrized per tenant ───────────────────────────────────────

/// A `PutUser` + an `Append` event whose ids/values are derived from `n`, so two tenants seeded
/// with different `n` produce visibly different data.
fn seed_batch(n: u128) -> Vec<Change> {
    vec![
        Change::PutUser(User {
            id: UserId(n),
            role: Role::Squire,
            display_name: format!("user-{n}"),
            active: true,
        }),
        Change::Append(Event::Adjusted {
            command_id: domain_core::contract::CommandId(n),
            squire: UserId(n),
            actor: None,
            currency: Currency::Coins,
            amount: (n as i64) * 10,
            reason: format!("seed-{n}"),
            at: Timestamp(1_000 * n as i64),
        }),
    ]
}

// ─── tests ───────────────────────────────────────────────────────────────────────

/// Core ADR-A-0002 guarantee: two provisioned households see ONLY their own data.
#[test]
fn full_isolation_between_two_households() {
    each_backend(&["alpha", "beta"], |prov| {
        prov.provision("alpha").expect("provision alpha");
        prov.provision("beta").expect("provision beta");

        // Apply DIFFERENT data through each tenant's own store.
        {
            let mut alpha = prov.open("alpha", SystemClock).expect("open alpha");
            alpha.apply(None, &seed_batch(1)).expect("apply alpha");
            let mut beta = prov.open("beta", SystemClock).expect("open beta");
            beta.apply(None, &seed_batch(2)).expect("apply beta");
        }

        // Re-open and snapshot each: each sees only its own user + event.
        let alpha = prov.open("alpha", SystemClock).expect("reopen alpha");
        let beta = prov.open("beta", SystemClock).expect("reopen beta");
        let a = alpha.snapshot();
        let b = beta.snapshot();

        // Alpha has alpha's data, none of beta's.
        assert_eq!(a.users.len(), 1, "alpha should have exactly its own user");
        assert_eq!(a.users[0].id, UserId(1));
        assert_eq!(a.users[0].display_name, "user-1");
        assert_eq!(a.events.len(), 1, "alpha should have exactly its own event");
        assert!(a.users.iter().all(|u| u.id != UserId(2)), "alpha must not see beta's user");

        // Beta has beta's data, none of alpha's.
        assert_eq!(b.users.len(), 1, "beta should have exactly its own user");
        assert_eq!(b.users[0].id, UserId(2));
        assert_eq!(b.users[0].display_name, "user-2");
        assert_eq!(b.events.len(), 1, "beta should have exactly its own event");
        assert!(b.users.iter().all(|u| u.id != UserId(1)), "beta must not see alpha's user");
    });
}

/// provision → usable empty store; deprovision → gone; re-provision → usable again.
#[test]
fn provision_deprovision_lifecycle() {
    each_backend(&["gamma"], |prov| {
        // Provision yields a usable, EMPTY store.
        prov.provision("gamma").expect("provision gamma");
        {
            let store = prov.open("gamma", SystemClock).expect("open gamma");
            let snap = store.snapshot();
            assert!(snap.users.is_empty(), "fresh tenant snapshot should be empty");
            assert!(snap.events.is_empty(), "fresh tenant snapshot should be empty");
        }

        // Write some data so we can prove deprovision removes it.
        {
            let mut store = prov.open("gamma", SystemClock).expect("open gamma");
            store.apply(None, &seed_batch(7)).expect("apply gamma");
            assert_eq!(store.snapshot().users.len(), 1);
        }

        // Deprovision removes the tenant entirely.
        prov.deprovision("gamma").expect("deprovision gamma");
        assert_tenant_absent(prov, "gamma");

        // Deprovision again is a no-op (idempotent).
        prov.deprovision("gamma").expect("deprovision gamma (idempotent)");

        // Re-provision works and starts empty again (old data gone).
        prov.provision("gamma").expect("re-provision gamma");
        let store = prov.open("gamma", SystemClock).expect("reopen gamma");
        let snap = store.snapshot();
        assert!(snap.users.is_empty(), "re-provisioned tenant should be empty");
    });
}

/// Sanitization rejects unsafe handles (uppercase, traversal, SQL/quote chars, empty).
#[test]
fn rejects_unsafe_handles() {
    each_backend(&[], |prov| {
        for bad in ["", "Alpha", "a-b", "a.b", "../etc", "a b", "a'; DROP", "a/b"] {
            let r = prov.provision(bad);
            assert!(
                matches!(r, Err(ProvisionError::InvalidHandle(_))),
                "handle {bad:?} should be rejected, got {r:?}"
            );
        }
        // A valid handle is accepted.
        prov.provision("ok_123").expect("valid handle accepted");
        prov.deprovision("ok_123").expect("cleanup");
    });
}

/// Assert the tenant's backing store is gone (SQLite file deleted / PG schema dropped). For
/// SQLite we re-derive the path; for Postgres we check `information_schema.schemata`.
fn assert_tenant_absent(prov: &Provisioner, handle: &str) {
    match prov.backend() {
        Backend::Sqlite { dir } => {
            let path = dir.join(format!("{handle}.sqlite"));
            assert!(!path.exists(), "SQLite tenant file should be gone: {}", path.display());
        }
        #[cfg(feature = "postgres")]
        Backend::Postgres { base_url } => {
            use diesel::prelude::*;
            let mut conn =
                diesel::pg::PgConnection::establish(base_url).expect("pg connect for assert");
            let schema = format!("t_{handle}");
            let exists: i64 = diesel::sql_query(format!(
                "SELECT count(*) AS n FROM information_schema.schemata WHERE schema_name = '{schema}'"
            ))
            .get_result::<CountRow>(&mut conn)
            .expect("query schemata")
            .n;
            assert_eq!(exists, 0, "Postgres schema {schema} should be dropped");
        }
    }
}

#[cfg(feature = "postgres")]
#[derive(diesel::QueryableByName)]
struct CountRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    n: i64,
}
