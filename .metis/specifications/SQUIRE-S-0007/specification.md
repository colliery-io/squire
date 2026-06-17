---
id: identity-tenancy-registration
level: specification
title: "Identity, Tenancy & Registration"
short_code: "SQUIRE-S-0007"
created_at: 2026-06-17T02:24:17.285353+00:00
updated_at: 2026-06-17T02:24:17.285353+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# Identity, Tenancy & Registration

## Overview **[REQUIRED]**

This component is the **registration + auth + tenancy control plane** — the side that *produces* household identity, member credentials, tenant-scoped tokens, and the isolated storage each household lives in. It owns the path from "no household yet" to "a member holding a token that authorizes a specific call", and it provisions the per-tenant schema/file that all other components read and write within.

A **tenant is a Household**, fully isolated at the schema level: its own Postgres schema (hosted) or its own SQLite file (local Keep) — ADR SQUIRE-A-0002. Tenancy never appears in the domain types: a `Snapshot` *is* one household's data, so the Domain Core stays tenant-agnostic. This component is precisely where tenancy is made real — handle → schema/connection routing — so that everything above it can ignore it.

Within a household there are **per-user accounts** (ADR SQUIRE-A-0004): one-or-more Knights (adult/parent) and one-or-more Squires (child/player), with counts fixed nowhere. Each user has a hashed credential stored *inside that household's own schema* — there is no global user directory and no cross-tenant user table, which is what preserves full isolation. Auth is per-user: a member exchanges a secret for a tenant-scoped `AuthToken`, and every downstream call is authorized by the triple **(tenant, user, role)**. Knight = the full privileged surface; Squire = read + propose for itself (ADR SQUIRE-A-0005). This supersedes the PRD's single shared pairing-secret model (NFR-5).

The component produces the registration/auth contract DTOs (`RegisterHouseholdReq/Resp`, `AddMemberReq/Resp`, `LoginReq/Resp`) and the `HouseholdHandle` / `AuthToken` primitives. It also produces the identity-lifecycle `Change`s (`PutUser`, `SetUserActive`) — note these are emitted by *this* component, not by `Engine::handle` (which never manages users), and are persisted through the same single-writer `Repository::apply` as every other `Change`. Provisioning (create + migrate a fresh schema/file) and de-provisioning (drop it) are coordinated with SQUIRE-S-0002, which owns the Repository and the Diesel migrations that double as the provisioning step (ADR SQUIRE-A-0003).

It runs **dual-mode**: in the local single-tenant Keep there is exactly one household, the tenant registry is degenerate/absent, and routing is trivial; in the hosted multi-tenant deployment a thin tenant registry maps a `HouseholdHandle` to its schema/connection. MVP target is LAN-local with high-trust seeding of N Knights + N Squires through the *real* registration/`AddMember` tables (no fixture back-doors).

## System Context **[CONDITIONAL: System-Level Spec]**

### Actors

- **Registrant / admin Knight**: the human who registers a household (`POST /register`) and is seeded as its first Knight (admin). Thereafter a Knight adds further members (`AddMember`) and holds the full privileged surface; a Squire authenticates only to read and propose for itself.
- **Local API (SQUIRE-S-0003), in-process consumer**: calls into this component on every request to verify the presented `AuthToken`, resolve it to (tenant, user, role), authorize the operation, and route to the correct tenant schema/connection before serving state or routing a child submission. It fills a `Command`'s `squire` from the authenticated token rather than trusting the wire.
- **Keep / Admin App (SQUIRE-S-0004), in-process consumer**: drives registration and `AddMember` to seed and administer household members through the real flow, and authenticates the local Knight.

### External Systems

- **Per-tenant DB schema / file**: each household's isolated store — a Postgres schema (hosted) or a SQLite file (local). Created and migrated at provisioning, dropped at de-provisioning. Owns the `User` table (with hashed credentials) alongside the definition tables and event log.
- **Tenant registry (hosted mode only)**: a thin lookup mapping `HouseholdHandle` → schema/connection routing. Holds *no* household domain data and *no* global user directory — only routing metadata — so full isolation is preserved. Degenerate or absent in local single-tenant mode.

### Boundaries

- **Inside**: registration (create household tenant + seed first Knight admin), member management (`AddMember`), per-user credential storage + hashing inside the tenant schema, login/token issuance, token verification, authorization by (tenant, user, role), tenant provisioning (create + migrate) and de-provisioning (drop), the hosted tenant registry + handle→schema routing, and emission of the `PutUser` / `SetUserActive` identity `Change`s.
- **Outside**: all domain rules and validation (`Engine::handle`); per-Squire activity, balance, streaks, and projections (SQUIRE-S-0005 and the Domain Core); the physical storage schema and migration mechanics (owned by SQUIRE-S-0002 — this component invokes them); and transport/serving concerns (owned by the Local API, SQUIRE-S-0003, which merely consumes this component for authn/authz + routing).

## Requirements **[REQUIRED]**

### Functional Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| REQ-1.1 | `RegisterHouseholdReq → RegisterHouseholdResp` creates a Household tenant, provisions its isolated schema/file (create + migrate), seeds its first Knight admin with a hashed credential, and returns the `HouseholdHandle`, the admin's `UserId`, and an `AuthToken`. | A-0002 (schema-per-tenant), A-0003 (migrations provision the schema), A-0004 (registration = create household → seed first Knight). |
| REQ-1.2 | `AddMemberReq → AddMemberResp` lets a Knight add a member of either `Role` (Knight or Squire) with an `initial_secret`, with no assumed counts or family shape, emitting a `PutUser` `Change`. | A-0004 — N Knights and N Squires; membership is open-ended; new members are created via the real `User` table. |
| REQ-1.3 | `LoginReq → LoginResp` exchanges a member secret (under a `HouseholdHandle` + `UserId`) for a tenant-scoped `AuthToken` carrying (household, user, role), and returns the member's `Role`. | A-0004 — auth is per-user; the token is the proof of (tenant, user, role). |
| REQ-1.4 | Verify a presented `AuthToken`: reject if invalid/expired/forged, otherwise resolve it to (tenant, user, role) for the caller. | A-0004 — every call is authorized off a verified token; no account-bypass. |
| REQ-1.5 | Authorize each operation by (tenant, user, role): a Knight gets the full privileged surface; a Squire gets read + propose **for itself only** (cannot act as or read another user). | A-0004 / A-0005 — role-scoped authority; a Squire is confined to its own per-Squire activity. |
| REQ-1.6 | Enforce full per-tenant isolation: all credential storage, user records, and reads/writes occur within the resolved tenant's own schema/file; no operation can reach across tenants and there is no global user directory. | A-0002 — schema-per-tenant FULL isolation; the domain core stays tenant-agnostic. |
| REQ-1.7 | In hosted mode, route a `HouseholdHandle` to its schema/connection via the thin tenant registry (routing metadata only — no household domain data, no global users). In local mode the registry is degenerate/absent and routing is trivial. | A-0002 — dual-mode; hosted needs handle→schema routing while preserving isolation. |
| REQ-1.8 | Provision a fresh tenant by invoking SQUIRE-S-0002's Diesel migrations against a new schema/file; de-provision by dropping that schema/file. | A-0003 — migrations double as per-tenant provisioning; S-0002 owns the migration mechanics. |
| REQ-1.9 | Identity lifecycle `Change`s (`PutUser`, `SetUserActive`) are produced by this component and applied **only** through the single-writer `Repository::apply`, never by `Engine::handle`. | Contract — `Engine::handle` never manages users; single-writer still governs every mutation. |
| REQ-1.10 | For the MVP, seed N Knights + N Squires for a household through the real registration / `AddMember` flow and `User` tables (high-trust, no fixture back-door). | A-0004 — MVP seeds users via the REAL flow so the registration path is exercised end-to-end. |
| REQ-1.11 | When this component (and the API/Keep) applies identity `Change`s, it passes `by` = the authenticated acting user to `Repository::apply(by, changes)`, so the store stamps `created_by`/`updated_by` (+ timestamps) on user rows; `by = None` only for system/seed writes. | A-0007 — authoring writes are audited at the store layer via the actor threaded alongside `apply`. |
| REQ-1.12 | The **users** table carries last-editor audit columns (`created_by`, `created_at`, `updated_by`, `updated_at`); user lifecycle `Change`s (`PutUser`, `SetUserActive`) are audited via `by`, so "who added / deactivated this member (and when)" is answerable. The `User` domain type stays pure — audit is a persistence concern only. | A-0007 — last-editor metadata on definition tables (here, users); domain types do not carry audit fields (AR-7). |

### Non-Functional Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| NFR-2.1 | Full tenant isolation: a household's identity, credentials, and data are physically separated (own schema/file) and unreachable from any other tenant; the registry never holds domain data or a cross-tenant user list. | A-0002 — isolation is a hard architectural guarantee, not a query-time filter. |
| NFR-2.2 | Per-user authentication with no account-bypass: every privileged or per-Squire call requires a verified tenant-scoped token; there is no shared/anonymous path that grants access. | A-0004 — supersedes the shared pairing-secret; integrity (PRD NFR-2) depends on it. |
| NFR-2.3 | Credentials are stored only as hashes (never plaintext); request DTOs carry opaque secrets and hashes are server-internal. | Security — a leaked schema must not expose usable member secrets. |
| NFR-2.4 | MVP runs LAN-local (local HTTP over the home network); no cloud relay is required for the local single-tenant deployment. | A-0002 / PRD NFR-6 — local Keep mode is the MVP target. |
| NFR-2.5 | All user-record mutations go through the single writer (`Repository::apply`), serialized with every other `Change`. | A-0001 / Contract — single-writer is a system-wide invariant covering identity too. |

## Architecture Framing **[CONDITIONAL: System-Level Spec]**

### Decision Area: Token format & crypto
- **Context**: How an `AuthToken` proves (household, user, role) — a self-contained signed bearer token (e.g. signed/MAC'd claims verified without a lookup) vs an opaque token resolved against a server-side session/lookup. Includes expiry/rotation and, for hosted, whether the token must also carry/route the tenant.
- **Constraints**: token is tenant-scoped and carries (tenant, user, role) — ADR A-0004; verification must respect full tenant isolation (A-0002); LAN-local MVP (NFR-2.4); no global user directory to look against in the isolated model.
- **Required Capabilities**: tamper-evident binding of (tenant, user, role), cheap per-request verification, revocation/expiry, and a path that works in both local single-tenant and hosted multi-tenant modes.
- **ADR**: TBD

### Decision Area: Credential hashing scheme
- **Context**: Which password/secret hashing function backs the stored member credential (e.g. argon2 vs bcrypt), with parameters.
- **Constraints**: credentials are stored only as hashes inside the tenant schema (NFR-2.3); secrets arrive as opaque strings in the DTOs.
- **Required Capabilities**: memory-/cost-tunable one-way hashing with per-credential salt, constant-time verification, and a documented upgrade path for parameters.
- **ADR**: TBD

### Decision Area: Tenant-registry storage & routing (hosted)
- **Context**: How the hosted registry stores `HouseholdHandle → schema/connection` and performs routing, while holding no domain data and no global user directory.
- **Constraints**: routing metadata only — full isolation preserved (A-0002, NFR-2.1); degenerate/absent in local single-tenant mode; dual-backend via `diesel-dual-db` (A-0003).
- **Required Capabilities**: fast handle→schema lookup, safe addition/removal of tenants at (de)provisioning, and a degenerate local mode that compiles to a trivial single-tenant route.
- **ADR**: TBD

### Decision Area: Device pairing / binding flow
- **Context**: How a member's phone/device establishes and presents credentials — the pairing/binding step that yields a `HouseholdHandle` + token, and whether a token is bound to a device.
- **Constraints**: per-user auth, no account-bypass (NFR-2.2); supersedes the old shared pairing-secret; LAN-local MVP (NFR-2.4).
- **Required Capabilities**: establish a handle + first token for a device, re-login/re-pair, and optional device binding without introducing a cross-tenant identity store.
- **ADR**: TBD

### Decision Area: How provisioning invokes Diesel migrations (shared with SQUIRE-S-0002)
- **Context**: The mechanism by which provisioning runs the migration set against a fresh schema/file, and de-provisioning drops it — owned jointly with S-0002, which owns the migrations/Repository.
- **Constraints**: migrations double as per-tenant provisioning (A-0003); dual-backend (`diesel-dual-db`); S-0002 owns the migration definitions; must produce a `User` table in-tenant.
- **Required Capabilities**: programmatically create + migrate a new Postgres schema or SQLite file, run idempotently, and drop it cleanly on de-provision — in both backends.
- **ADR**: TBD

## Constraints **[CONDITIONAL: Has Constraints]**

### Technical Constraints
- Diesel dual-backend (`diesel-dual-db`): the same code targets hosted Postgres and local SQLite (A-0003).
- Schema-per-tenant: each household is a distinct Postgres schema / SQLite file, fully isolated (A-0002).
- Users live in-tenant: the `User` table and its hashed credentials reside inside each household's own schema — there is no global/shared user table (A-0002, A-0004).
- No global user directory: the tenant registry holds routing metadata only; cross-tenant user enumeration is impossible by construction.
- LAN-local MVP: the local single-tenant Keep deployment runs over the home LAN with no cloud relay (NFR-2.4).
- Supersedes shared pairing-secrets: the PRD's single shared pairing-secret (NFR-5) is replaced by per-user accounts and tenant-scoped tokens (A-0004).
- Contract types are fixed: `HouseholdHandle`, `AuthToken`, the `Register/AddMember/Login` DTOs, `User`, `Role`, and the `PutUser`/`SetUserActive` `Change`s are defined in `shared_contract.rs` and cannot be altered by this component.
- Single-writer: identity `Change`s are applied only via `Repository::apply` (A-0001), never through `Engine::handle`.
- Audit seam is `apply(by, …)`: the authoring-audit actor enters through the single seam `Repository::apply(by: Option<UserId>, changes)` — the API/Keep supplies `by` = the authenticated acting user, the store stamps `created_by`/`updated_by` (+ timestamps via `Clock`) on user rows, and `by = None` for system/seed writes. Audit lives on table columns, not on the pure `User` type (A-0007).