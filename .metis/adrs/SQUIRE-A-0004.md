---
id: 001-identity-roles-and-registration
level: adr
title: "Identity, roles and registration: per-user accounts, N Knights + N Squires, tenant-scoped tokens"
number: 1
short_code: "SQUIRE-A-0004"
created_at: 2026-06-17T02:14:14.515167+00:00
updated_at: 2026-06-17T02:17:10.894676+00:00
decision_date: 
decision_maker: 
parent: 
archived: false

tags:
  - "#adr"
  - "#phase/decided"


exit_criteria_met: false
initiative_id: NULL
---

# ADR-1: Identity, roles and registration: per-user accounts, N Knights + N Squires, tenant-scoped tokens

**Status:** Decided (2026-06-16) · **Decision maker:** Dylan Storey
**Relates to:** SQUIRE-A-0002 (tenancy), SQUIRE-A-0003 (Diesel), SQUIRE-S-0003 (Local API), SQUIRE-S-0004 (the Keep), SQUIRE-S-0007 (Identity & Registration, planned). **Revises** vision NFR-5 ("no account system") and the single-child/single-household scope. **Forces follow-on:** SQUIRE-A-0005 (Domain Core → per-Squire subject, not yet decided).

## Context **[REQUIRED]**

We need real security with a **registration path**, and the model must **not assume a single-child / two-parent family** — a household has arbitrary numbers of adults and children. A high-trust, hardcoded MVP is acceptable, but the identity model must be *real* (not a throwaway) and must live **inside each fully-isolated tenant** (SQUIRE-A-0002).

## Decision **[REQUIRED]**

- **Users live inside each tenant schema** — no global user directory (preserves A-0002 isolation). A `User` has: id, role, display name, credential hash, active flag.
- **Two roles: Knight (adult/parent) and Squire (child/player).** A household has **one-or-more of each**; counts are fixed nowhere.
  - **Knight** → full privileged surface: admin authoring on the Keep (SQUIRE-S-0004) and quick-actions on the Knight phone (SQUIRE-S-0006).
  - **Squire** → read + propose **for their own activity** only (SQUIRE-S-0005).
- **Auth: per-user hashed credentials → a tenant-scoped bearer token** carrying `(household handle, user id, role)`. The API authorizes by `(tenant, user, role)`. Tokens are scoped to exactly one tenant; device pairing binds a device to a `(household, user)` and issues such a token. **This supersedes the earlier two shared pairing-secrets design** (child-secret / parent-secret) in SQUIRE-S-0003.
- **Registration path:** `register → create Household (tenant)` provisions an isolated schema (A-0002) and seeds the **first Knight (admin)**. That admin then **adds Knights and Squires** — no assumed counts or family shape. (Invite/onboarding UX can come later.)
- **MVP (high trust):** one household on SQLite; **seed N Knights + N Squires** (example: 2 Knights + 1 Squire) with seeded credentials/tokens via the **real tables and flow** (not bypassed). No external IdP; credential/token verification may be minimal but is present and on the real path.
- **The child trust boundary is preserved and generalized:** a Squire token reaches only read + propose for itself; only Knight tokens unlock privileged operations. Per-user identity additionally yields **per-user audit**.

## Alternatives Analysis **[CONDITIONAL: Complex Decision]**

| Option | Pros | Cons | Risk | Cost |
|--------|------|------|------|------|
| **Per-user accounts + tenant-scoped tokens (chosen)** | Real identity; names *which* Squire (required for >1 child); per-user audit; fits multi-tenant future; high-trust seeding keeps MVP simple | More than MVP strictly needs; **forces the Domain Core to become per-Squire** (A-0005) | Medium | M |
| Per-role shared secrets, tenant-scoped (child-secret / parent-secret) | Lightest; minimal change to current specs | **Cannot distinguish multiple Squires** → violates "don't assume a single child"; no per-user audit | High (req. miss) | S |
| External IdP / OAuth | Offloads auth; strong | Overkill for a local-first MVP; heavier infra; network dependency | Medium | L |

## Rationale **[REQUIRED]**

Per-user accounts are the **only** option consistent with "don't assume a single child": with more than one Squire you must be able to name *which* one a claim, balance, or token belongs to — shared per-role secrets cannot. Accounts also give per-user audit and fit the multi-tenant future. Keeping users inside the tenant schema preserves full isolation (A-0002). High-trust **seeding** (real tables, real flow, fixed credentials) lets the MVP stay simple without building something we throw away.

## Consequences **[REQUIRED]**

### Positive
- Real identity, roles, and a registration path from day one; per-user audit.
- Supports arbitrary household composition (N Knights + N Squires) and the multi-tenant future.
- Generalizes the trust boundary cleanly: authorization is `(tenant, user, role)` instead of two shared secrets.

### Negative
- **Supersedes the per-role pairing-secret design in SQUIRE-S-0003** — that spec must be updated to per-user tenant-scoped tokens.
- **Requires a new Identity, Tenancy & Registration component** (SQUIRE-S-0007).
- **Forces the per-Squire Domain Core revision (SQUIRE-A-0005):** player-specific events and projections must carry a subject (a Squire `UserId`); `StateView`/balance/streaks become per-Squire. A real `shared_contract.rs` revision, tracked separately.
- Vision NFR-5 ("no account system") and the single-child/single-household scope are revised.

### Neutral
- Token format / crypto strength is a later detail (MVP minimal but present); password reset, invites, etc. are out of MVP scope.
- The same human is typically both a Knight (on phone + Keep); role is per-user, and a user has exactly one role in a household.