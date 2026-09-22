//! Integration tests for the production [`ProdIdentity`] (SQUIRE-T-0022).
//!
//! These exercise the real wiring end-to-end against a temp-dir SQLite tenant: hashed credentials
//! stored IN the tenant, tenant-scoped HMAC tokens, registry-routed tenants, and audited member
//! writes through the single writer.

use domain_core::contract::{
    AddMemberReq, AuthToken, HouseholdHandle, LoginReq, RegisterHouseholdReq, Repository, Role,
    UserId,
};

use identity::creds::verify_secret;
use identity::tenant::TenantRegistry;
use identity::token::TokenSigner;
use identity::{AuthError, Identity, Principal, ProdIdentity};

use store::tenant::{Backend, Provisioner};
use store::{user_audit, SystemClock};

/// Build a local single-tenant `ProdIdentity` over a fresh temp-dir SQLite backend, returning the
/// identity, the bound handle, a second [`Provisioner`] over the SAME dir (so tests can re-open the
/// tenant store and assert on rows the identity wrote), and the `TempDir` (kept alive).
fn local_identity(
    ttl_ms: i64,
) -> (
    ProdIdentity,
    HouseholdHandle,
    Provisioner,
    tempfile::TempDir,
) {
    let dir = tempfile::tempdir().expect("tempdir");
    let backend = Backend::Sqlite {
        dir: dir.path().to_path_buf(),
    };
    let handle = HouseholdHandle("house1".into());
    let registry = TenantRegistry::local(backend.clone(), handle.clone());
    let signer = TokenSigner::new(b"test-server-key");
    let id = ProdIdentity::local(registry, signer, handle.clone(), ttl_ms);
    let inspector = Provisioner::new(backend);
    (id, handle, inspector, dir)
}

#[test]
fn register_seeds_knight_and_hashes_credential() {
    let (id, handle, inspector, _dir) = local_identity(60_000);

    let resp = id
        .register(RegisterHouseholdReq {
            household_name: "The Round Table".into(),
            admin_name: "Arthur".into(),
            admin_secret: "excalibur".into(),
        })
        .expect("register");

    assert_eq!(resp.household, handle);

    // verify(handle, token) → the admin Principal (Knight).
    let principal = id
        .verify(&resp.household, &resp.token)
        .expect("admin token verifies");
    assert_eq!(
        principal,
        Principal {
            household: handle.clone(),
            user: resp.admin,
            role: Role::Knight
        }
    );

    // The admin's credential is stored as a HASH (not the plaintext), inside the tenant.
    let store = inspector.open(&handle.0, SystemClock).expect("open tenant");
    let hash = store.credential(resp.admin).expect("credential set");
    assert_ne!(
        hash, "excalibur",
        "secret must be hashed, not stored in plaintext"
    );
    assert!(
        verify_secret("excalibur", &hash),
        "stored hash verifies against the secret"
    );
}

#[test]
fn add_member_then_login_and_audit() {
    let (id, handle, inspector, _dir) = local_identity(60_000);

    let admin = id
        .register(RegisterHouseholdReq {
            household_name: "House".into(),
            admin_name: "Admin".into(),
            admin_secret: "admin-secret".into(),
        })
        .expect("register");
    let caller = id
        .verify(&admin.household, &admin.token)
        .expect("admin principal");

    // Knight caller adds a Squire.
    let added = id
        .add_member(
            &caller,
            AddMemberReq {
                role: Role::Squire,
                display_name: "Squire One".into(),
                initial_secret: "squire-secret".into(),
            },
        )
        .expect("add_member");

    // Login as that squire → token + Role::Squire.
    let login = id
        .login(LoginReq {
            household: handle.clone(),
            user: added.user,
            secret: "squire-secret".into(),
        })
        .expect("login");
    assert_eq!(login.role, Role::Squire);

    // verify → that Squire Principal.
    let principal = id
        .verify(&handle, &login.token)
        .expect("squire token verifies");
    assert_eq!(
        principal,
        Principal {
            household: handle.clone(),
            user: added.user,
            role: Role::Squire
        }
    );

    // The new member's users-row audit (created_by / updated_by) == the caller (the admin Knight).
    let mut store = inspector.open(&handle.0, SystemClock).expect("open tenant");
    let audit = user_audit(&mut store.connection(), added.user)
        .expect("audit query")
        .expect("member row exists");
    assert_eq!(audit.created_by, Some(caller.user), "created_by == caller");
    assert_eq!(audit.updated_by, Some(caller.user), "updated_by == caller");
}

/// Pairing returns the member's display name, so the app can greet them by name ("Hi, <name>!").
#[test]
fn pairing_returns_the_members_display_name() {
    let (id, handle, _inspector, _dir) = local_identity(60_000);
    let admin = id
        .register(RegisterHouseholdReq {
            household_name: "House".into(),
            admin_name: "Admin".into(),
            admin_secret: "admin-secret".into(),
        })
        .expect("register");
    let caller = id
        .verify(&admin.household, &admin.token)
        .expect("admin principal");
    let added = id
        .add_member(
            &caller,
            AddMemberReq {
                role: Role::Squire,
                display_name: "Lancelot".into(),
                initial_secret: "lake".into(),
            },
        )
        .expect("add_member");

    let minted = id
        .mint_pairing_code(&caller, added.user)
        .expect("mint pairing code");
    let paired = id
        .consume_pairing_code(&handle, &minted.code)
        .expect("consume pairing code");

    assert_eq!(paired.user, added.user);
    assert_eq!(paired.role, Role::Squire);
    assert_eq!(
        paired.display_name, "Lancelot",
        "PairResp carries the member's display name"
    );
}

#[test]
fn add_member_with_squire_caller_is_forbidden() {
    let (id, handle, _inspector, _dir) = local_identity(60_000);
    let squire_caller = Principal {
        household: handle,
        user: UserId(2),
        role: Role::Squire,
    };

    assert_eq!(
        id.add_member(
            &squire_caller,
            AddMemberReq {
                role: Role::Squire,
                display_name: "X".into(),
                initial_secret: "s".into(),
            }
        )
        .err(),
        Some(AuthError::Forbidden)
    );
}

#[test]
fn login_wrong_secret_is_bad_token() {
    let (id, handle, _inspector, _dir) = local_identity(60_000);
    let admin = id
        .register(RegisterHouseholdReq {
            household_name: "House".into(),
            admin_name: "Admin".into(),
            admin_secret: "right".into(),
        })
        .expect("register");

    assert_eq!(
        id.login(LoginReq {
            household: handle,
            user: admin.admin,
            secret: "wrong".into()
        })
        .err(),
        Some(AuthError::BadToken)
    );
}

#[test]
fn login_unknown_user_is_bad_token() {
    let (id, handle, _inspector, _dir) = local_identity(60_000);
    id.register(RegisterHouseholdReq {
        household_name: "House".into(),
        admin_name: "Admin".into(),
        admin_secret: "secret".into(),
    })
    .expect("register");

    // No credential for this user id → BadToken.
    assert_eq!(
        id.login(LoginReq {
            household: handle,
            user: UserId(99_999),
            secret: "x".into()
        })
        .err(),
        Some(AuthError::BadToken)
    );
}

#[test]
fn tampered_token_is_bad_token() {
    let (id, handle, _inspector, _dir) = local_identity(60_000);
    let admin = id
        .register(RegisterHouseholdReq {
            household_name: "House".into(),
            admin_name: "Admin".into(),
            admin_secret: "secret".into(),
        })
        .expect("register");

    // Flip a byte in the token payload.
    let mut bytes = admin.token.0.into_bytes();
    bytes[0] ^= 0x01;
    let tampered = AuthToken(String::from_utf8(bytes).expect("ascii"));
    assert_eq!(id.verify(&handle, &tampered), Err(AuthError::BadToken));
}

#[test]
fn expired_token_is_bad_token() {
    // Tiny TTL: by the time `verify` runs (a few millis later) the token's `exp` has passed.
    let (id, handle, _inspector, _dir) = local_identity(0);
    let admin = id
        .register(RegisterHouseholdReq {
            household_name: "House".into(),
            admin_name: "Admin".into(),
            admin_secret: "secret".into(),
        })
        .expect("register");

    // exp = now_ms + 0, and verify uses a now_ms >= issue time, so `exp <= now` → BadToken.
    assert_eq!(id.verify(&handle, &admin.token), Err(AuthError::BadToken));
}

#[test]
fn token_for_other_household_is_wrong_tenant() {
    let (id, _handle, _inspector, _dir) = local_identity(60_000);
    let admin = id
        .register(RegisterHouseholdReq {
            household_name: "House".into(),
            admin_name: "Admin".into(),
            admin_secret: "secret".into(),
        })
        .expect("register");

    // A token minted for household A (the bound handle) presented with handle B → WrongTenant.
    let other = HouseholdHandle("house2".into());
    assert_eq!(id.verify(&other, &admin.token), Err(AuthError::WrongTenant));
}

#[test]
fn hosted_two_households_are_fully_isolated() {
    // The local-posture tests above cover one tenant; this proves the HOSTED multi-tenant flow:
    // two households registered through the real `register`, each provisioning its own tenant, with
    // no cross-tenant reach and no global user directory (NFR-2.1). A `probe` registry over the same
    // backend re-opens each tenant to assert on its rows directly.
    let dir = tempfile::tempdir().expect("tempdir");
    let backend = Backend::Sqlite {
        dir: dir.path().to_path_buf(),
    };
    let id = ProdIdentity::hosted(
        TenantRegistry::hosted(backend.clone()),
        TokenSigner::new(b"test-server-key"),
        60_000,
    );
    let probe = TenantRegistry::hosted(backend);

    let a = id
        .register(RegisterHouseholdReq {
            household_name: "Alpha".into(),
            admin_name: "Alice".into(),
            admin_secret: "asecret".into(),
        })
        .expect("register Alpha");
    let b = id
        .register(RegisterHouseholdReq {
            household_name: "Beta".into(),
            admin_name: "Bob".into(),
            admin_secret: "bsecret".into(),
        })
        .expect("register Beta");

    assert_ne!(
        a.household, b.household,
        "distinct tenants get distinct handles"
    );

    // Each admin token verifies against its OWN household only.
    assert!(
        id.verify(&a.household, &a.token).is_ok(),
        "A's token verifies on A"
    );
    assert_eq!(
        id.verify(&b.household, &a.token),
        Err(AuthError::WrongTenant),
        "A's token must NOT verify against B"
    );

    // No global directory: A's admin cannot log into B (B holds no credential for that id), but can
    // into A.
    assert_eq!(
        id.login(LoginReq {
            household: b.household.clone(),
            user: a.admin,
            secret: "asecret".into()
        })
        .err(),
        Some(AuthError::BadToken),
        "A's admin must not log into B"
    );
    assert!(
        id.login(LoginReq {
            household: a.household.clone(),
            user: a.admin,
            secret: "asecret".into()
        })
        .is_ok(),
        "A's admin logs into A"
    );

    // A member added to A exists ONLY in A's tenant.
    let caller = Principal {
        household: a.household.clone(),
        user: a.admin,
        role: Role::Knight,
    };
    let added = id
        .add_member(
            &caller,
            AddMemberReq {
                role: Role::Squire,
                display_name: "Gareth".into(),
                initial_secret: "g".into(),
            },
        )
        .expect("add member to A");

    probe.register_known(&a.household).expect("know A");
    probe.register_known(&b.household).expect("know B");
    let a_users = probe
        .resolve(&a.household)
        .expect("resolve A")
        .snapshot()
        .users;
    let b_users = probe
        .resolve(&b.household)
        .expect("resolve B")
        .snapshot()
        .users;
    assert!(
        a_users.iter().any(|u| u.id == added.user),
        "A sees its new Squire"
    );
    assert!(
        b_users.iter().all(|u| u.id != added.user),
        "B never sees A's Squire"
    );
    assert_eq!(b_users.len(), 1, "B still has only its own admin");
}

// ─── Device pairing (ADR SQUIRE-A-0010 / SQUIRE-T-0044) ──────────────────────────────────────────

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use sha2::{Digest, Sha256};
use store::SystemClock as StoreClock;

/// Recompute the stored hash of a code the same way `prod.rs` does (so a test can pre-seed a row).
fn code_hash(code: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(code.as_bytes()))
}

/// Register an admin Knight and add one Squire; return (identity, handle, inspector, dir, caller, squire).
fn household_with_squire() -> (
    ProdIdentity,
    HouseholdHandle,
    Provisioner,
    tempfile::TempDir,
    Principal,
    UserId,
) {
    let (id, handle, inspector, dir) = local_identity(60_000);
    let admin = id
        .register(RegisterHouseholdReq {
            household_name: "House".into(),
            admin_name: "Admin".into(),
            admin_secret: "admin-secret".into(),
        })
        .expect("register");
    let caller = id
        .verify(&admin.household, &admin.token)
        .expect("admin principal");
    let squire = id
        .add_member(
            &caller,
            AddMemberReq {
                role: Role::Squire,
                display_name: "Kid".into(),
                initial_secret: "s".into(),
            },
        )
        .expect("add squire")
        .user;
    (id, handle, inspector, dir, caller, squire)
}

#[test]
fn mint_then_consume_round_trips_to_a_member_token() {
    let (id, handle, _inspector, _dir, caller, squire) = household_with_squire();

    let minted = id
        .mint_pairing_code(&caller, squire)
        .expect("knight mints a pairing code for the squire");

    // Consume the code → the squire's tenant-scoped token + identity.
    let paired = id
        .consume_pairing_code(&handle, &minted.code)
        .expect("consume");
    assert_eq!(paired.user, squire);
    assert_eq!(paired.role, Role::Squire);
    assert_eq!(paired.household, handle);

    // The returned token verifies to exactly that Squire Principal.
    let principal = id
        .verify(&handle, &paired.token)
        .expect("paired token verifies");
    assert_eq!(
        principal,
        Principal {
            household: handle,
            user: squire,
            role: Role::Squire
        }
    );
}

#[test]
fn pairing_code_is_single_use() {
    let (id, handle, _inspector, _dir, caller, squire) = household_with_squire();
    let minted = id.mint_pairing_code(&caller, squire).expect("mint");

    assert!(
        id.consume_pairing_code(&handle, &minted.code).is_ok(),
        "first consume works"
    );
    // Second consume of the same code is rejected (the row was deleted on the first).
    assert_eq!(
        id.consume_pairing_code(&handle, &minted.code).unwrap_err(),
        AuthError::BadToken,
        "a used code can't be replayed"
    );
}

#[test]
fn unknown_pairing_code_is_rejected() {
    let (id, handle, _inspector, _dir, _caller, _squire) = household_with_squire();
    assert_eq!(
        id.consume_pairing_code(&handle, "not-a-real-code")
            .unwrap_err(),
        AuthError::BadToken,
    );
}

#[test]
fn non_knight_cannot_mint() {
    let (id, handle, _inspector, _dir, caller, squire) = household_with_squire();
    // Forge a Squire principal and try to mint — rejected before any store write.
    let squire_principal = Principal {
        household: handle,
        user: squire,
        role: Role::Squire,
    };
    assert_eq!(
        id.mint_pairing_code(&squire_principal, caller.user)
            .map(|_| ()),
        Err(AuthError::Forbidden),
    );
}

#[test]
fn expired_pairing_code_is_rejected() {
    let (id, handle, inspector, _dir, _caller, squire) = household_with_squire();
    // Pre-seed an already-expired code directly in the tenant store (expires_at = 1ms past epoch).
    let code = "expired-demo-code";
    inspector
        .open(&handle.0, StoreClock)
        .expect("open tenant")
        .insert_pairing_code(&code_hash(code), squire, Role::Squire, 1)
        .expect("seed expired code");

    assert_eq!(
        id.consume_pairing_code(&handle, code).unwrap_err(),
        AuthError::BadToken,
        "an expired code is rejected (and consumed)"
    );
    // And it's been deleted — a second attempt is still rejected (single-use even when expired).
    assert_eq!(
        id.consume_pairing_code(&handle, code).unwrap_err(),
        AuthError::BadToken
    );
}

// ─── login throttling (SQUIRE-T-0130) ────────────────────────────────────────────────────────

/// After the free allowance of wrong secrets the account locks, and while locked even the CORRECT
/// secret is refused — a lock that a right guess walks through would bound nothing.
#[test]
fn repeated_wrong_secrets_lock_the_account_even_against_the_right_secret() {
    use identity::throttle::FREE_FAILURES;

    let (id, handle, _inspector, _dir) = local_identity(60_000);
    let admin = id
        .register(RegisterHouseholdReq {
            household_name: "The Round Table".into(),
            admin_name: "Arthur".into(),
            admin_secret: "excalibur".into(),
        })
        .expect("register")
        .admin;
    let attempt = |secret: &str| {
        id.login(LoginReq {
            household: handle.clone(),
            user: admin,
            secret: secret.into(),
        })
        .map(|_| ())
    };

    for n in 1..=FREE_FAILURES {
        assert_eq!(
            attempt("wrong"),
            Err(AuthError::BadToken),
            "failure {n} is a plain 401"
        );
    }
    match attempt("excalibur") {
        Err(AuthError::Throttled { retry_after_s }) => {
            assert!(
                (1..=30).contains(&retry_after_s),
                "first lock is ≤30s, got {retry_after_s}"
            )
        }
        other => panic!("the correct secret must NOT beat the lock, got {other:?}"),
    }
}

/// A success inside the allowance clears the streak, so ordinary typos never accumulate to a lock.
#[test]
fn a_successful_login_resets_the_failure_streak() {
    use identity::throttle::FREE_FAILURES;

    let (id, handle, _inspector, _dir) = local_identity(60_000);
    let admin = id
        .register(RegisterHouseholdReq {
            household_name: "The Round Table".into(),
            admin_name: "Arthur".into(),
            admin_secret: "excalibur".into(),
        })
        .expect("register")
        .admin;
    let attempt = |secret: &str| {
        id.login(LoginReq {
            household: handle.clone(),
            user: admin,
            secret: secret.into(),
        })
        .map(|_| ())
    };

    for _round in 0..3 {
        for _ in 0..(FREE_FAILURES - 1) {
            assert_eq!(attempt("wrong"), Err(AuthError::BadToken));
        }
        assert_eq!(attempt("excalibur"), Ok(()), "still inside the allowance");
    }
}

/// Failures are only counted for accounts that have a credential, so made-up user ids can neither
/// lock anything nor grow the throttle's map.
#[test]
fn unknown_users_are_never_throttled() {
    let (id, handle, _inspector, _dir) = local_identity(60_000);
    id.register(RegisterHouseholdReq {
        household_name: "The Round Table".into(),
        admin_name: "Arthur".into(),
        admin_secret: "excalibur".into(),
    })
    .expect("register");

    for _ in 0..25 {
        let got = id
            .login(LoginReq {
                household: handle.clone(),
                user: UserId(999_999),
                secret: "whatever".into(),
            })
            .map(|_| ());
        assert_eq!(got, Err(AuthError::BadToken));
    }
}

// ─── change my secret (SQUIRE-T-0131) ────────────────────────────────────────────────────────

/// Registers Arthur/"excalibur" and returns (identity, handle, Arthur's Principal, tempdir).
fn arthur() -> (ProdIdentity, HouseholdHandle, Principal, tempfile::TempDir) {
    let (id, handle, _inspector, dir) = local_identity(60_000);
    let admin = id
        .register(RegisterHouseholdReq {
            household_name: "The Round Table".into(),
            admin_name: "Arthur".into(),
            admin_secret: "excalibur".into(),
        })
        .expect("register")
        .admin;
    let me = Principal {
        household: handle.clone(),
        user: admin,
        role: Role::Knight,
    };
    (id, handle, me, dir)
}

fn try_login(id: &ProdIdentity, me: &Principal, secret: &str) -> Result<(), AuthError> {
    id.login(LoginReq {
        household: me.household.clone(),
        user: me.user,
        secret: secret.into(),
    })
    .map(|_| ())
}

#[test]
fn change_secret_swaps_the_credential_and_keeps_old_tokens_valid() {
    let (id, handle, me, _dir) = arthur();
    let old_token = id
        .login(LoginReq {
            household: handle.clone(),
            user: me.user,
            secret: "excalibur".into(),
        })
        .expect("login")
        .token;

    assert_eq!(
        id.change_secret(&me, "excalibur", "caliburn-the-second"),
        Ok(())
    );

    assert_eq!(
        try_login(&id, &me, "excalibur"),
        Err(AuthError::BadToken),
        "old secret is dead"
    );
    assert_eq!(
        try_login(&id, &me, "caliburn-the-second"),
        Ok(()),
        "new secret works"
    );
    // Documented limitation, pinned so a change to it is deliberate: no revocation list.
    assert!(
        id.verify(&handle, &old_token).is_ok(),
        "already-issued tokens stay valid"
    );
}

#[test]
fn change_secret_requires_the_current_secret_and_strikes_the_throttle() {
    use identity::throttle::FREE_FAILURES;
    let (id, _handle, me, _dir) = arthur();

    for _ in 0..FREE_FAILURES {
        assert_eq!(
            id.change_secret(&me, "not-my-secret", "a-perfectly-long-secret"),
            Err(AuthError::BadToken)
        );
    }
    // Locked now — for change_secret AND for login: one budget, no side door for guessing.
    assert!(matches!(
        id.change_secret(&me, "excalibur", "a-perfectly-long-secret"),
        Err(AuthError::Throttled { .. })
    ));
    assert!(matches!(
        try_login(&id, &me, "excalibur"),
        Err(AuthError::Throttled { .. })
    ));
}

#[test]
fn a_too_short_new_secret_is_refused_without_costing_a_strike() {
    use identity::throttle::FREE_FAILURES;
    let (id, _handle, me, _dir) = arthur();

    for _ in 0..(FREE_FAILURES * 3) {
        assert_eq!(
            id.change_secret(&me, "excalibur", "short"),
            Err(AuthError::WeakSecret)
        );
        assert_eq!(
            id.change_secret(&me, "excalibur", "  1234567  "),
            Err(AuthError::WeakSecret)
        );
    }
    assert_eq!(
        try_login(&id, &me, "excalibur"),
        Ok(()),
        "unchanged, and never locked"
    );
}
