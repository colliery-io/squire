//! Tenant-registry routing + isolation tests (task SQUIRE-S-0007).
//!
//! Mirrors the store crate's `tenant.rs` gating: SQLite always (a `tempfile::TempDir` base dir);
//! Postgres under `--features postgres` + `DATABASE_URL`, serialized behind a `Mutex` (the tests
//! share one database) and cleaned up after each scenario.

use domain_core::contract::{Change, HouseholdHandle, Repository, Role, User, UserId};

use identity::tenant::{TenantError, TenantRegistry};
use store::tenant::Backend;

// ─── backend harness ───────────────────────────────────────────────────────────

/// Hands `test` a freshly-built [`Backend`] for each available backend, plus the handles to clean
/// up afterwards (Postgres schemas survive process exit; SQLite files live in a `TempDir` that
/// auto-deletes). SQLite always; Postgres only under the feature + `DATABASE_URL`.
#[cfg_attr(not(feature = "postgres"), allow(unused_variables))]
fn each_backend(handles: &[&str], test: impl Fn(Backend)) {
    // SQLite — always. Base dir is a TempDir that drops (and deletes) at scope end.
    {
        let dir = tempfile::tempdir().expect("tempdir");
        test(Backend::Sqlite { dir: dir.path().to_path_buf() });
    }

    // Postgres — opt-in, serialized behind a Mutex (shared database).
    #[cfg(feature = "postgres")]
    {
        use std::sync::Mutex;
        use store::tenant::Provisioner;
        static PG_LOCK: Mutex<()> = Mutex::new(());

        if let Ok(url) = std::env::var("DATABASE_URL") {
            let _guard = PG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            // Pre-clean any leftover schemas from a previous aborted run.
            let cleaner = Provisioner::new(Backend::Postgres { base_url: url.clone() });
            for h in handles {
                cleaner.deprovision(h).expect("pre-clean schema");
            }
            test(Backend::Postgres { base_url: url.clone() });
            // Post-clean.
            for h in handles {
                cleaner.deprovision(h).expect("post-clean schema");
            }
        } else {
            eprintln!("skipping postgres tenant test: DATABASE_URL not set");
        }
    }
}

fn put_user(n: u128, name: &str) -> Vec<Change> {
    vec![Change::PutUser(User {
        id: UserId(n),
        role: Role::Squire,
        display_name: name.to_string(),
        active: true,
    })]
}

// ─── local degenerate single-tenant ──────────────────────────────────────────────

/// A single-tenant registry resolves its OWN handle to a usable store and rejects any other.
#[test]
fn local_resolves_own_handle_and_rejects_others() {
    each_backend(&["house"], |backend| {
        let bound = HouseholdHandle("house".into());
        let reg = TenantRegistry::local(backend, bound.clone());

        // Provision (migrate) the one tenant, then resolve it: the store is usable.
        reg.provision(&bound).expect("provision bound tenant");
        {
            let mut store = reg.resolve(&bound).expect("resolve bound tenant");
            store.apply(None, &put_user(1, "knight")).expect("apply to bound store");
        }
        // Re-resolve and snapshot: the write is visible (it's a real, usable store).
        let snap = reg.resolve(&bound).expect("re-resolve bound").snapshot();
        assert_eq!(snap.users.len(), 1);
        assert_eq!(snap.users[0].display_name, "knight");

        // A DIFFERENT handle is rejected as WrongTenant (no cross-tenant reach).
        let other = HouseholdHandle("other".into());
        match reg.resolve(&other).err() {
            Some(TenantError::WrongTenant { expected, got }) => {
                assert_eq!(expected, bound);
                assert_eq!(got, other);
            }
            o => panic!("expected WrongTenant, got {o:?}"),
        }
    });
}

// ─── hosted multi-tenant isolation ───────────────────────────────────────────────

/// Two provisioned households see ONLY their own data; unknown handle → error; deprovision →
/// resolve fails (NFR-2.1 full isolation, no global directory).
#[test]
fn hosted_isolation_between_two_households() {
    each_backend(&["alpha", "beta"], |backend| {
        let reg = TenantRegistry::hosted(backend);
        let alpha = HouseholdHandle("alpha".into());
        let beta = HouseholdHandle("beta".into());

        // Resolving an un-provisioned handle is an error (not registered).
        match reg.resolve(&alpha).err() {
            Some(TenantError::UnknownTenant(h)) => assert_eq!(h, alpha),
            o => panic!("expected UnknownTenant before provision, got {o:?}"),
        }

        // Provision both, then apply DIFFERENT data through each one's resolved store.
        reg.provision(&alpha).expect("provision alpha");
        reg.provision(&beta).expect("provision beta");
        {
            let mut a = reg.resolve(&alpha).expect("resolve alpha");
            a.apply(None, &put_user(1, "alpha-user")).expect("apply alpha");
            let mut b = reg.resolve(&beta).expect("resolve beta");
            b.apply(None, &put_user(2, "beta-user")).expect("apply beta");
        }

        // Each snapshot sees ONLY its own user (full isolation).
        let a = reg.resolve(&alpha).expect("re-resolve alpha").snapshot();
        let b = reg.resolve(&beta).expect("re-resolve beta").snapshot();
        assert_eq!(a.users.len(), 1, "alpha sees only its own user");
        assert_eq!(a.users[0].id, UserId(1));
        assert_eq!(a.users[0].display_name, "alpha-user");
        assert!(a.users.iter().all(|u| u.id != UserId(2)), "alpha must not see beta");
        assert_eq!(b.users.len(), 1, "beta sees only its own user");
        assert_eq!(b.users[0].id, UserId(2));
        assert_eq!(b.users[0].display_name, "beta-user");
        assert!(b.users.iter().all(|u| u.id != UserId(1)), "beta must not see alpha");

        // Deprovision alpha → forgotten → resolve(alpha) errors; beta is untouched.
        reg.deprovision(&alpha).expect("deprovision alpha");
        match reg.resolve(&alpha).err() {
            Some(TenantError::UnknownTenant(h)) => assert_eq!(h, alpha),
            o => panic!("expected UnknownTenant after deprovision, got {o:?}"),
        }
        assert!(reg.resolve(&beta).is_ok(), "beta still resolvable after alpha deprovisioned");

        // Clean up beta (PG harness also post-cleans, but keep the SQLite path tidy too).
        reg.deprovision(&beta).expect("deprovision beta");
    });
}
