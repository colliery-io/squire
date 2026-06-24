//! Tenant registry + provisioning / routing (task SQUIRE-S-0007).
//!
//! This is the household-handle → tenant *routing* layer that the low-level
//! [`store::tenant`] module deliberately leaves out of scope. It wraps a
//! [`store::tenant::Provisioner`] (which owns the SQLite-file / Postgres-schema isolation and,
//! via SQUIRE-S-0002, the migrations) and adds the decision "is THIS handle a tenant I know
//! about, and which store does it route to?".
//!
//! It does **NOT** reimplement migrations: every create/migrate goes through
//! [`Provisioner::provision`], every open through [`Provisioner::open`], every drop through
//! [`Provisioner::deprovision`]. The store layer remains the single owner of the schema.
//!
//! ## Two modes (decision: tenant-registry storage & routing)
//!
//! * **Local single-tenant (degenerate).** One fixed [`HouseholdHandle`] bound to one backend.
//!   Routing is trivial: [`resolve`](TenantRegistry::resolve) returns a store iff the handle
//!   *equals* the bound one, else [`TenantError::WrongTenant`]. The registry holds no set — the
//!   single bound handle IS the routing table. This is the LAN-local MVP posture.
//!
//! * **Hosted multi-tenant.** A thin in-memory set of *known* handles
//!   (`HouseholdHandle → ()`) — routing metadata ONLY. It holds **no** household domain data and
//!   **no** user list (NFR-2.1: no global user directory, no cross-tenant reach). To resolve, the
//!   registry checks the handle is in the known set, then asks the [`Provisioner`] to `open` a
//!   tenant-scoped [`Store`]. Persisting the known-set itself is a hosted-ops concern; an
//!   in-memory map is fine for the MVP (a fresh process re-learns tenants as they are
//!   (re-)provisioned, or from a future ops-owned catalog).
//!
//! ## Isolation invariant
//!
//! A resolved [`Store`] is already tenant-scoped by the [`Provisioner`] (a SQLite file for the
//! handle, or a Postgres connection pinned to the handle's schema). The registry never opens more
//! than one tenant at a time and never threads a `tenant_id` discriminator — isolation is
//! structural, inherited wholesale from the store layer.

use std::collections::HashSet;
use std::sync::Mutex;

use domain_core::contract::HouseholdHandle;
use store::tenant::{Backend, ProvisionError, Provisioner};
use store::{Store, SystemClock};

/// A failure resolving / provisioning / deprovisioning a tenant.
///
/// Wraps the store layer's [`ProvisionError`] (invalid handle, IO, backend/migration failure)
/// and adds the two routing-only outcomes the registry itself decides.
#[derive(Debug)]
pub enum TenantError {
    /// The handle is not a known/provisioned tenant (hosted: not in the known set; local: never
    /// happens — a non-matching handle is [`WrongTenant`](TenantError::WrongTenant)).
    UnknownTenant(HouseholdHandle),
    /// The handle does not match this single-tenant registry's bound household (local mode only).
    WrongTenant {
        /// The handle this registry is bound to.
        expected: HouseholdHandle,
        /// The handle the caller asked to resolve.
        got: HouseholdHandle,
    },
    /// The underlying store provision/open/deprovision failed (invalid handle, IO, migration…).
    Store(ProvisionError),
}

impl std::fmt::Display for TenantError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TenantError::UnknownTenant(h) => {
                write!(
                    f,
                    "unknown tenant {:?}: not provisioned / not registered",
                    h.0
                )
            }
            TenantError::WrongTenant { expected, got } => write!(
                f,
                "wrong tenant: this registry serves {:?}, not {:?}",
                expected.0, got.0
            ),
            TenantError::Store(e) => write!(f, "tenant store error: {e}"),
        }
    }
}

impl std::error::Error for TenantError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TenantError::Store(e) => Some(e),
            _ => None,
        }
    }
}

impl From<ProvisionError> for TenantError {
    fn from(e: ProvisionError) -> Self {
        TenantError::Store(e)
    }
}

/// Routing mode — see the module docs.
enum Mode {
    /// Local single-tenant: exactly one bound handle, no registry set.
    Local { handle: HouseholdHandle },
    /// Hosted multi-tenant: an in-memory set of known (routing-only) handles.
    Hosted {
        known: Mutex<HashSet<HouseholdHandle>>,
    },
}

/// Maps a [`HouseholdHandle`] to a tenant-scoped [`Store`], over a [`store::tenant::Backend`].
///
/// Construct with [`local`](Self::local) (degenerate single-tenant) or [`hosted`](Self::hosted)
/// (multi-tenant). All create/migrate/open/drop is delegated to an inner [`Provisioner`].
pub struct TenantRegistry {
    provisioner: Provisioner,
    mode: Mode,
}

impl TenantRegistry {
    /// A **local single-tenant** registry: one fixed `handle` bound to `backend`. `resolve`
    /// returns a store only for that handle; any other handle is [`TenantError::WrongTenant`].
    ///
    /// The bound tenant is provisioned lazily on first [`resolve`](Self::resolve)/[`provision`]
    /// (the [`Provisioner`] runs migrations on open), so a fresh deployment is usable immediately.
    pub fn local(backend: Backend, handle: HouseholdHandle) -> Self {
        Self {
            provisioner: Provisioner::new(backend),
            mode: Mode::Local { handle },
        }
    }

    /// A **hosted multi-tenant** registry over `backend`, starting with an empty known-set.
    /// Tenants become resolvable after [`provision`](Self::provision) (or
    /// [`register_known`](Self::register_known) for an already-provisioned backing store).
    pub fn hosted(backend: Backend) -> Self {
        Self {
            provisioner: Provisioner::new(backend),
            mode: Mode::Hosted {
                known: Mutex::new(HashSet::new()),
            },
        }
    }

    /// Borrow the inner [`Provisioner`] (advanced / introspection use).
    pub fn provisioner(&self) -> &Provisioner {
        &self.provisioner
    }

    /// Record `handle` as a known tenant WITHOUT (re)creating its store. Hosted-only routing
    /// metadata — use when the backing store already exists (e.g. learned from an ops catalog).
    ///
    /// * Hosted: inserts into the known set.
    /// * Local: a no-op (the single bound handle is fixed at construction); registering the
    ///   bound handle is accepted, a different one is rejected as [`TenantError::WrongTenant`].
    pub fn register_known(&self, handle: &HouseholdHandle) -> Result<(), TenantError> {
        match &self.mode {
            Mode::Local { handle: bound } => {
                if bound == handle {
                    Ok(())
                } else {
                    Err(TenantError::WrongTenant {
                        expected: bound.clone(),
                        got: handle.clone(),
                    })
                }
            }
            Mode::Hosted { known } => {
                known
                    .lock()
                    .expect("tenant registry poisoned")
                    .insert(handle.clone());
                Ok(())
            }
        }
    }

    /// Create + migrate the tenant for `handle` (via [`Provisioner::provision`]) and record it as
    /// known (hosted). Idempotent: re-provisioning a tenant re-runs migrations harmlessly and the
    /// known-set insert is a set operation.
    ///
    /// In local mode `handle` MUST equal the bound household, else [`TenantError::WrongTenant`].
    pub fn provision(&self, handle: &HouseholdHandle) -> Result<(), TenantError> {
        if let Mode::Local { handle: bound } = &self.mode {
            if bound != handle {
                return Err(TenantError::WrongTenant {
                    expected: bound.clone(),
                    got: handle.clone(),
                });
            }
        }
        self.provisioner.provision(&handle.0)?;
        self.register_known(handle)?;
        Ok(())
    }

    /// Drop the tenant for `handle` (via [`Provisioner::deprovision`]) and forget it (hosted).
    /// Idempotent (deprovisioning a missing backing store is a no-op in the store layer).
    ///
    /// In local mode `handle` MUST equal the bound household, else [`TenantError::WrongTenant`].
    pub fn deprovision(&self, handle: &HouseholdHandle) -> Result<(), TenantError> {
        match &self.mode {
            Mode::Local { handle: bound } => {
                if bound != handle {
                    return Err(TenantError::WrongTenant {
                        expected: bound.clone(),
                        got: handle.clone(),
                    });
                }
                self.provisioner.deprovision(&handle.0)?;
            }
            Mode::Hosted { known } => {
                self.provisioner.deprovision(&handle.0)?;
                known
                    .lock()
                    .expect("tenant registry poisoned")
                    .remove(handle);
            }
        }
        Ok(())
    }

    /// Open a tenant-scoped [`Store`] for `handle`, rejecting unknown / cross-tenant handles.
    ///
    /// * Local: returns the one store iff `handle` equals the bound household, else
    ///   [`TenantError::WrongTenant`].
    /// * Hosted: returns a store iff `handle` is in the known set, else
    ///   [`TenantError::UnknownTenant`]; the store is opened fresh via [`Provisioner::open`]
    ///   (pinned to the handle's SQLite file / Postgres schema).
    ///
    /// The returned [`Store`] carries a [`SystemClock`].
    pub fn resolve(&self, handle: &HouseholdHandle) -> Result<Store<SystemClock>, TenantError> {
        match &self.mode {
            Mode::Local { handle: bound } => {
                if bound != handle {
                    return Err(TenantError::WrongTenant {
                        expected: bound.clone(),
                        got: handle.clone(),
                    });
                }
            }
            Mode::Hosted { known } => {
                let known = known.lock().expect("tenant registry poisoned");
                if !known.contains(handle) {
                    return Err(TenantError::UnknownTenant(handle.clone()));
                }
            }
        }
        let store = self.provisioner.open(&handle.0, SystemClock)?;
        Ok(store)
    }
}
