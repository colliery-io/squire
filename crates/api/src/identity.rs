//! The `Identity` port — the API's authentication / membership seam (ADR SQUIRE-A-0004).
//!
//! The API consumes this trait and nothing else for auth: every request is reduced to a
//! verified [`Principal`] before any handler runs. The *production* implementation (real
//! credential hashing, a household-handle → tenant registry, multi-tenant routing) is owned
//! by SQUIRE-S-0007; this task ships only the port plus a minimal in-memory [`DevIdentity`]
//! so the downstream endpoint tasks (T-0015/16/17) can build and test against a stable seam.

use std::collections::HashMap;
use std::sync::Mutex;

use domain_core::contract::{
    AddMemberReq, AddMemberResp, AuthToken, HouseholdHandle, LoginReq, LoginResp,
    RegisterHouseholdReq, RegisterHouseholdResp, Role, UserId,
};

/// A verified caller: the `(household, user, role)` triple every authorized request carries.
/// Produced by [`Identity::verify`] and threaded into handlers by the auth extractor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Principal {
    pub household: HouseholdHandle,
    pub user: UserId,
    pub role: Role,
}

impl Principal {
    /// True when this caller holds the given role. Role gates ([`crate::auth::RequireKnight`] /
    /// [`crate::auth::RequireSquire`]) build their 403 decision on this.
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
/// `verify` looks a [`Principal`] up by token in a seeded map; the control-plane methods are
/// stubs until T-0017 wires `store::Provisioner` and real credential handling. Tests mint
/// tokens with [`DevIdentity::seed`] so T-0015/16 can exercise protected routes.
#[derive(Default)]
pub struct DevIdentity {
    tokens: Mutex<HashMap<String, Principal>>,
}

impl DevIdentity {
    /// An empty dev identity (no tokens recognized yet).
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed a `token -> Principal` mapping so `verify` will accept that bearer token. Test
    /// helper (also usable by a dev `main`) for minting a usable token without registration.
    pub fn seed(&self, token: AuthToken, principal: Principal) {
        self.tokens
            .lock()
            .expect("DevIdentity token map poisoned")
            .insert(token.0, principal);
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

    fn register(&self, _req: RegisterHouseholdReq) -> Result<RegisterHouseholdResp, AuthError> {
        // TODO(T-0017): provision a tenant via `store::Provisioner`, hash the admin secret,
        // seed the first Knight, mint and return a token. Dev stub for now.
        Err(AuthError::Forbidden)
    }

    fn login(&self, _req: LoginReq) -> Result<LoginResp, AuthError> {
        // TODO(T-0017): verify the member secret against the stored hash and mint a token.
        Err(AuthError::BadToken)
    }

    fn add_member(
        &self,
        _caller: &Principal,
        _req: AddMemberReq,
    ) -> Result<AddMemberResp, AuthError> {
        // TODO(T-0017): Knight-only; create the member row + initial secret hash.
        Err(AuthError::Forbidden)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn principal(handle: &str) -> Principal {
        Principal {
            household: HouseholdHandle(handle.into()),
            user: UserId(7),
            role: Role::Knight,
        }
    }

    #[test]
    fn verify_accepts_seeded_token() {
        let id = DevIdentity::new();
        id.seed(AuthToken("t".into()), principal("house1"));
        let p = id
            .verify(&HouseholdHandle("house1".into()), &AuthToken("t".into()))
            .expect("seeded token verifies");
        assert_eq!(p.user, UserId(7));
    }

    #[test]
    fn verify_rejects_unknown_token() {
        let id = DevIdentity::new();
        assert_eq!(
            id.verify(&HouseholdHandle("house1".into()), &AuthToken("nope".into())),
            Err(AuthError::BadToken)
        );
    }

    #[test]
    fn verify_rejects_cross_tenant_token() {
        let id = DevIdentity::new();
        id.seed(AuthToken("t".into()), principal("house1"));
        assert_eq!(
            id.verify(&HouseholdHandle("house2".into()), &AuthToken("t".into())),
            Err(AuthError::WrongTenant)
        );
    }
}
