---
id: concurrent-hosted-multi-tenant-api
level: task
title: "Concurrent hosted multi-tenant API: per-tenant shared store (single writer per tenant)"
short_code: "SQUIRE-T-0024"
created_at: 2026-06-17T10:57:50.777176+00:00
updated_at: 2026-06-17T10:57:50.777176+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
---

# Concurrent hosted multi-tenant API: per-tenant shared store (single writer per tenant)

Spec: [[SQUIRE-S-0007]] (Identity, Tenancy & Registration). Deferred from [[SQUIRE-T-0023]].

## Objective

Make the API able to serve **multiple tenants from one process under concurrent requests** while
preserving the single-writer-per-tenant invariant (AR-1). Today the API is single-tenant: one
`AppState.store` (`Arc<Mutex<Store>>`) and a `ProdIdentity::shared_local` bound to that one store.
T-0023's hosted `ProdIdentity` exists and is isolation-tested at the identity layer, but it routes
each call through a registry that **opens its own connection per tenant** — fine for a standalone
identity, but it would be a *second, uncoordinated writer* if dropped behind a concurrent
multi-tenant API. This task builds the per-tenant shared-store routing so handlers and identity
share exactly one `Arc<Mutex<Store>>` **per tenant**.

## Backlog Item Details

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P3 - Low (when time permits) — not needed for the LAN-local single-tenant MVP; required before
  a hosted multi-tenant deployment.

### Business Justification
- **User Value**: enables a hosted offering (many households on shared infrastructure) without
  spinning a process per tenant.
- **Business Value**: the multi-tenant path the project explicitly wants to keep open (schema/file
  isolation already exists; this is the *concurrent serving* layer on top).
- **Effort Estimate**: M.

## Acceptance Criteria

- [ ] A tenant-routing layer maps a verified household handle → that tenant's **single** shared
  `Arc<Mutex<Store>>`, created once and reused (a per-tenant store cache/registry), NOT a fresh
  connection per request.
- [ ] Both the feature handlers and `ProdIdentity` (hosted) resolve a request to the SAME per-tenant
  shared store, so all writes for a tenant serialize on one lock/connection (AR-1 holds per tenant).
- [ ] The API resolves the tenant from the verified `Principal`/`X-Household` (auth still first), and
  request handling for tenant A never touches tenant B's store (structural isolation preserved).
- [ ] Registration provisions a new tenant AND makes it immediately routable (store entry created)
  without racing concurrent first-requests to that tenant.
- [ ] Lifecycle: a sensible bound on cached open stores (idle eviction / LRU or explicit close), and
  correct behaviour after deprovision (route gone, store closed). Document the chosen policy.
- [ ] Concurrency tests: concurrent requests **across multiple tenants** (and within each) all
  succeed, apply exactly once, with no cross-tenant leakage and no SQLite busy / lost-update errors.
- [ ] Dual-backend: passes on SQLite (file-per-tenant) and the compose Postgres (schema-per-tenant).
- [ ] `cargo test --workspace` green and warning-free.

## Implementation Notes

### Technical Approach
- Introduce a per-tenant store registry in the API layer, e.g. `Arc<Mutex<HashMap<HouseholdHandle,
  SharedStore>>>` (or a purpose-built `TenantStores` type), that lazily opens-and-caches one
  `SharedStore` per handle via `store::Provisioner::open` and hands the SAME `SharedStore` to both
  the handler path and the identity path.
- Generalize `AppState` so the per-request store is **resolved by handle** rather than a single fixed
  `store` field. The squire/knight/control handlers take the resolved `SharedStore` for the request's
  tenant.
- Add a `Tenancy` variant to `ProdIdentity` (e.g. `SharedHosted`) that, instead of
  `registry.resolve` opening a fresh connection, looks the handle up in the SAME per-tenant store
  registry the handlers use. The existing `Shared` (single local tenant) and registry-based
  `LocalRegistry`/`Hosted` variants stay for their current uses.
- Keep auth/tenant resolution ordering: verify token → derive handle/principal → resolve tenant store.
- Reuse the existing `store::tenant::Provisioner` + `identity::tenant::TenantRegistry` for
  provision/migrate/isolation; this task adds the *shared, cached, concurrent* routing on top.

### Dependencies
- [[SQUIRE-T-0023]] (ProdIdentity, `Tenancy` enum, `shared_local`, dual-backend harness).
- `store::Provisioner` / `identity::tenant::TenantRegistry` (schema/file-per-tenant isolation).

### Risk Considerations
- **Single-writer per tenant** is the core invariant — the per-tenant store MUST be a shared single
  instance, never re-opened per request (the exact bug T-0023's follow-up fixed, generalized to N
  tenants).
- **Connection/store lifecycle** under many tenants (fd/connection limits) — needs an eviction policy.
- **Provision↔first-request race** — ensure a freshly registered tenant is routable atomically.
- **`u128` ids as JSON numbers** (noted separately) is orthogonal but will surface in any wire-client
  work that rides on this.

## Status Updates

**2026-06-17 — Backlogged.** Shape captured at the moment we identified it (during T-0023's
concurrency follow-up). Not started; deferred — the single-tenant local MVP does not need it.