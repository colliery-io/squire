//! The production [`Identity`] (tasks SQUIRE-T-0022 / T-0023).
//!
//! This is the real auth/membership implementation, combining the pieces built in T-0020/T-0021:
//!
//! * **Hashed credentials, stored in the tenant.** Member secrets are Argon2id-hashed
//!   ([`creds::hash_secret`]) and persisted in each tenant's own `credentials` table
//!   (`store::Store::set_credential` / `credential`), never in a global directory and never in
//!   plaintext (REQ-1.6). Credentials are NOT domain `Change`s — they are store rows written
//!   directly, bypassing `apply`.
//! * **Tenant-scoped HMAC tokens.** Tokens are self-contained HMAC-SHA256 bearer tokens
//!   ([`TokenSigner`]) carrying `(household, user, role, exp)`; `verify` recomputes the MAC and
//!   checks expiry without a DB lookup, then confirms the presented handle matches the token's.
//! * **Users via the single writer, with audit.** Member rows are written through
//!   `Store::apply` (the single-writer seam), stamped with the acting caller (`by`) so the
//!   `users` audit columns record who created/updated each member.
//!
//! ## Tenancy: where writes land (and the single-writer invariant under concurrency)
//!
//! The api services requests **concurrently**, so AR-1 (one writer per tenant) must hold against
//! real interleaving, not just sequential calls. A tenant is exactly one connection behind one
//! lock, and BOTH the api's feature handlers and this identity's control-plane writes
//! (`register` / `add_member`) must go through that one lock — otherwise two independent
//! connections race (on SQLite, `SQLITE_BUSY`; everywhere, lost-update hazards on `users`).
//!
//! [`Tenancy`] captures the three deployment shapes:
//!
//! * [`Tenancy::Shared`] — **the production local api.** The identity writes through the SAME
//!   `Arc<Mutex<Store>>` ([`SharedStore`]) the api handlers hold, so every writer (handlers +
//!   identity) serializes on one mutex over one connection. This is what [`ProdIdentity::shared_local`]
//!   builds and what `api::AppState::local_prod` wires.
//! * [`Tenancy::LocalRegistry`] — a single bound tenant routed through a [`TenantRegistry`] that
//!   opens its own connection. For a standalone identity service (no shared api store) and for the
//!   registry-path tests. NOT for the concurrent api — it would be a second writer.
//! * [`Tenancy::Hosted`] — multi-tenant: each `register` derives + provisions a fresh tenant.
//!   (Wiring a *concurrent* hosted api needs a per-tenant `SharedStore` map shared with the
//!   handlers — out of scope for the local MVP; this mode is exercised at the identity layer.)

use std::sync::Mutex;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use password_hash::rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};

use domain_core::contract::{
    AddMemberReq, AddMemberResp, AuthToken, Change, Clock, HouseholdHandle, LoginReq, LoginResp,
    MintPairCodeResp, PairResp, RegisterHouseholdReq, RegisterHouseholdResp, Repository, Role,
    Timestamp, User, UserId,
};
use store::{Store, SystemClock};

/// Device-pairing code lifetime: 30 minutes (ADR SQUIRE-A-0010), in unix millis.
const PAIR_CODE_TTL_MS: i64 = 30 * 60 * 1000;

/// A fresh ≥128-bit single-use pairing code (32 random bytes, base64url-encoded). The plaintext is
/// returned to the Keep once (for the QR); only its [`hash_code`] is persisted.
fn generate_code() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// `sha256(code)` (base64url) — what we store/compare, so the plaintext code never hits the DB.
fn hash_code(code: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(code.as_bytes()))
}

use crate::creds::{hash_secret, verify_secret, MIN_SECRET_LEN};
use crate::tenant::{TenantError, TenantRegistry};
use crate::throttle::LoginThrottle;
use crate::token::TokenSigner;
use crate::{AuthError, Identity, Principal, SharedStore};

/// Where a [`ProdIdentity`]'s reads/writes land. See the module docs for the single-writer rationale.
enum Tenancy {
    /// Local single-tenant over a **shared** store: the api handlers' `Arc<Mutex<Store>>`. Both the
    /// handlers and this identity write through the one lock/connection, so writes serialize under
    /// concurrency (AR-1). `handle` is the one bound household.
    Shared {
        handle: HouseholdHandle,
        store: SharedStore,
    },
    /// Local single-tenant routed through a registry that opens its own connection. `handle` is the
    /// one bound household.
    LocalRegistry {
        handle: HouseholdHandle,
        registry: TenantRegistry,
    },
    /// Hosted multi-tenant: `register` derives a fresh handle and provisions its tenant.
    Hosted { registry: TenantRegistry },
}

/// The production [`Identity`]: hashed in-tenant credentials, tenant-scoped HMAC tokens, audited
/// member writes, over one of the [`Tenancy`] shapes. See the module docs.
pub struct ProdIdentity {
    tenancy: Tenancy,
    signer: TokenSigner,
    clock: SystemClock,
    /// Token lifetime in milliseconds (`exp = now_ms + ttl_ms` at issue).
    token_ttl_ms: i64,
    /// Deterministic monotonic source for fresh `UserId`s and hosted handle suffixes (no rand).
    counter: Mutex<u128>,
    /// Failed-login bookkeeping (SQUIRE-T-0130). `login` holds this lock across check → verify →
    /// record; nothing else takes it, so there is no ordering hazard with the store lock.
    throttle: Mutex<LoginThrottle>,
}

/// The highest user id currently in `store` (the id high-water mark), or 0 if empty / unreadable.
/// Used to seed the `shared_local` id counter so restarts don't reuse ids and clobber members.
fn max_user_id(store: &SharedStore) -> u128 {
    store
        .lock()
        .ok()
        .map(|s| s.snapshot().users.iter().map(|u| u.id.0).max().unwrap_or(0))
        .unwrap_or(0)
}

impl ProdIdentity {
    /// The **production local api** identity: writes through the SAME [`SharedStore`] the api
    /// handlers hold, bound to `handle`, so identity and handler writes serialize on one
    /// lock/connection under concurrency (AR-1). `register` seeds this one household's first Knight.
    pub fn shared_local(
        store: SharedStore,
        signer: TokenSigner,
        handle: HouseholdHandle,
        token_ttl_ms: i64,
    ) -> Self {
        // Seed the id counter from the store's existing high-water mark, NOT 0. The counter is
        // in-memory, so it resets on every process start; seeding from 0 means the first post-restart
        // `add_member` allocates UserId(1) and overwrites an existing member (this corrupted a live
        // household). Seeding from max(existing user id) keeps ids monotonic across restarts. Empty
        // store → 0, so a first-run `register` still bootstraps the admin as UserId(1).
        let seed = max_user_id(&store);
        Self {
            tenancy: Tenancy::Shared { handle, store },
            signer,
            clock: SystemClock,
            token_ttl_ms,
            counter: Mutex::new(seed),
            throttle: Mutex::new(LoginThrottle::default()),
        }
    }

    /// A **local single-tenant** identity bound to `handle`, routed through `registry` (which opens
    /// its OWN connection). For a standalone identity (no shared api store) and registry-path tests;
    /// for the concurrent api prefer [`shared_local`](Self::shared_local).
    pub fn local(
        registry: TenantRegistry,
        signer: TokenSigner,
        handle: HouseholdHandle,
        token_ttl_ms: i64,
    ) -> Self {
        Self {
            tenancy: Tenancy::LocalRegistry { handle, registry },
            signer,
            clock: SystemClock,
            token_ttl_ms,
            counter: Mutex::new(0),
            throttle: Mutex::new(LoginThrottle::default()),
        }
    }

    /// A **hosted multi-tenant** identity over `registry` (which must be in hosted mode). Each
    /// `register` provisions a fresh tenant whose handle is derived from the household name.
    pub fn hosted(registry: TenantRegistry, signer: TokenSigner, token_ttl_ms: i64) -> Self {
        Self {
            tenancy: Tenancy::Hosted { registry },
            signer,
            clock: SystemClock,
            token_ttl_ms,
            counter: Mutex::new(0),
            throttle: Mutex::new(LoginThrottle::default()),
        }
    }

    /// Next value of the deterministic monotonic counter (fresh user ids + hosted handle suffixes).
    /// In `shared_local` the counter is seeded from the store's id high-water mark at construction
    /// (see [`max_user_id`]); `local`/`hosted` still start at 0 (registry/multi-tenant shapes used
    /// by tests + hosting, where the production restart-collision path doesn't apply).
    fn next(&self) -> u128 {
        let mut c = self.counter.lock().expect("ProdIdentity counter poisoned");
        *c += 1;
        *c
    }

    /// Verify `secret` for `(household, user)` under the login throttle (SQUIRE-T-0130), then run
    /// `then` inside the SAME store access with the member's current role. Shared by `login` and
    /// `change_secret` so the security-critical sequence exists once.
    ///
    /// The throttle lock is held across check → verify → record, so a burst of concurrent attempts
    /// cannot all pass the check before the first failure is counted. A locked account is refused
    /// BEFORE the secret is tested — a correct guess must not beat the lock.
    fn with_verified_secret<R>(
        &self,
        household: &HouseholdHandle,
        user: UserId,
        secret: &str,
        then: impl FnOnce(&mut Store<SystemClock>, Role) -> Result<R, AuthError>,
    ) -> Result<R, AuthError> {
        let mut throttle = self.throttle.lock().expect("login throttle poisoned");
        let key = (household.0.clone(), user.0);
        let now_ms = self.now_ms();
        if let Err(retry_after_s) = throttle.check(&key, now_ms) {
            return Err(AuthError::Throttled { retry_after_s });
        }

        let mut wrong_secret = false;
        let mut verified = false;
        let outcome = self.with_tenant_store(household, |store| {
            // A missing credential or a mismatch is an authentication failure (BadToken), never 403.
            let hash = store.credential(user).ok_or(AuthError::BadToken)?;
            if !verify_secret(secret, &hash) {
                // Counted only for accounts that HAVE a credential, so made-up user ids cannot
                // grow the throttle's map (see `throttle` — bounded memory).
                wrong_secret = true;
                return Err(AuthError::BadToken);
            }
            verified = true;
            // Resolve the member's current role; missing / inactive → BadToken.
            let role = store
                .snapshot()
                .users
                .iter()
                .find(|u| u.id == user && u.active)
                .map(|u| u.role)
                .ok_or(AuthError::BadToken)?;
            then(store, role)
        });
        if wrong_secret {
            throttle.record_failure(key, now_ms);
        } else if verified {
            throttle.record_success(&key);
        }
        outcome
    }

    /// Current wall-clock time in unix millis, from the injected clock.
    fn now_ms(&self) -> i64 {
        self.clock.now().0
    }

    /// The one bound household for the single-tenant shapes (`None` for hosted).
    fn bound_handle(&self) -> Option<&HouseholdHandle> {
        match &self.tenancy {
            Tenancy::Shared { handle, .. } | Tenancy::LocalRegistry { handle, .. } => Some(handle),
            Tenancy::Hosted { .. } => None,
        }
    }

    /// Run `f` against the [`Store`] for `handle`, abstracting over the tenancy shape:
    ///
    /// * [`Tenancy::Shared`] — lock the shared mutex (the single writer) and operate on the one
    ///   connection. The whole closure runs under that lock, so a read-then-write inside `f` is
    ///   atomic against any other handler/identity write.
    /// * registry shapes — resolve a tenant-scoped store (its own connection) and operate on it.
    ///
    /// In the single-tenant shapes a `handle` other than the bound one is [`AuthError::WrongTenant`],
    /// so an identity bound to household B never acts on a request routed for household A.
    fn with_tenant_store<R>(
        &self,
        handle: &HouseholdHandle,
        f: impl FnOnce(&mut Store<SystemClock>) -> Result<R, AuthError>,
    ) -> Result<R, AuthError> {
        match &self.tenancy {
            Tenancy::Shared {
                handle: bound,
                store,
            } => {
                if handle != bound {
                    return Err(AuthError::WrongTenant);
                }
                let mut guard = store.lock().map_err(|_| AuthError::BadToken)?;
                f(&mut guard)
            }
            Tenancy::LocalRegistry {
                handle: bound,
                registry,
            } => {
                if handle != bound {
                    return Err(AuthError::WrongTenant);
                }
                let mut store = registry.resolve(handle).map_err(tenant_err_to_auth)?;
                f(&mut store)
            }
            Tenancy::Hosted { registry } => {
                let mut store = registry.resolve(handle).map_err(tenant_err_to_auth)?;
                f(&mut store)
            }
        }
    }
}

/// Map a tenant routing/provision error onto an [`AuthError`]. An unknown / wrong-tenant handle is
/// an authn outcome (`WrongTenant`); any other store/provision failure is treated as a bad-token
/// class failure (the caller cannot proceed) rather than a 403.
fn tenant_err_to_auth(e: TenantError) -> AuthError {
    match e {
        TenantError::WrongTenant { .. } => AuthError::WrongTenant,
        TenantError::UnknownTenant(_) => AuthError::WrongTenant,
        TenantError::Store(_) => AuthError::BadToken,
    }
}

/// Derive a safe tenant handle from a free-form household name plus a unique suffix.
///
/// Lower-cases ASCII letters/digits and maps everything else to `_`, collapses to the
/// `[a-z0-9_]` subset the store's `sanitize_handle` accepts, trims to leave room for the suffix,
/// and appends `_<n>` so repeated names never collide. Falls back to `h` if the name has no usable
/// characters.
fn derive_handle(household_name: &str, n: u128) -> HouseholdHandle {
    let mut slug: String = household_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    // Trim leading/trailing underscores for a tidier handle.
    let trimmed = slug.trim_matches('_');
    slug = if trimmed.is_empty() {
        "h".to_string()
    } else {
        trimmed.to_string()
    };
    // Keep the whole handle within the store's 48-char limit, leaving room for `_<n>`.
    let suffix = format!("_{n}");
    let max_slug = 48usize.saturating_sub(suffix.len());
    slug.truncate(max_slug);
    HouseholdHandle(format!("{slug}{suffix}"))
}

impl Identity for ProdIdentity {
    fn verify(
        &self,
        household: &HouseholdHandle,
        token: &AuthToken,
    ) -> Result<Principal, AuthError> {
        // The signer handles tamper / expiry (→ BadToken). Then enforce the presented handle
        // matches the token's household, so a token can't be replayed against another tenant.
        let principal = self.signer.verify(token, self.now_ms())?;
        if principal.household != *household {
            return Err(AuthError::WrongTenant);
        }
        // Single-tenant: this identity serves EXACTLY one household, and its store is that one
        // tenant's. Reject any other handle outright (defense in depth) so a token valid for a
        // *different* tenant can never resolve here even though its signature checks out — an api
        // bound to household B must never act on a token minted for household A.
        if let Some(bound) = self.bound_handle() {
            if household != bound {
                return Err(AuthError::WrongTenant);
            }
        }
        Ok(principal)
    }

    fn register(&self, req: RegisterHouseholdReq) -> Result<RegisterHouseholdResp, AuthError> {
        // Pick the tenant handle: hosted derives + provisions a fresh one; single-tenant uses its
        // bound household.
        let handle = match &self.tenancy {
            Tenancy::Hosted { registry } => {
                let handle = derive_handle(&req.household_name, self.next());
                registry.provision(&handle).map_err(tenant_err_to_auth)?;
                handle
            }
            _ => self
                .bound_handle()
                .expect("single-tenant has a bound handle")
                .clone(),
        };

        // Mint the first member (a Knight). The existing-Knight guard and the write happen inside
        // ONE `with_tenant_store` closure, so under the shared lock the check-then-seed is atomic:
        // two concurrent registers cannot both pass the guard and seed a second admin.
        let admin = UserId(self.next());
        let admin_name = req.admin_name;
        let admin_hash = hash_secret(&req.admin_secret);
        self.with_tenant_store(&handle, |store| {
            // Refuse re-registration of an already-bootstrapped household (a fresh hosted tenant is
            // empty, so this is a no-op there).
            if store
                .snapshot()
                .users
                .iter()
                .any(|u| u.role == Role::Knight)
            {
                return Err(AuthError::Forbidden);
            }
            // System seed → by = None.
            store
                .apply(
                    None,
                    &[Change::PutUser(User {
                        id: admin,
                        role: Role::Knight,
                        display_name: admin_name,
                        active: true,
                    })],
                )
                .map_err(|_| AuthError::BadToken)?;
            // Persist the admin's hashed secret IN the tenant's credentials table.
            store
                .set_credential(admin, &admin_hash)
                .map_err(|_| AuthError::BadToken)?;
            Ok(())
        })?;

        // Issue a tenant-scoped token for the new admin.
        let principal = Principal {
            household: handle.clone(),
            user: admin,
            role: Role::Knight,
        };
        let token = self
            .signer
            .issue(&principal, self.now_ms(), self.token_ttl_ms);

        Ok(RegisterHouseholdResp {
            household: handle,
            admin,
            token,
        })
    }

    fn login(&self, req: LoginReq) -> Result<LoginResp, AuthError> {
        let role =
            self.with_verified_secret(&req.household, req.user, &req.secret, |_store, role| {
                Ok(role)
            })?;

        let principal = Principal {
            household: req.household,
            user: req.user,
            role,
        };
        let token = self
            .signer
            .issue(&principal, self.now_ms(), self.token_ttl_ms);
        Ok(LoginResp { token, role })
    }

    fn change_secret(
        &self,
        caller: &Principal,
        current_secret: &str,
        new_secret: &str,
    ) -> Result<(), AuthError> {
        // Judged before the current secret is tested, so a too-short new one never costs the
        // caller a throttle strike.
        if new_secret.trim().chars().count() < MIN_SECRET_LEN {
            return Err(AuthError::WeakSecret);
        }
        let new_hash = hash_secret(new_secret);
        // Proving the CURRENT secret (not just holding a token) is the point: tokens are long-lived
        // and sit on paired devices, and a borrowed phone must not be able to take the account over.
        // It goes through the same throttle as login, so this is not a side door for guessing.
        self.with_verified_secret(
            &caller.household,
            caller.user,
            current_secret,
            |store, _role| {
                store
                    .set_credential(caller.user, &new_hash)
                    .map_err(|_| AuthError::BadToken)
            },
        )
    }

    fn add_member(
        &self,
        caller: &Principal,
        req: AddMemberReq,
    ) -> Result<AddMemberResp, AuthError> {
        // Knight-only (defense-in-depth: the API's `RequireKnight` extractor also gates the route).
        if caller.role != Role::Knight {
            return Err(AuthError::Forbidden);
        }

        // Mint a new member of the requested role, audited by the caller, with its hashed secret —
        // both writes under the single lock (Shared), so they serialize with all other writers.
        let new_id = UserId(self.next());
        let display_name = req.display_name;
        let role = req.role;
        let secret_hash = hash_secret(&req.initial_secret);
        self.with_tenant_store(&caller.household, |store| {
            store
                .apply(
                    Some(caller.user),
                    &[Change::PutUser(User {
                        id: new_id,
                        role,
                        display_name,
                        active: true,
                    })],
                )
                .map_err(|_| AuthError::BadToken)?;
            store
                .set_credential(new_id, &secret_hash)
                .map_err(|_| AuthError::BadToken)?;
            Ok(())
        })?;

        Ok(AddMemberResp { user: new_id })
    }

    fn mint_pairing_code(
        &self,
        caller: &Principal,
        target: UserId,
    ) -> Result<MintPairCodeResp, AuthError> {
        // Knight-only (defense in depth: the API route also gates on RequireKnight).
        if caller.role != Role::Knight {
            return Err(AuthError::Forbidden);
        }
        let now = self.now_ms();
        let expires_at = now + PAIR_CODE_TTL_MS;
        let code = generate_code();
        let code_hash = hash_code(&code);

        // Resolve the target member (must exist + be active) and store the code hash under one lock
        // (Shared), so the check-then-write is atomic against other writers.
        self.with_tenant_store(&caller.household, |store| {
            let role = store
                .snapshot()
                .users
                .iter()
                .find(|u| u.id == target && u.active)
                .map(|u| u.role)
                .ok_or(AuthError::Forbidden)?; // unknown / inactive target
            store
                .insert_pairing_code(&code_hash, target, role, expires_at)
                .map_err(|_| AuthError::BadToken)?;
            Ok(())
        })?;

        Ok(MintPairCodeResp {
            code,
            expires_at: Timestamp(expires_at),
        })
    }

    fn consume_pairing_code(
        &self,
        household: &HouseholdHandle,
        code: &str,
    ) -> Result<PairResp, AuthError> {
        let now = self.now_ms();
        let code_hash = hash_code(code);

        // Take (lookup + delete = single-use) and confirm the member is still active, under one
        // lock. An unknown code, an inactive member, and (below) an expired code ALL surface as the
        // same `BadToken` so `/pair` is not an enumeration oracle.
        let (user, role, expires_at, display_name) =
            self.with_tenant_store(household, |store| {
                let taken = store
                    .take_pairing_code(&code_hash)
                    .ok_or(AuthError::BadToken)?;
                let (user, role, exp) = taken;
                // Confirm the member is still active AND grab their display name for the greeting.
                let name = store
                    .snapshot()
                    .users
                    .iter()
                    .find(|u| u.id == user && u.active && u.role == role)
                    .map(|u| u.display_name.clone())
                    .ok_or(AuthError::BadToken)?;
                Ok((user, role, exp, name))
            })?;

        // Expiry is checked after the take, so an expired code is still consumed (deleted) and can't
        // be retried.
        if now >= expires_at {
            return Err(AuthError::BadToken);
        }

        let principal = Principal {
            household: household.clone(),
            user,
            role,
        };
        let token = self.signer.issue(&principal, now, self.token_ttl_ms);
        Ok(PairResp {
            token,
            household: household.clone(),
            user,
            role,
            display_name,
        })
    }
}
