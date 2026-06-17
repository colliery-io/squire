//! Shared application state wired into every request via axum's [`State`] extractor.
//!
//! At household scale there is a single writer, so the [`Store`] lives behind a plain
//! [`std::sync::Mutex`] and handlers call the sync `Repository`/`Engine` inline (the task's
//! NFR-1.1.3 single-writer model — no async DB pool needed). `AppState` is shared as an
//! `Arc<AppState>` so it is cheap to clone into each handler.
//!
//! [`Store`] borrows interior-mutably ([`std::cell::RefCell`]) and so is not `Sync`; the
//! `Mutex` provides the `Sync` boundary required to share it across axum's worker tasks.

use std::sync::{Arc, Mutex};

use domain_core::contract::HouseholdHandle;
use domain_core::DomainEngine;
use store::tenant::{Backend, ProvisionError, Provisioner};
use store::SystemClock;

use identity::{Identity, ProdIdentity, TokenSigner};

/// The tenant store shared (interior-mutably, single-writer) across the API. Held by both
/// [`AppState`] (the request handlers) and the [`identity::Identity`] impl (the control-plane,
/// which seeds the `users` table on register / add-member). `Arc` so the two sides share one
/// store; `Mutex` because writes are serialized (single writer) and `Store` is not `Sync` on
/// its own. Defined by the `identity` crate (the seam that also owns it) and re-exported here.
pub use identity::SharedStore;

/// Everything a handler needs: the per-tenant store (single-writer, behind a `Mutex`), the
/// pure domain engine, the wall clock, and the identity port.
pub struct AppState {
    /// The tenant-scoped persistence, shared with the identity port (see [`SharedStore`]).
    pub store: SharedStore,
    /// The pure, stateless domain engine (the one validated entry point).
    pub engine: DomainEngine,
    /// Real wall-clock, passed to [`domain_core::contract::Engine::handle`].
    pub clock: SystemClock,
    /// The authentication / membership seam (dev impl now, production in SQUIRE-S-0007).
    pub identity: Arc<dyn identity::Identity>,
}

impl AppState {
    /// Wire the pieces into shared, ref-counted state. The `store` is the same [`SharedStore`]
    /// the `identity` impl was built over, so register / add-member writes land in the store
    /// these handlers read.
    pub fn new(store: SharedStore, identity: Arc<dyn Identity>) -> Arc<Self> {
        Arc::new(Self {
            store,
            engine: DomainEngine,
            clock: SystemClock,
            identity,
        })
    }

    /// Wire a **production** `AppState` for a single local tenant (the LAN-local MVP posture).
    ///
    /// Provisions the tenant for `handle` over `backend` (creating the SQLite file / Postgres
    /// schema and running migrations — required on Postgres, where `open` alone does not create
    /// the schema), opens its store for the request handlers, and builds a [`ProdIdentity`] (real
    /// Argon2id credentials, HMAC tokens signed by `signer`, `token_ttl_ms` lifetime) bound to the
    /// SAME `handle` over the same backend.
    ///
    /// The api services requests **concurrently**, so both the feature handlers and the identity's
    /// control-plane writes (`register` / `add_member`) go through the SAME `Arc<Mutex<Store>>` —
    /// one lock over one connection is the single writer (AR-1). `ProdIdentity::shared_local` is
    /// handed that same [`SharedStore`], so there is no second, uncoordinated connection to race it
    /// (which on SQLite would surface as `SQLITE_BUSY`, and anywhere risks lost updates on `users`).
    pub fn local_prod(
        backend: Backend,
        handle: HouseholdHandle,
        signer: TokenSigner,
        token_ttl_ms: i64,
    ) -> Result<Arc<Self>, ProvisionError> {
        // Create + migrate the tenant once (idempotent — required on Postgres, where `open` alone
        // does not create the schema), then open the ONE store both sides share.
        let provisioner = Provisioner::new(backend);
        provisioner.provision(&handle.0)?;
        let store = provisioner.open(&handle.0, SystemClock)?;
        let store: SharedStore = Arc::new(Mutex::new(store));

        // The production identity writes through that same shared store (single writer).
        let identity = ProdIdentity::shared_local(store.clone(), signer, handle, token_ttl_ms);

        Ok(AppState::new(store, Arc::new(identity)))
    }
}
