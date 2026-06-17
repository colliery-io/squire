//! Shared application state wired into every request via axum's [`State`] extractor.
//!
//! At household scale there is a single writer, so the [`Store`] lives behind a plain
//! [`std::sync::Mutex`] and handlers call the sync `Repository`/`Engine` inline (the task's
//! NFR-1.1.3 single-writer model — no async DB pool needed). `AppState` is shared as an
//! `Arc<AppState>` so it is cheap to clone into each handler.
//!
//! [`Store`] borrows interior-mutably ([`std::cell::RefCell`]) and so is not `Sync`; the
//! `Mutex` provides the `Sync` boundary required to share it across axum's worker tasks.

use std::sync::Arc;

use domain_core::DomainEngine;
use store::SystemClock;

use identity::Identity;

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
}
