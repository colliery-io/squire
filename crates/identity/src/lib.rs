//! The `Identity` port — the API's authentication / membership seam (ADR SQUIRE-A-0004).
//!
//! The API consumes this trait and nothing else for auth: every request is reduced to a
//! verified [`Principal`] before any handler runs. The *production* implementation (real
//! credential hashing, a household-handle → tenant registry, multi-tenant routing) is owned
//! by SQUIRE-S-0007; this task ships only the port plus a minimal in-memory [`DevIdentity`]
//! so the downstream endpoint tasks (T-0015/16/17) can build and test against a stable seam.
//!
//! This crate is deliberately downstream of `store` and `domain-core` and has **no** dependency
//! on `api` (SQUIRE-T-0019): the dependency direction is `api → identity → {store, domain-core}`,
//! so a production identity can be built without a cycle back through the HTTP layer.

pub mod creds;
pub mod tenant;
pub mod token;

pub use token::TokenSigner;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use domain_core::contract::{
    AddMemberReq, AddMemberResp, AuthToken, Change, HouseholdHandle, LoginReq, LoginResp,
    RegisterHouseholdReq, RegisterHouseholdResp, Repository, Role, User, UserId,
};
use store::{Store, SystemClock};

/// The tenant store shared (interior-mutably, single-writer) across the API. Held by both
/// `AppState` (the request handlers) and the [`Identity`] impl (the control-plane, which seeds
/// the `users` table on register / add-member). `Arc` so the two sides share one store; `Mutex`
/// because writes are serialized (single writer) and `Store` is not `Sync` on its own.
pub type SharedStore = Arc<Mutex<Store<SystemClock>>>;

/// A verified caller: the `(household, user, role)` triple every authorized request carries.
/// Produced by [`Identity::verify`] and threaded into handlers by the auth extractor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Principal {
    pub household: HouseholdHandle,
    pub user: UserId,
    pub role: Role,
}

impl Principal {
    /// True when this caller holds the given role. Role gates (the API's `RequireKnight` /
    /// `RequireSquire`) build their 403 decision on this.
    pub fn has_role(&self, role: Role) -> bool {
        self.role == role
    }
}

/// Why authentication / authorization failed. The auth extractor maps these onto HTTP status:
/// [`MissingToken`](AuthError::MissingToken) / [`BadToken`](AuthError::BadToken) /
/// [`WrongTenant`](AuthError::WrongTenant) → **401**; [`Forbidden`](AuthError::Forbidden) → **403**.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthError {
    /// No `Authorization: Bearer` / `X-Household` header (or it was empty).
    MissingToken,
    /// The token was present but is not recognized / not valid.
    BadToken,
    /// The token is valid but for a different household than the one presented.
    WrongTenant,
    /// The caller is authenticated but lacks the required role for this route.
    Forbidden,
}

/// The authentication & membership port the API authorizes every call against.
///
/// `verify` is the hot path used by the auth extractor on every request. The
/// `register`/`login`/`add_member` methods are the control-plane operations whose handlers
/// land in T-0017; their signatures are fixed here so the seam is stable now.
pub trait Identity: Send + Sync {
    /// Resolve `(household, token)` to a verified [`Principal`], or fail with an [`AuthError`].
    fn verify(&self, household: &HouseholdHandle, token: &AuthToken)
        -> Result<Principal, AuthError>;

    /// Create a household (tenant) + seed its first Knight. Body lands in T-0017.
    fn register(&self, req: RegisterHouseholdReq) -> Result<RegisterHouseholdResp, AuthError>;

    /// Exchange a member secret for a tenant-scoped token. Body lands in T-0017.
    fn login(&self, req: LoginReq) -> Result<LoginResp, AuthError>;

    /// Knight-only: add a member (Knight or Squire). Body lands in T-0017.
    fn add_member(
        &self,
        caller: &Principal,
        req: AddMemberReq,
    ) -> Result<AddMemberResp, AuthError>;
}

/// Minimal in-memory [`Identity`] for the LAN-local single-tenant MVP and for tests.
///
/// `verify` looks a [`Principal`] up by token in a seeded map. The control-plane methods
/// (`register` / `login` / `add_member`) manage the household's members by writing the
/// `users` table of the shared tenant [`store`](SharedStore) (via [`Change::PutUser`] through
/// the store's single writer) and keeping their credentials + live tokens in memory.
///
/// **Dev posture (NOT production).** Secrets are kept *plaintext* in [`secrets`](Self::secrets)
/// and tokens are kept in [`tokens`](Self::tokens); ids/tokens come from a simple monotonic
/// [`counter`](Self::counter) (deterministic, no `Date::now`/rand). The production identity —
/// argon2/scrypt secret hashing, persisted credentials, a household-handle → tenant registry,
/// and multi-tenant provisioning via `store::Provisioner` — is owned by **SQUIRE-S-0007**.
/// (MVP single-tenant: there is exactly one store; `register` seeds the first Knight into it.)
pub struct DevIdentity {
    /// The shared tenant store — the same [`SharedStore`] held by the API's `AppState`, so
    /// member rows seeded here are visible to the request handlers.
    store: SharedStore,
    /// Live `token -> Principal` map consulted by `verify`, keyed by the raw token string
    /// (`AuthToken` is not `Hash`/`Eq`). Dev only: in memory, never persisted.
    tokens: Mutex<HashMap<String, Principal>>,
    /// Per-member secrets keyed by `(household, user)` (dev: plaintext, never hashed/persisted).
    secrets: Mutex<HashMap<(HouseholdHandle, UserId), String>>,
    /// Monotonic source for fresh `UserId`s and token strings. Deterministic for dev/tests.
    counter: Mutex<u128>,
}

impl DevIdentity {
    /// A dev identity over the given shared store (no tokens recognized until seeded or minted).
    pub fn new(store: SharedStore) -> Self {
        Self {
            store,
            tokens: Mutex::new(HashMap::new()),
            secrets: Mutex::new(HashMap::new()),
            // Seed the counter above the ids the T-0015/16 tests seed directly (1..=3), so a
            // minted member never collides with a hand-seeded one.
            counter: Mutex::new(1_000),
        }
    }

    /// Seed a `token -> Principal` mapping so `verify` will accept that bearer token. Test
    /// helper (also usable by a dev `main`) for minting a usable token without registration.
    pub fn seed(&self, token: AuthToken, principal: Principal) {
        self.tokens
            .lock()
            .expect("DevIdentity token map poisoned")
            .insert(token.0, principal);
    }

    /// Next value of the monotonic counter (used for both user ids and token uniqueness).
    fn next(&self) -> u128 {
        let mut c = self.counter.lock().expect("DevIdentity counter poisoned");
        *c += 1;
        *c
    }

    /// Apply changes to the shared store under its single writer, stamping `by`.
    fn apply(&self, by: Option<UserId>, changes: &[Change]) {
        self.store
            .lock()
            .expect("store mutex poisoned")
            .apply(by, changes)
            .expect("apply: single-writer store write failed");
    }

    /// Look a user's [`Role`] up from the current store snapshot, if the user exists & is active.
    fn role_of(&self, user: UserId) -> Option<Role> {
        let snap = self.store.lock().expect("store mutex poisoned").snapshot();
        snap.users
            .iter()
            .find(|u| u.id == user && u.active)
            .map(|u| u.role)
    }
}

impl Identity for DevIdentity {
    fn verify(
        &self,
        household: &HouseholdHandle,
        token: &AuthToken,
    ) -> Result<Principal, AuthError> {
        let map = self.tokens.lock().expect("DevIdentity token map poisoned");
        let principal = map.get(&token.0).ok_or(AuthError::BadToken)?;
        // MVP single local tenant: still enforce that the presented handle matches the
        // token's household, so a token can't be replayed against another tenant.
        if &principal.household != household {
            return Err(AuthError::WrongTenant);
        }
        Ok(principal.clone())
    }

    fn register(&self, req: RegisterHouseholdReq) -> Result<RegisterHouseholdResp, AuthError> {
        // Derive a handle from the household name + the counter (sanitized to a stable, opaque
        // slug). MVP single-tenant: there is one store, and registration seeds its first Knight.
        let n = self.next();
        let slug: String = req
            .household_name
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
            .collect();
        let handle = HouseholdHandle(format!("{slug}-{n}"));

        // Mint the first member: a Knight (the admin).
        let admin = UserId(self.next());
        self.apply(
            Some(admin),
            &[Change::PutUser(User {
                id: admin,
                role: Role::Knight,
                display_name: req.admin_name,
                active: true,
            })],
        );

        // Record the admin's secret (dev: plaintext) and mint a live token.
        self.secrets
            .lock()
            .expect("DevIdentity secrets poisoned")
            .insert((handle.clone(), admin), req.admin_secret);
        let token = AuthToken(format!("dev-tok-{}", self.next()));
        self.seed(
            token.clone(),
            Principal { household: handle.clone(), user: admin, role: Role::Knight },
        );

        Ok(RegisterHouseholdResp { household: handle, admin, token })
    }

    fn login(&self, req: LoginReq) -> Result<LoginResp, AuthError> {
        // Verify the member secret (dev: plaintext compare). A missing or mismatched secret is
        // an authentication failure, not a 403.
        {
            let secrets = self.secrets.lock().expect("DevIdentity secrets poisoned");
            match secrets.get(&(req.household.clone(), req.user)) {
                Some(s) if *s == req.secret => {}
                _ => return Err(AuthError::BadToken),
            }
        }

        // Resolve the member's current role from the store, then mint a fresh token.
        let role = self.role_of(req.user).ok_or(AuthError::BadToken)?;
        let token = AuthToken(format!("dev-tok-{}", self.next()));
        self.seed(
            token.clone(),
            Principal { household: req.household, user: req.user, role },
        );
        Ok(LoginResp { token, role })
    }

    fn add_member(
        &self,
        caller: &Principal,
        req: AddMemberReq,
    ) -> Result<AddMemberResp, AuthError> {
        // Knight-only (defense-in-depth: the `RequireKnight` extractor already gates the route).
        if caller.role != Role::Knight {
            return Err(AuthError::Forbidden);
        }

        // Mint a new member of the requested role and seed its row + initial secret.
        let new_id = UserId(self.next());
        self.apply(
            Some(caller.user),
            &[Change::PutUser(User {
                id: new_id,
                role: req.role,
                display_name: req.display_name,
                active: true,
            })],
        );
        self.secrets
            .lock()
            .expect("DevIdentity secrets poisoned")
            .insert((caller.household.clone(), new_id), req.initial_secret);

        Ok(AddMemberResp { user: new_id })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use store::tenant::{Backend, Provisioner};
    use store::SystemClock;

    use super::*;

    /// A `DevIdentity` over a fresh temp-dir SQLite tenant store. Returns the identity and the
    /// `TempDir` (kept alive for the duration of the test).
    fn dev_identity() -> (DevIdentity, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let provisioner = Provisioner::new(Backend::Sqlite { dir: dir.path().to_path_buf() });
        let store = provisioner.open("house1", SystemClock).expect("open tenant store");
        (DevIdentity::new(Arc::new(Mutex::new(store))), dir)
    }

    fn principal(handle: &str) -> Principal {
        Principal {
            household: HouseholdHandle(handle.into()),
            user: UserId(7),
            role: Role::Knight,
        }
    }

    #[test]
    fn verify_accepts_seeded_token() {
        let (id, _dir) = dev_identity();
        id.seed(AuthToken("t".into()), principal("house1"));
        let p = id
            .verify(&HouseholdHandle("house1".into()), &AuthToken("t".into()))
            .expect("seeded token verifies");
        assert_eq!(p.user, UserId(7));
    }

    #[test]
    fn verify_rejects_unknown_token() {
        let (id, _dir) = dev_identity();
        assert_eq!(
            id.verify(&HouseholdHandle("house1".into()), &AuthToken("nope".into())),
            Err(AuthError::BadToken)
        );
    }

    #[test]
    fn verify_rejects_cross_tenant_token() {
        let (id, _dir) = dev_identity();
        id.seed(AuthToken("t".into()), principal("house1"));
        assert_eq!(
            id.verify(&HouseholdHandle("house2".into()), &AuthToken("t".into())),
            Err(AuthError::WrongTenant)
        );
    }

    #[test]
    fn register_then_login_round_trips() {
        let (id, _dir) = dev_identity();
        let resp = id
            .register(RegisterHouseholdReq {
                household_name: "The Round Table".into(),
                admin_name: "Arthur".into(),
                admin_secret: "excalibur".into(),
            })
            .expect("register");
        // The minted admin token verifies as a Knight against the returned handle.
        let p = id.verify(&resp.household, &resp.token).expect("admin token verifies");
        assert_eq!(p.user, resp.admin);
        assert_eq!(p.role, Role::Knight);

        // Login with the admin's secret yields a fresh Knight token.
        let login = id
            .login(LoginReq {
                household: resp.household.clone(),
                user: resp.admin,
                secret: "excalibur".into(),
            })
            .expect("login");
        assert_eq!(login.role, Role::Knight);
        assert!(id.verify(&resp.household, &login.token).is_ok());
    }

    #[test]
    fn login_with_wrong_secret_is_rejected() {
        let (id, _dir) = dev_identity();
        let resp = id
            .register(RegisterHouseholdReq {
                household_name: "House".into(),
                admin_name: "Admin".into(),
                admin_secret: "right".into(),
            })
            .expect("register");
        assert_eq!(
            id.login(LoginReq { household: resp.household, user: resp.admin, secret: "wrong".into() })
                .err(),
            Some(AuthError::BadToken)
        );
    }

    #[test]
    fn add_member_requires_knight() {
        let (id, _dir) = dev_identity();
        let squire_caller = Principal {
            household: HouseholdHandle("house1".into()),
            user: UserId(2),
            role: Role::Squire,
        };
        assert_eq!(
            id.add_member(
                &squire_caller,
                AddMemberReq { role: Role::Squire, display_name: "X".into(), initial_secret: "s".into() }
            )
            .err(),
            Some(AuthError::Forbidden)
        );
    }
}
