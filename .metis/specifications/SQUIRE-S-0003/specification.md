---
id: local-api-trust-boundary
level: specification
title: "Local API & Trust Boundary"
short_code: "SQUIRE-S-0003"
created_at: 2026-06-17T00:52:24.290879+00:00
updated_at: 2026-06-17T00:52:24.290879+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# Local API & Trust Boundary

## Overview **[REQUIRED]**

The Local API is the **single network seam** of Squire and the system's **trust boundary**. It runs in-process on the computer (Rust), composed with the Engine, Repository, and Projections (AR-6, AR-7), and exposes **two token-authenticated, role-scoped surfaces** — never a direct write path. The Keep remains the only writer; the API turns network requests into `Command`s that `Engine::handle` validates and commits. Authentication is **per-user**: every request carries a tenant-scoped bearer token proving `(household, user, role)`, and the API authorizes by that triple (SQUIRE-A-0004). The roles on the wire are **derived from the token**, not from a shared per-role secret.

> **Revision note (v2, post-A-0004/A-0005):** auth moves from **two shared pairing secrets** (a child-secret and a parent-secret) to **per-user tenant-scoped bearer tokens** carrying `(household, user, role)` (SQUIRE-A-0004). Each network role is now **role-from-token**: a **Squire-role** token grants read + propose for its own activity; a **Knight-role** token grants the privileged quick-actions. Token issuance and verification are produced by the Identity & Registration component (SQUIRE-S-0007); **this spec consumes and enforces them** on every call. Reads are also now **per-Squire** (SQUIRE-A-0005): `GET /state` returns the authenticated Squire's `StateView` (carrying `squire`), and the Knight gets a cross-Squire `HouseholdReview`.

> **Revision note (v1, post-PRD):** the original PRD §7.9 / AR-8 made the boundary *structural* — "only read-state + child-submission cross; approve/reject/redeem/adjust are never on the network." To support the parent quick-admin phone (the Knight, SQUIRE-S-0006), AR-8 is **revised** from *"admin ops never on the network"* to *"privileged ops only under a Knight-role token."* The deeper invariants are unchanged: the **Squire role** still cannot mint/approve/redeem (its surface offers no such route), and the **Keep is still the only writer** (AR-1). Authoring (define/archive) stays off the network entirely.

**Squire role** (consumed by the Squire, SQUIRE-S-0005) — read + propose for the authenticated Squire's own activity only:
- `GET /state` → `StateView` — the authenticated **Squire's** view (carries `squire`), built by running `Projections` over a `Snapshot` scoped to that Squire (balance, today's quests, streaks, rewards, that Squire's recent claims/requests), fully cacheable for offline render (FR-API1, FR-PL1).
- `POST /claims` (`SubmitClaimReq` → `SubmitClaimResp`) — turns a completion into a `SubmitClaim` `Command`; the `squire` is filled from the caller's token (the wire DTO omits it) (FR-API2).
- `POST /redemption-requests` (`RequestRedemptionReq` → `RequestRedemptionResp`) — turns a redemption ask into a `RequestRedemption` `Command`; `squire` filled from the token (FR-API3).

**Knight role** (consumed by the Knight, SQUIRE-S-0006) — privileged quick-actions, gated by a **Knight-role token**. A role-scoped surface (e.g. `POST /admin/*`) carrying the parent quick-action `Command`s from `shared_contract.rs`: `ReviewClaim{Approve/Reject}`, `ReviewRedemption{Approve/Reject}`, `RedeemItem` (direct, carrying the target `squire`), `AdjustPoints` (add-funds, carrying the target `squire`), and "mark-done" (a minted `claim_id` claim that is immediately approved, reusing the claim path). It also serves the Knight a cross-Squire **`HouseholdReview`** read — pending claims/requests across all Squires plus per-Squire balances. **Define/Archive are NOT here** — authoring lives only on the Keep (SQUIRE-S-0004).

**Control-plane endpoints** (co-owned with SQUIRE-S-0007, exposed by this API): `POST /register` (`RegisterHouseholdReq` → `RegisterHouseholdResp` — create household + provision schema + seed first Knight), `POST /login` (`LoginReq` → `LoginResp` — member secret → tenant-scoped token), and a Knight-only add-member (`AddMemberReq` → `AddMemberResp`). The API exposes these; SQUIRE-S-0007 owns the identity/token machinery and the tenant registry behind them.

The API owns `StateView` and all its nested view types (`QuestCard`/`QuestStatus`, `StreakView`, `RewardCard`/`LockReason`, `ClaimStatus`/`ClaimState`, `RedemptionStatus`/`RedemptionState`) and `HouseholdReview` (with `SquireSummary`/`PendingClaim`/`PendingRequest`) as the produced wire contract, plus the parent quick-action request/response envelopes (incl. the idempotency-id extension below); the identity/registration DTOs (`HouseholdHandle`, `AuthToken`, `RegisterHouseholdReq/Resp`, `LoginReq/Resp`, `AddMemberReq/Resp`) are co-owned with SQUIRE-S-0007. It does not own business rules (Engine), persistence (Repository), or token issuance/verification and tenant routing (SQUIRE-S-0007). It is the producing/server side of both surfaces; the two phone clients are the consuming sides over the LAN.

## System Context **[CONDITIONAL: System-Level Spec]**

### Actors
- **Squire (per-user, via the Squire phone)**: a network caller holding a **Squire-role** tenant-scoped token. Reads its *own* `StateView` and submits completion claims and redemption requests over the LAN; `squire` is taken from the token. Never an admin actor; cannot reach any mutation beyond the two proposals, and cannot read another Squire's state. A household may have one-or-more Squires (SQUIRE-A-0004).
- **Knight (per-user, via the Knight phone)**: a network caller holding a **Knight-role** tenant-scoped token. Reads the cross-Squire `HouseholdReview` and issues privileged quick-action commands (review/redeem/adjust/mark-done), each naming a target `squire` where applicable. Cannot author (define/archive) — that is not on the wire. A household may have one-or-more Knights; the first is seeded at registration.

### External Systems
- **The Squire — child phone client (consumer)**: fetches its own `StateView`, posts claims/redemption-requests with phone-minted ids under its Squire token, retries from its offline outbox (SQUIRE-S-0005).
- **The Knight — parent phone client (consumer)**: reads `HouseholdReview`, posts privileged quick-action commands with client-minted idempotency ids under its Knight token, retries from its offline outbox (SQUIRE-S-0006).
- **Identity & Registration (SQUIRE-S-0007) — authn / tenant-routing collaborator**: issues and verifies the tenant-scoped tokens, owns the registration/login/add-member flow and the tenant registry (`HouseholdHandle` → schema, SQUIRE-A-0002). The API resolves the tenant and authorizes `(tenant, user, role)` *through* this component, then enforces the result; it produces tokens, this spec consumes them.
- **Engine (in-process collaborator)**: the single validated door (`Engine::handle`). The API translates *both* Squire submissions and Knight quick-actions into `Command`s (filling `squire` from the token / the quick-action's target) and hands them to the Engine; all rules live there, not here.
- **Repository (in-process collaborator)**: supplies the tenant-scoped `Snapshot` the API reads for per-Squire `StateView` / `HouseholdReview` and that the Engine reasons over; applies the `Change`s an accepted command produces (single writer, AR-1).

### Boundaries
**Inside:** both role surfaces — the Squire endpoints (`GET /state`, `POST /claims`, `POST /redemption-requests`) and the Knight quick-action surface (`POST /admin/*`: review claim, review redemption, direct redeem, adjust/add-funds, mark-done) plus the `HouseholdReview` read; the control-plane endpoints exposed here (`POST /register`, `POST /login`, Knight-only add-member) that delegate to SQUIRE-S-0007; DTO serialization of `StateView`/`HouseholdReview`, their nested views, and the request/response + idempotency + identity envelopes; idempotency on phone-minted `claim_id`/`request_id` **and** on client-minted parent command ids; **token-based per-user authentication on every request — resolve the tenant from the `HouseholdHandle`, then authorize `(tenant, user, role)` — and filling `squire` from the token**; binding/discoverability on the LAN.

**Outside:** authoring — define/archive of quests/items/achievements — which is **never routable** and stays local to the Keep (SQUIRE-S-0004); token issuance/verification, credential hashing, and the tenant registry/routing internals (SQUIRE-S-0007); the business rules and projections logic (Engine/Projections); persistence and the event log (Repository); affordability checking for redemptions (recorded only, re-checked at commit by the Engine); each phone client's cache and outbox behavior.

## Requirements **[REQUIRED]**

### Functional Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| REQ-1.1.1 | `GET /state` returns the **authenticated Squire's** `StateView` (carrying `squire`), built by running `Projections` over a `Snapshot` filtered to that Squire (balance, `quests_today`, `streaks`, `rewards`, `my_claims`, `my_requests`). The `squire` is resolved from the token; a Squire never reads another Squire's state. A `QuestCard` may carry `QuestStatus::TakenByOther` for a `Race` quest a sibling has already claimed/won. Each `RewardCard` carries `last_redeemed: Option<Timestamp>` (derived: the most recent `ItemRedeemed` for the item) and, when locked, a `LockReason` that is `NeedsAchievement` or `OutOfStock` only; availability is `Once`/`Repeatable`, and a `Once` item reads `OutOfStock` once redeemed (SQUIRE-A-0006). | FR-API1, FR-PL1, SQUIRE-A-0005 — the phone renders entirely from this one per-Squire payload. |
| REQ-1.1.2 | `POST /claims` accepts `SubmitClaimReq` (which omits `squire`), fills `squire` from the caller's token, issues a `SubmitClaim` command via `Engine::handle`, and returns `SubmitClaimResp` carrying `ClaimState`: `Pending`, or `Approved { points }` when the quest's `auto_approve` is true. | FR-API2, FR-C3, SQUIRE-A-0005 — the response reflects immediate auto-approval so the phone shows credited points without a parent round-trip. |
| REQ-1.1.3 | `POST /claims` is idempotent on the phone-minted `claim_id`: a resubmit of an already-seen id is a no-op that re-returns the current `ClaimState`, never appending a second `CompletionClaimed`. | FR-API2, FR-SY3, AC-3 — the offline outbox may retry freely. |
| REQ-1.1.4 | `POST /redemption-requests` accepts `RequestRedemptionReq` (which omits `squire`), fills `squire` from the token, issues `RequestRedemption` via `Engine::handle`, and returns `RequestRedemptionResp` with `RedemptionState::Pending`. Affordability is **not** checked here — the request is only recorded. | FR-API3, FR-R5 — no points are reserved; affordability is re-validated at Knight approval (FR-R4). |
| REQ-1.1.5 | `POST /redemption-requests` is idempotent on the phone-minted `request_id`: a resubmit re-returns current state and never appends a duplicate `RedemptionRequested`. | FR-API3, FR-SY3. |
| REQ-1.2.1 | The Squire role exposes exactly the three endpoints above and nothing privileged. No claim-approval, redemption, or adjustment is reachable in the Squire role; auto-approval is an engine rule, not a Squire-callable operation. | FR-API4, NFR-2 — the Squire surface cannot move the balance. |
| REQ-1.2.2 | Authoring — define/archive of quests, items, achievements — is **not routable in any network role**. It exists only as local commands on the Keep. | AR-8 (retained for authoring), FR-ADM4 — keep the privileged surface minimal. |
| REQ-1.2.3 | Every request carries a **tenant-scoped bearer token** proving `(household, user, role)`; the API **resolves the tenant first** (from the `HouseholdHandle`), then **authorizes `(tenant, user, role)`** before any state read or command. A Squire-role token can never reach the Knight surface, and the role is taken from the verified token, not from the route. Token issuance/verification is owned by SQUIRE-S-0007; this spec enforces it. | SQUIRE-A-0004 (supersedes per-role pairing secrets), SQUIRE-A-0002 (tenant routing) — role + tenant separation is enforced by token, not by shared secret. |
| REQ-1.2.4 | The API resolves the tenant from the client-presented `HouseholdHandle` **before** authorizing the user within it; hosted mode routes to that schema via the registry (owned by SQUIRE-S-0007), local mode is a single degenerate tenant (no routing). | SQUIRE-A-0002 — tenant is selected at the connection boundary; the pure core never sees a tenant id. |

**Knight role** (the Knight, SQUIRE-S-0006) — privileged quick-actions:

| ID | Requirement | Rationale |
|----|-------------|-----------|
| REQ-1.3.1 | The Knight quick-action surface (`POST /admin/*`) accepts and dispatches, via `Engine::handle`: `ReviewClaim{Approve/Reject}`, `ReviewRedemption{Approve/Reject}`, `RedeemItem` (direct, carrying the target `squire`), `AdjustPoints` (add-funds, carrying the target `squire`, with required reason), and "mark-done" (mint a `claim_id`, submit + immediately approve). On each privileged command the API fills the `actor` from the authenticated **Knight's token** — it is **never client-supplied**, so a client cannot forge who acted. | §7.3/§7.6/§7.4 exposed as Knight quick-actions; new (parent quick-admin). The Keep still performs every write; SQUIRE-A-0005 — targeted commands name their `squire` and carry a token-derived `actor`. |
| REQ-1.3.2 | The Knight role serves the cross-Squire **`HouseholdReview`** read — `pending_claims` + `pending_requests` across all Squires (each labeled with its `squire`) + per-Squire balances (`SquireSummary`) — to render the Knight, cacheable for offline display. | FR-ADM1 over the network, SQUIRE-A-0005; new (parent quick-admin). |
| REQ-1.3.3 | Every Knight quick-action carries a client-minted idempotency id; a replay of the same id is a no-op that re-returns the prior outcome and never double-applies (no double-approve, double-redeem, or double-credit). Reviews dedupe on `claim_id`/`request_id` (engine rejects double-review); direct redeem/adjust dedupe on the `command_id` carried onto their events. | NFR-3 for privileged commands; supports the Knight's offline outbox. Mechanism fixed by ADR SQUIRE-A-0001 (`CommandId`). |
| REQ-1.3.4 | `AdjustPoints` requires a non-empty reason; a request without one is rejected. | FR-P2 — adjustments are always explained. |

**Control plane** (endpoints exposed here, machinery owned by SQUIRE-S-0007):

| ID | Requirement | Rationale |
|----|-------------|-----------|
| REQ-1.4.1 | `POST /register` accepts `RegisterHouseholdReq`, delegates to SQUIRE-S-0007 to create the Household (tenant) + provision an isolated schema (SQUIRE-A-0002) + seed the first Knight (admin), and returns `RegisterHouseholdResp` (`HouseholdHandle`, admin `UserId`, admin `AuthToken`). Unauthenticated (it bootstraps the tenant). | SQUIRE-A-0004 registration path, SQUIRE-A-0002 provisioning — the only way a household and its first token come into existence. |
| REQ-1.4.2 | `POST /login` accepts `LoginReq` (`HouseholdHandle`, `UserId`, member secret), delegates verification to SQUIRE-S-0007, and returns `LoginResp` (a tenant-scoped `AuthToken` + the member's `Role`). The API resolves the tenant from the handle before verifying the member within it. | SQUIRE-A-0004 (member secret → token), SQUIRE-A-0002 (tenant resolved first). |
| REQ-1.4.3 | A **Knight-only** add-member endpoint accepts `AddMemberReq` (role, display name, initial secret), delegates to SQUIRE-S-0007 to create the member (Knight or Squire) within the caller's tenant, and returns `AddMemberResp` (the new `UserId`). Requires a Knight-role token; no assumed household composition. | SQUIRE-A-0004 — the seeded admin adds arbitrary N Knights + N Squires. |

### Non-Functional Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| NFR-1.1.1 | **Integrity (Squire role structural; Knight role credentialed).** The Squire surface offers no way to mint points, approve, or redeem — a Squire can only propose for itself, which carries zero balance impact until committed. Privileged operations exist only under a Knight-role token, and even then the **Keep is the sole writer** — the network never writes the store, it submits commands to the one validated door. | NFR-2 (preserved), AR-1 — the authoritative balance lives only on the computer, derived from the log. |
| NFR-1.1.2 | **Idempotency (both roles).** Duplicate Squire submissions (same `claim_id`/`request_id`) and duplicate Knight quick-actions (same client-minted command id) are no-ops; the server recognizes a repeat and re-returns the prior outcome without duplicating events or effects. | NFR-3, FR-SY3 — both outboxes may retry freely. |
| NFR-1.1.3 | **Per-user, tenant-scoped token access control.** Every request presents a bearer token proving `(household, user, role)`, issued by SQUIRE-S-0007 and verified on every request; the API resolves the tenant from the `HouseholdHandle`, then authorizes `(tenant, user, role)`. A Knight-role token unlocks the privileged surface; a Squire-role token does not and is confined to its own activity. This supersedes the earlier two shared pairing-secrets (child/parent) design. | SQUIRE-A-0004 (per-user tokens, revises NFR-5), SQUIRE-A-0002 (tenant scoping) — real per-user authorization, enforced here, produced by SQUIRE-S-0007. |
| NFR-1.1.4 | **LAN network scope.** The API is local HTTP, bound and discoverable on the home LAN. The computer's address may change; the server must be bindable/discoverable, while the phone tolerates "not found" via its cache + outbox. (MVP is LAN-local single-tenant; hosted multi-tenant routing is the same model, degenerate locally.) | NFR-6 — no cloud relay; home-network only. |

## Architecture Framing **[CONDITIONAL: System-Level Spec]**

### Decision Area: HTTP framework / transport
- **Context**: Which Rust HTTP server stack hosts the endpoints in-process with the Engine/Repository.
- **Constraints**: Local HTTP only (NFR-6); in-process composition (AR-6); serves the fixed DTOs from `shared_contract.rs`.
- **Required Capabilities**: Route the Squire/Knight/control-plane endpoints (no others); (de)serialize the DTOs; inject the token-verification + tenant-resolution middleware and the shared Engine/Repository handle.
- **ADR**: TBD

### Decision Area: LAN discovery
- **Context**: How the phone locates the computer's address when it may change (manual `host:port` entry vs mDNS/zeroconf). PRD §10 open item.
- **Constraints**: LAN-only, no cloud relay (NFR-6); the phone must degrade gracefully to cache + outbox when not found.
- **Required Capabilities**: The server is bindable and (if mDNS) advertisable on the LAN; a stable way for the phone to resolve/re-resolve the address.
- **ADR**: TBD

### Decision Area: Per-user token authentication & tenant resolution
- **Context**: How a request proves `(household, user, role)` and is routed to the right tenant. A paired device presents a `HouseholdHandle` + a tenant-scoped bearer token; the API resolves the tenant (hosted: registry lookup → schema; local: single degenerate tenant) **before** authorizing the user within it. Token issuance/verification and the registry are owned by SQUIRE-S-0007 — this spec defines how the API **enforces** them on every call.
- **Constraints**: Per-user tokens carrying `(household, user, role)`, no shared per-role secret (SQUIRE-A-0004, supersedes NFR-5); schema-per-tenant isolation, tenant selected at the connection boundary (SQUIRE-A-0002); checked on every request before any read or command; a Squire-role token must never authorize the Knight surface; `squire` is filled from the token for child submissions.
- **Required Capabilities**: Present the handle + token per request (e.g. headers); resolve/verify the tenant from the handle; verify the token and extract `(user, role)`; authorize the route by role; reject missing/invalid/wrong-tenant/wrong-role tokens. The producing machinery lives in SQUIRE-S-0007; this area covers the enforcement seam.
- **ADR**: SQUIRE-A-0004 (decided), SQUIRE-A-0002 (decided); enforcement details shared with SQUIRE-S-0007 / SQUIRE-S-0006

### Decision Area: Control-plane endpoint exposure (register / login / add-member)
- **Context**: The API exposes `POST /register`, `POST /login`, and the Knight-only add-member endpoint, but the identity machinery (credential hashing, token minting/verification, tenant provisioning, the registry) lives in SQUIRE-S-0007. This area is the **division of labor and the request handling** at the API seam: which calls are unauthenticated (register bootstraps a tenant), which resolve the tenant first then verify a member (login), and which require an existing Knight token (add-member).
- **Constraints**: DTOs fixed by `shared_contract.rs` (`RegisterHouseholdReq/Resp`, `LoginReq/Resp`, `AddMemberReq/Resp`, `HouseholdHandle`, `AuthToken`); secrets are opaque in requests, hashes never cross the wire; provisioning + seeding the first Knight is delegated (SQUIRE-A-0004 / A-0002); add-member is Knight-only within the caller's tenant.
- **Required Capabilities**: Route + (de)serialize the three control-plane DTOs; delegate to SQUIRE-S-0007 for create-tenant/provision/seed, verify-member→token, and create-member; enforce the auth posture per endpoint (register unauthenticated, login tenant-resolved, add-member Knight-token).
- **ADR**: TBD (co-owned with SQUIRE-S-0007; grounded in SQUIRE-A-0004 / SQUIRE-A-0002)

### Decision Area: Privileged-command idempotency & contract extension — RESOLVED
- **Context**: Child submissions and parent reviews dedupe naturally via the event log (`claim_id`/`request_id`). Direct `RedeemItem` and `AdjustPoints` had **no client id** — a retry from the Knight's outbox would double-redeem / double-credit.
- **Decision**: A client-minted `CommandId` is carried **onto the emitted event** (`RedeemItem`/`AdjustPoints` gain `command_id`; `ItemRedeemed.command_id: Option<CommandId>`, `PointsAdjusted.command_id: CommandId`). The server dedupes by finding an event already carrying that id and re-returns current state — dedup stays *derived from the append-only log*, no processed-commands side table. Rejected: a command envelope + side table (breaks AR-3). **Applied additively to `shared_contract.rs`.**
- **ADR**: SQUIRE-A-0001 (decided)

### Decision Area: Knight cross-Squire review read shape (`HouseholdReview`)
- **Context**: The Knight reads a Knight-scoped endpoint returning the cross-Squire `HouseholdReview` — `pending_claims` + `pending_requests` across all Squires (each labeled with its `squire`) + per-Squire balances (`SquireSummary`). The contract gives a provisional shape; this area refines exactly what the Knight needs to triage and act per-Squire.
- **Constraints**: Cacheable for offline render; Knight-role authenticated within the resolved tenant; reuses `Projections` over a `Snapshot`, filtering/grouping by `squire` (SQUIRE-A-0005); must not leak credentials or another tenant's data.
- **Required Capabilities**: Aggregate pending work across all Squires with per-item `squire` labels; per-Squire balances; surface enough to triage and act without over-sharing.
- **ADR**: TBD (shape refined with SQUIRE-S-0006; per-Squire basis fixed by SQUIRE-A-0005)

## Constraints **[CONDITIONAL: Has Constraints]**

### Technical Constraints
- Two role surfaces on the wire: Squire (read + propose for its own activity) and Knight (privileged quick-actions), plus the control-plane endpoints (register/login/add-member). Nothing writes the store directly — every request becomes a `Command` the Engine validates and the Keep commits (single-writer, AR-1).
- **Authoring (define/archive) is never routable in any role** — it stays local to the Keep (AR-8 retained for authoring, FR-ADM4).
- The Squire surface cannot reach any privileged operation; the Knight surface is reachable only under a Knight-role token (AR-8 revised → credentialed, not absent).
- Access is gated by **per-user tenant-scoped bearer tokens** carrying `(household, user, role)`, issued/verified by SQUIRE-S-0007 and enforced here on every request; the API **resolves the tenant from the `HouseholdHandle` first, then authorizes `(tenant, user, role)`**. This supersedes the earlier two shared pairing secrets (SQUIRE-A-0004, revising NFR-5; SQUIRE-A-0002 for tenant routing).
- Tenancy is schema-per-tenant and resolved at the connection boundary; hosted mode routes via the registry (owned by SQUIRE-S-0007), local mode is a single degenerate tenant (SQUIRE-A-0002). The pure core never sees a tenant id.
- Reads and submissions are **per-Squire** (SQUIRE-A-0005): `GET /state` returns the authenticated Squire's `StateView` (carries `squire`); child submissions get `squire` filled from the token (wire DTOs omit it); Knight `RedeemItem`/`AdjustPoints` carry the target `squire`; the Knight reads a cross-Squire `HouseholdReview`.
- LAN-only local HTTP; no cloud relay; the computer's address may change and the server must be bindable/discoverable on the LAN, for both phones (NFR-6). MVP is LAN-local single-tenant.
- Every privileged Knight command is idempotent on a client-minted id; `shared_contract.rs` carries `CommandId` on `RedeemItem`/`AdjustPoints` and their emitted events (ADR SQUIRE-A-0001).
- The wire DTOs are fixed by `shared_contract.rs` (`StateView`/`HouseholdReview` and their nested views, `SubmitClaimReq/Resp`, `RequestRedemptionReq/Resp`, and the identity/registration DTOs `RegisterHouseholdReq/Resp`, `LoginReq/Resp`, `AddMemberReq/Resp`, `HouseholdHandle`, `AuthToken`); the Knight quick-action envelopes extend the contract additively. The API produces these but does not redefine the domain types or own token issuance.
- Credentials never cross this boundary in the clear: requests carry opaque member secrets/tokens only; hashing and token minting are server-internal to SQUIRE-S-0007.
- Affordability for redemptions is not checked at request time — only recorded; it is re-checked at commit by the Engine (FR-R5, FR-R4).
- Implemented in Rust, in-process with the Engine, Repository, and Projections (AR-6, AR-7).