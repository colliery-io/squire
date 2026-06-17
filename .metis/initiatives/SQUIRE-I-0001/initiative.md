---
id: squire-v1
level: initiative
title: "Squire v1"
short_code: "SQUIRE-I-0001"
created_at: 2026-06-16T23:15:22.252544+00:00
updated_at: 2026-06-17T03:01:43.153654+00:00
parent: SQUIRE-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/decompose"


exit_criteria_met: false
estimated_complexity: L
initiative_id: squire-v1
---

# Squire v1 Initiative

This initiative delivers Squire v1 end-to-end. It is the container for the per-component specifications; each spec defines the interface(s) on its **producing** side. Source of truth: `squire_prd.md` (PRD) and `shared_contract.rs` (authoritative types).

## Context **[REQUIRED]**

Squire gamifies a single child's household chores while keeping all point/reward authority on the parent's computer (see vision SQUIRE-V-0001). The system is the Keep (computer — single source of truth and only writer) plus two offline-capable phone clients: the Squire (child — read + propose) and the Knight (parent — privileged quick-actions under a separate credential). Integrity rests on single-writer commit through one validated door, not on hiding operations from the network. The domain model, events, commands, ports, and API DTOs are fixed in `shared_contract.rs`; this initiative implements behavior against that contract. The foundational decisions are now decided: schema-per-tenant full isolation with dual-mode local-SQLite-single-tenant / hosted-Postgres-multi-tenant and a tenant-agnostic domain (A-0002); Diesel dual-backend persistence, resolving rusqlite/sqlx (A-0003); per-user accounts with roles Knight/Squire (N of each), tenant-scoped tokens, and registration = create household → add Knights & Squires, superseding shared pairing-secrets (A-0004); and a per-Squire domain/StateView (A-0005). Two additive contract extensions have been applied to `shared_contract.rs`: a `CommandId` idempotency key on the `RedeemItem`/`AdjustPoints` commands and their events (A-0001), and identity (`UserId`/`Role`/`User`) plus a per-Squire subject on every event (A-0005).

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- Implement the pure domain core (`handle` + projections) against the shared contract, with property tests for its invariants.
- Persist definitions + an append-only event log durably via **Diesel dual-backend** (SQLite local / Postgres hosted, same code), single-writer, with a full-store export (A-0003).
- Provision **multi-tenant-capable** storage — schema-per-tenant full isolation, dual-mode single local tenant for the MVP (A-0002) — with **per-user accounts and a registration path** (create household → add Knights & Squires) (A-0004).
- Serve the LAN-only API with **per-user tenant-scoped tokens** carrying a role: Squire (read + submit, moves nothing on its own) and Knight (privileged quick-actions), superseding shared pairing secrets (A-0004).
- Ship the admin app (the Keep): authoring, a batched review queue, direct redeem, manual adjustment.
- Ship the child phone app (the Squire): StateView rendering, offline cache, idempotent outbox, sync.
- Ship the parent phone app (the Knight): quick-actions only — approve/reject claims, mark-done, approve/reject + direct redeem, add funds — over an idempotent offline outbox.
- Pass all eight PRD acceptance criteria (AC-1 … AC-8) end-to-end.

**Non-Goals (v1):**
- Play Store publishing; multi-child / multi-household; off-network/cloud sync; photo evidence; streak freezes/grace days (see PRD §3, §11).

## Requirements **[CONDITIONAL: Requirements-Heavy Initiative]**

Full requirements live in `squire_prd.md` and are decomposed into the component specs below. Traceability summary:

| PRD area | Owning component spec |
| --- | --- |
| §6 domain model, §7.2 due-logic, §7.4 ledger, §7.5 streaks/achievements, AR-3/4/5/7 | Domain Core |
| Tenancy/isolation (A-0002), per-user accounts + roles + registration (A-0004), tenant-scoped tokens, per-Squire subject (A-0005) | Identity, Tenancy & Registration (SQUIRE-S-0007) |
| AR-1/2, NFR-4 durability/export, NFR-8 perf | Persistence & Store |
| §7.9 API, AR-8 trust boundary (revised → two authenticated roles), §7.8 idempotency, NFR-5 pairing (per-role secrets) | Local API & Trust Boundary |
| §7.1 authoring, §7.3 review, §7.6 redemption, §7.10 admin | Admin App (the Keep) |
| §7.7 player app, §7.8 sync/offline | Phone Client (the Squire) |
| §7.3 review + §7.6 redemption + §7.4 adjust, exposed as parent quick-actions over the network with an idempotent offline outbox | Parent Phone App (the Knight) |

- **Non-Functional Requirements** (PRD §8): offline-first (NFR-1), integrity (NFR-2), idempotency (NFR-3), durability (NFR-4), pairing/access control (NFR-5), LAN scope (NFR-6), timezone stability (NFR-7), performance <100ms (NFR-8), testability/property tests (NFR-9), core portability (NFR-10), observability/event-log inspection (NFR-11), low review friction (NFR-12).

## Use Cases **[CONDITIONAL: User-Facing Initiative]**

### Use Case 1: Offline claim → approval → balance
- **Actor**: Child (phone), then Parent (computer)
- **Scenario**: Child completes a daily quest while the computer is off; the claim queues in the outbox. On next sync it flushes and appears in the parent's review queue; the parent approves.
- **Expected Outcome**: Exactly one `CompletionApproved` with the snapshotted reward; child's balance rises on next refresh (AC-1, AC-3).

### Use Case 2: Gated, affordability-checked redemption
- **Actor**: Child (phone), Parent (computer)
- **Scenario**: Child requests a reward gated on an achievement and/or near their balance limit; the parent reviews.
- **Expected Outcome**: Locked rewards show why and cannot be redeemed; affordability is re-checked at commit, so a request can fail at approval with "insufficient points" if the balance was since drained (AC-5, AC-6).

## Architecture **[CONDITIONAL: Technically Complex Initiative]**

### Overview

Layered around a pure single-writer core. The network edge carries **two authenticated roles** — child (read + propose) and parent (privileged quick-actions) — but every command still commits through the one engine door on the Keep:

```
            THE KEEP (computer, Rust — single writer)
  ┌───────────────────────────────────────────────┐         THE SQUIRE (child phone, Kotlin)
  │  Admin App ──consumes──► Engine.handle()        │        ┌──────────────────────────┐
  │  (the Keep UI: authoring, full admin)  │        │        │  Player UI (Compose)     │
  │                           ▼            │        │        │  ◄ renders StateView      │
  │  Domain Core (pure, no I/O) ── types,  │        │ ┌LAN┐  │  Outbox (Room): claims/   │
  │     handle(), projections, events      │        ├─┤   ├──┤    requests (idempotent)   │
  │                           │            │        │ └───┘  │  Cache (last StateView)   │
  │  Repository (SQLite) ◄────┘ apply()/snapshot()  │  child │  ◄ CHILD pairing secret   │
  │                                        │        │  role  └──────────────────────────┘
  │  Local API server ─ two roles:         │        │
  │    child:  GET /state, POST /claims,   │        │        THE KNIGHT (parent phone, Kotlin)
  │            POST /redemption-requests   │        │ ┌LAN┐  ┌──────────────────────────┐
  │    parent: POST /admin/* (review,      │────────┼─┤   ├──┤  Review/queue UI (Compose)│
  │            redeem, adjust) ── privileged│        │ └───┘  │  Outbox (Room): privileged │
  │            commands, idempotent         │        │ parent │    commands (idempotent)   │
  └───────────────────────────────────────┘  role   │  ◄ PARENT pairing secret  │
                                                              └──────────────────────────┘
```

The network exposes two role-scoped surfaces, never a write path: the child role can only read + propose; the parent role can issue privileged quick-action commands but the **Keep is still the only writer** — it validates every command through `Engine::handle` before committing (AR-1 preserved; AR-8 revised from "admin ops never on the network" to "admin ops only under the parent credential"). Authoring (define/archive) stays local to the Keep — it is *not* in the parent quick-action set.

### Component → produced interface (spec ownership)

- **Domain Core** → `Engine::handle` (Command → Vec<Change>), `Projections` (balance, quests_due, current_streak, is_unlocked, can_redeem), and the domain/event/command types. Consumed by Admin App and the API layer.
- **Identity, Tenancy & Registration (SQUIRE-S-0007)** → tenant provisioning (schema-per-tenant via Diesel migrations, A-0002/A-0003), per-user accounts + roles (Knight/Squire), the registration path (create household → add Knights & Squires), and tenant-scoped token issuance/verification (A-0004). Provides the identity primitives (`UserId`/`Role`/`User`) the API's auth consumes and the per-Squire subject carried on every event (A-0005).
- **Persistence & Store** → `Repository` (snapshot/apply, single-writer) + `Clock`; **Diesel dual-backend** schema (SQLite local / Postgres hosted, same code, A-0003), schema-per-tenant isolation (A-0002), durability, full-store export.
- **Local API & Trust Boundary** → the HTTP surface + DTOs (per-Squire `StateView`, `SubmitClaim*`, `RequestRedemption*`, plus a per-household HouseholdReview) **plus a Knight quick-action surface** (review/redeem/adjust commands); **per-user tenant-scoped token** auth carrying a role (Knight/Squire), superseding pairing secrets (A-0004); idempotency semantics for both Squire submissions and Knight commands. Consumes Identity (S-0007) for auth. Consumed by both phone clients.
- **Admin App (the Keep)** → parent-facing authoring + review/redeem/adjust experience (consumes Engine directly, in-process — the full admin set including define/archive).
- **Phone Client (the Squire)** → child player experience, local cache, offline outbox, sync loop (consumes the child API).
- **Parent Phone App (the Knight)** → parent quick-action experience (approve/reject, mark-done, redeem, add-funds) over the parent API, with a cached review queue and an idempotent offline outbox of privileged commands. *No authoring.*

### Sequences

- **Child claim → approve:** `SubmitClaim` (Squire → child API) → append `CompletionClaimed` (or auto-approve → `CompletionApproved`) → parent `ReviewClaim{Approve}` (from the Keep *or* the Knight) → append `CompletionApproved{points snapshotted}` → next `GET /state` reflects new balance/streaks.
- **Parent quick-action (Knight, possibly offline):** parent taps approve/redeem/add-funds → command queued in the Knight outbox with a client-minted idempotency id → on reconnect, flushed to the parent API → `Engine::handle` validates and the Keep commits exactly once (a retry of the same id is a no-op) → reflected on next `GET /state`.

## Detailed Design **[REQUIRED]**

Detailed, per-component design and interface contracts are delegated to the child specification documents (one per component). The foundation is set by the decided tech ADRs: schema-per-tenant full isolation with a tenant-agnostic domain (A-0002), Diesel dual-backend persistence (A-0003), per-user accounts/roles/registration with tenant-scoped tokens (A-0004), and a per-Squire domain (A-0005). Accordingly the contract now carries identity (`UserId`/`Role`/`User`) and a per-Squire subject on every event. Cross-cutting design rules that every spec must honor:

- Single validated door: every mutation is a `Command` through `Engine::handle`, returning `Vec<Change>` applied atomically by the `Repository` (AR-5).
- Derived state only: balance/streaks/due/unlocks are computed from `Snapshot`, never stored as counters (AR-3).
- Snapshot-at-approval: `CompletionApproved.points` captures the quest reward at commit time (AR-4, AC-7).
- Timezone-stable dates: a single configured household timezone drives all `Date`/day-boundary math (NFR-7).
- Privileged-command idempotency (**resolved — ADR SQUIRE-A-0001**): parent quick-actions flush from an offline outbox and must be retry-safe. Reviews are naturally idempotent (they key on `claim_id`/`request_id`; the engine rejects double-review), and "mark-done" mints a `claim_id` and reuses the child claim path. Direct `RedeemItem` and `AdjustPoints` had no client id, so a client-minted `CommandId` is now **carried onto their emitted events**; the engine dedupes by finding an event already bearing that id — dedup stays derived from the append-only log (no side table). Two additive edits are now **applied to `shared_contract.rs`**: (1) the `CommandId` primitive + `command_id` on those two commands and their events (A-0001); and (2) identity — `UserId`/`Role`/`User` — plus a per-Squire subject on every event (A-0005).

## Testing Strategy **[CONDITIONAL: Separate Testing Initiative]**

- **Unit / property (core):** the domain core is pure and tested with an in-memory `Repository`. Property tests assert invariants — an approved claim counts exactly once; balance only goes negative via explicit adjustment; idempotent re-submission of a client id is a no-op (NFR-9).
- **Integration:** Repository against real SQLite (durability, restart, export round-trip); API idempotency and trust-boundary (admin commands not routable).
- **System / acceptance:** the eight PRD acceptance criteria (AC-1 … AC-8) drive end-to-end tests across both apps, including offline/queue/sync flows.

## Alternatives Considered **[REQUIRED]**

- **Store balances/streaks as mutable counters** — rejected; breaks auditability and makes reward edits rewrite history. Derived-from-log is an architectural invariant (AR-3).
- **Expose admin operations over the API with role checks** — *adopted, deliberately and narrowly*, to support the Knight (parent quick-admin phone). The original PRD made the boundary structural ("admin ops never on the network"); we revise that to "privileged ops require a separate parent credential." Crucially the deeper invariant is unchanged: the **child** role still cannot mint/approve/redeem (its surface offers no such route), and the **Keep is still the only writer** — the parent phone submits commands, it does not write. We accept a slightly larger attack surface (a parent secret on the LAN) in exchange for the review-loop convenience; authoring stays off the network to keep that surface minimal.
- **Knight as a second writer / peer store** — rejected; would break single-writer (AR-1) and the derive-from-one-log model. The Knight is a thin client that queues commands for the Keep to validate and commit.
- **Make the Knight require the computer online (no outbox)** — rejected; the parent should be able to act while the Keep is asleep, mirroring the child's offline-first experience. The cost is privileged-command idempotency (see Detailed Design).
- **Cloud relay / shared backend** — out of scope for v1; LAN-only keeps the system simple and private (PRD §3).
- **Cross-platform UI toolkit for both apps** — rejected; native Kotlin/Compose on the phone and a Rust-native admin UI keep each side idiomatic; the portable core is the shared asset, not the UI.

## Implementation Plan **[REQUIRED]**

Sequenced by dependency (the pure core unblocks everything):

1. **Domain Core** — types are fixed; implement `handle` + projections + property tests. *(unblocks all)*
2. **Persistence & Store** — `Repository`/`Clock` on **Diesel dual-backend** (SQLite local / Postgres hosted), durability, export. *(depends on 1)*
3. **Identity, Tenancy & Registration (SQUIRE-S-0007)** — schema-per-tenant provisioning via Diesel migrations (A-0002/A-0003), per-user accounts + roles + the registration path, tenant-scoped token issuance/verification (A-0004). Built near/with Persistence since provisioning uses Diesel migrations; **it unblocks the API's auth**. *(depends on 1, 2)*
4. **Local API & Trust Boundary** — Squire read+submit endpoints + the Knight quick-action surface, per-user tenant-scoped token auth, idempotency (incl. the contract extensions for `RedeemItem`/`AdjustPoints` and identity). *(depends on 1, 2, 3)*
5. **Admin App (the Keep)** — authoring + review queue + redeem/adjust. *(depends on 1, 2; parallel with 3, 4)*
6. **Phone Client (the Squire)** — per-Squire StateView render, cache, outbox, sync. *(depends on 4)*
7. **Parent Phone App (the Knight)** — review queue render, quick-actions, idempotent privileged outbox. *(depends on 4; parallel with 6)*

Each numbered item is its specification (SQUIRE-S-0001 … S-0007); tasks are decomposed per spec once the initiative reaches the decompose phase.