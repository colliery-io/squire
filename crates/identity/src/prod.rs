//! The production [`Identity`] (task SQUIRE-T-0022).
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
//! * **Registry-routed tenants.** A [`TenantRegistry`] maps a [`HouseholdHandle`] to its
//!   tenant-scoped [`Store`] (a SQLite file or a Postgres schema), in either local single-tenant
//!   or hosted multi-tenant mode.
//! * **Users via the single writer, with audit.** Member rows are written through
//!   `Store::apply` (the single-writer seam), stamped with the acting caller (`by`) so the
//!   `users` audit columns record who created/updated each member.
//!
//! The dev [`DevIdentity`](crate::DevIdentity) is kept as-is; T-0023 wires `ProdIdentity` into
//! the API.

use std::sync::Mutex;

use domain_core::contract::{
    AddMemberReq, AddMemberResp, AuthToken, Change, Clock, HouseholdHandle, LoginReq, LoginResp,
    RegisterHouseholdReq, RegisterHouseholdResp, Repository, Role, User, UserId,
};
use store::SystemClock;

use crate::creds::{hash_secret, verify_secret};
use crate::tenant::{TenantError, TenantRegistry};
use crate::token::TokenSigner;
use crate::{AuthError, Identity, Principal};

/// The deployment posture of a [`ProdIdentity`] — mirrors the [`TenantRegistry`] mode.
///
/// In **local** mode there is exactly one household, fixed at construction: `register` seeds its
/// first Knight (and refuses if one already exists). In **hosted** mode each `register` derives a
/// fresh handle from the household name and provisions a new tenant.
enum Posture {
    /// Single-tenant: the one bound household handle.
    Local { handle: HouseholdHandle },
    /// Multi-tenant: handles are derived per-registration.
    Hosted,
}

/// The production [`Identity`]: hashed in-tenant credentials, tenant-scoped HMAC tokens,
/// registry-routed tenants, audited member writes. See the module docs.
pub struct ProdIdentity {
    registry: TenantRegistry,
    signer: TokenSigner,
    clock: SystemClock,
    /// Token lifetime in milliseconds (`exp = now_ms + ttl_ms` at issue).
    token_ttl_ms: i64,
    posture: Posture,
    /// Deterministic monotonic source for fresh `UserId`s and hosted handle suffixes (no rand).
    counter: Mutex<u128>,
}

impl ProdIdentity {
    /// A **local single-tenant** identity bound to `handle`, over `registry` (which must itself be
    /// in local mode for `handle`). `signer` signs/verifies tokens; `token_ttl_ms` is their
    /// lifetime.
    pub fn local(
        registry: TenantRegistry,
        signer: TokenSigner,
        handle: HouseholdHandle,
        token_ttl_ms: i64,
    ) -> Self {
        Self {
            registry,
            signer,
            clock: SystemClock,
            token_ttl_ms,
            posture: Posture::Local { handle },
            counter: Mutex::new(0),
        }
    }

    /// A **hosted multi-tenant** identity over `registry` (which must be in hosted mode). Each
    /// `register` provisions a fresh tenant whose handle is derived from the household name.
    pub fn hosted(registry: TenantRegistry, signer: TokenSigner, token_ttl_ms: i64) -> Self {
        Self {
            registry,
            signer,
            clock: SystemClock,
            token_ttl_ms,
            posture: Posture::Hosted,
            counter: Mutex::new(0),
        }
    }

    /// Next value of the deterministic monotonic counter (fresh user ids + hosted handle suffixes).
    fn next(&self) -> u128 {
        let mut c = self.counter.lock().expect("ProdIdentity counter poisoned");
        *c += 1;
        *c
    }

    /// Current wall-clock time in unix millis, from the injected clock.
    fn now_ms(&self) -> i64 {
        self.clock.now().0
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
    slug = if trimmed.is_empty() { "h".to_string() } else { trimmed.to_string() };
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
        Ok(principal)
    }

    fn register(&self, req: RegisterHouseholdReq) -> Result<RegisterHouseholdResp, AuthError> {
        // Pick / provision the tenant handle for this registration.
        let handle = match &self.posture {
            Posture::Local { handle } => {
                // Single-tenant: the one bound household. Refuse if it already has a Knight
                // (already registered) — re-registration is forbidden, not an overwrite.
                let store = self.registry.resolve(handle).map_err(tenant_err_to_auth)?;
                let already_registered = store
                    .snapshot()
                    .users
                    .iter()
                    .any(|u| u.role == Role::Knight);
                if already_registered {
                    return Err(AuthError::Forbidden);
                }
                handle.clone()
            }
            Posture::Hosted => {
                // Multi-tenant: derive a fresh handle and provision its tenant.
                let handle = derive_handle(&req.household_name, self.next());
                self.registry.provision(&handle).map_err(tenant_err_to_auth)?;
                handle
            }
        };

        // Resolve the (now provisioned) tenant store.
        let mut store = self.registry.resolve(&handle).map_err(tenant_err_to_auth)?;

        // Mint the first member: a Knight (the admin). System seed → by = None.
        let admin = UserId(self.next());
        store
            .apply(
                None,
                &[Change::PutUser(User {
                    id: admin,
                    role: Role::Knight,
                    display_name: req.admin_name,
                    active: true,
                })],
            )
            .map_err(|_| AuthError::BadToken)?;

        // Persist the admin's hashed secret IN the tenant's credentials table.
        store
            .set_credential(admin, &hash_secret(&req.admin_secret))
            .map_err(|_| AuthError::BadToken)?;

        // Issue a tenant-scoped token for the new admin.
        let principal = Principal { household: handle.clone(), user: admin, role: Role::Knight };
        let token = self.signer.issue(&principal, self.now_ms(), self.token_ttl_ms);

        Ok(RegisterHouseholdResp { household: handle, admin, token })
    }

    fn login(&self, req: LoginReq) -> Result<LoginResp, AuthError> {
        let store = self.registry.resolve(&req.household).map_err(tenant_err_to_auth)?;

        // Verify the member secret against the stored hash. A missing credential or a mismatch is
        // an authentication failure (BadToken), never a 403.
        let hash = store.credential(req.user).ok_or(AuthError::BadToken)?;
        if !verify_secret(&req.secret, &hash) {
            return Err(AuthError::BadToken);
        }

        // Resolve the member's current role from the store; missing / inactive → BadToken.
        let role = store
            .snapshot()
            .users
            .iter()
            .find(|u| u.id == req.user && u.active)
            .map(|u| u.role)
            .ok_or(AuthError::BadToken)?;

        let principal = Principal { household: req.household, user: req.user, role };
        let token = self.signer.issue(&principal, self.now_ms(), self.token_ttl_ms);
        Ok(LoginResp { token, role })
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

        let mut store = self.registry.resolve(&caller.household).map_err(tenant_err_to_auth)?;

        // Mint a new member of the requested role, audited by the caller.
        let new_id = UserId(self.next());
        store
            .apply(
                Some(caller.user),
                &[Change::PutUser(User {
                    id: new_id,
                    role: req.role,
                    display_name: req.display_name,
                    active: true,
                })],
            )
            .map_err(|_| AuthError::BadToken)?;

        // Persist the new member's hashed initial secret in the tenant.
        store
            .set_credential(new_id, &hash_secret(&req.initial_secret))
            .map_err(|_| AuthError::BadToken)?;

        Ok(AddMemberResp { user: new_id })
    }
}
