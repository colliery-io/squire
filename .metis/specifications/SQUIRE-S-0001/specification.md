---
id: domain-core
level: specification
title: "Domain Core"
short_code: "SQUIRE-S-0001"
created_at: 2026-06-17T00:52:18.319911+00:00
updated_at: 2026-06-17T00:52:18.319911+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# Domain Core

## Overview **[REQUIRED]**

The Domain Core is the pure, portable heart of Squire: the in-process Rust module that owns the domain/event/command types and implements the two behavioral traits defined in `shared_contract.rs` — `Engine` and `Projections`.

- `Engine::handle(snap, cmd, clock) -> Result<Vec<Change>, DomainError>` is the single validated door (AR-5): every mutation (authoring, archive, claim, review, redeem, adjust) is expressed as a `Command`, validated against a read-only `Snapshot`, and turned into a list of `Change`s. The core never applies changes — it only decides them. The caller (a `Repository`, implemented elsewhere) is the single writer.
- `Projections` derives all read state — `balance`, `quests_due`, `current_streak`, `is_unlocked`, `can_redeem` — from the event log and definitions alone (AR-3). Nothing derivable is stored as a mutable counter. **Every projection is now scoped to one Squire**: each takes a `squire: UserId`, and per-Squire balance/streaks/due/unlocks are pure filters over the household log on `squire` (ADR SQUIRE-A-0005).

The core is **per-Squire (subject-carrying)** (ADR SQUIRE-A-0005, forced by A-0004's one-or-more Squires): identity types (`UserId`, `Role { Knight, Squire }`, the credential-free domain `User`) are in scope of the contract, the `Snapshot` carries `users: Vec<User>`, and **every `Event` carries `squire: UserId`** — the household log is the union of its Squires' sub-logs, so per-Squire state is a log-filter (consistent with AR-3). The core stays pure and **tenant-agnostic**: a `Snapshot` IS one household's data (ADR SQUIRE-A-0002), and tenancy never appears in these types. Items & achievements remain a **shared household catalog** (authored once, available to every Squire); **quests are *assigned*** to one-or-more Squires (`Assignment::AllSquires` | `Squires(set)`) with a completion mode (`Completion::EachAssignee` | `Race`); only *activity* is per-Squire. Quest→Squire assignment, the `Race` first-approved-wins resolution, and **Knight-actor audit** (`actor: Option<UserId>` on the five privileged events) are now all **decided and in scope** (ADR SQUIRE-A-0005).

The core also OWNS the definitions of the identity `User`/`Role` types, the domain types (`Quest`, `RedeemableItem`, `Achievement`, and their sub-enums), the activity `Event` log shape, the `Command`/`Decision` intents, the `Change` output, the `Snapshot` read shape, and the `DomainError`/`Blocked` error enums. It does **not** manage users: `Change::PutUser`/`SetUserActive` exist but are produced by the Identity component (SQUIRE-S-0007), never by `Engine::handle`.

It is PURE: no I/O, storage, transport, clock, or UI dependencies. It depends only on the `Clock` and `Repository` *ports* (defined in the contract, implemented outside the core). This purity is the foundation of the system's trust model and integrity guarantee (NFR-2): because all rules — affordability, gating, single-review, snapshot-at-approval, idempotency — live in one pure function, the child's phone cannot mint points or self-approve, and any balance or streak is explainable by replaying the log through this one module. It is consumed in-process by the Admin App and the Local API layer.

## System Context **[CONDITIONAL: System-Level Spec]**

The core has no human actors directly; it is a library consumed in-process. Its "actors" are its two in-process callers.

### Actors
- **Admin App (in-process consumer)**: Calls `handle` with parent/admin `Command`s (`DefineQuest`, `ArchiveQuest`, `DefineItem`, `ArchiveItem`, `DefineAchievement`, `ArchiveAchievement`, `ReviewClaim`, `ReviewRedemption`, `RedeemItem`, `AdjustPoints`) and reads via `Projections`. It owns the review queue and direct redemption/adjustment. The four committing commands (`ReviewClaim`/`ReviewRedemption`/`RedeemItem`/`AdjustPoints`) carry the acting Knight `actor: UserId`, which `handle` stamps onto the emitted event (`None` only on auto-approve).
- **Local API layer (in-process consumer)**: Calls `handle` with the child-originated `Command`s (`SubmitClaim`, `RequestRedemption`) — each carrying the acting `squire`, which the API fills from the caller's token — on behalf of the phone, and reads via the per-Squire `Projections` to build that Squire's `StateView`. Admin commands arriving over this path must be rejected (`BadCommandForActor`).

### External Systems
- **None.** The core is pure and links no external systems. It interacts with the outside world only through the `Clock` and `Repository` ports, which are *defined* in the contract but *implemented* by other components.

### Boundaries
- **Inside the core**: the identity/domain/event/command/change/snapshot/error type definitions (including `User`/`Role`); `Engine::handle` validation and decision logic — including validating a command's `squire` against the active Squires in `Snapshot.users`; the per-Squire `Projections` derivations; all reasoning over the append-only event log (per-Squire balance, due, streaks, unlocks, affordability, derived by filtering on `squire`). The `Snapshot` the core reads over now includes `users: Vec<User>` (the household's Knights + Squires) alongside quests/items/achievements/events.
- **Outside the core**: user lifecycle management (the Identity component, SQUIRE-S-0007, produces `Change::PutUser`/`SetUserActive`; `Engine::handle` never does); the parent's cross-Squire read assembly (the `HouseholdReview` DTO — the core only provides the per-Squire projections it is built from); persistence/storage (SQLite, the `Repository` implementation and single-writer apply), transport (the HTTP API), UI (admin and phone), and the concrete `Clock` implementation (system time / configured timezone). The core depends only on the `Clock` and `Repository` *ports*; it receives a `Snapshot` and a `&dyn Clock` and returns `Change`s for someone else to apply.

## Requirements **[REQUIRED]**

### Functional Requirements

**The one validated door — `handle`**

| ID | Requirement | Rationale |
|----|-------------|-----------|
| REQ-1.1.1 | `handle` is the sole entry point for every mutation; it validates a `Command` against the `Snapshot` and returns `Vec<Change>` or a `DomainError`, applying nothing itself. | One place for all rules (PRD AR-5). |
| REQ-1.1.2 | Admin `Command`s (`DefineQuest`/`ArchiveQuest`/`DefineItem`/`ArchiveItem`/`DefineAchievement`/`ArchiveAchievement`/`ReviewClaim`/`ReviewRedemption`/`RedeemItem`/`AdjustPoints`) are accepted from the admin caller and produce the corresponding `Change`s (`PutQuest`/`SetQuestActive`/`PutItem`/… or `Append(Event)`). An admin command presented on the child path is rejected `BadCommandForActor`. | Trust boundary (PRD AR-8, FR-A1..A3, FR-ADM*). |
| REQ-1.1.3 | Child `Command`s `SubmitClaim` and `RequestRedemption` are accepted and append `CompletionClaimed` / `RedemptionRequested` respectively (each carrying the acting `squire`); neither moves the balance. | PRD FR-C1, FR-C2, FR-R1. |
| REQ-1.1.4 | `SubmitClaim` / `RequestRedemption` are idempotent on the client-minted `claim_id` / `request_id`: re-submitting an id already present in the log is a no-op (returns no duplicate `Change`). | PRD FR-SY3, NFR-3; property: idempotent re-submit is a no-op. |
| REQ-1.1.5 | Every subject-bearing `Command` names a `squire` that `handle` validates against `Snapshot.users`: an unknown id is rejected `UserNotFound`, and a non-Squire (or inactive Squire) id is rejected `NotASquire`. The acting/target `squire` is carried explicitly by `SubmitClaim`/`RequestRedemption` (acting) and `RedeemItem`/`AdjustPoints` (target); `ReviewClaim`/`ReviewRedemption` carry no `squire` — it is derived from the referenced claim/request. Every emitted `Event` carries the resolved `squire`. | ADR SQUIRE-A-0005; per-user accounts (A-0004). |

**Claim, review & snapshot semantics**

| ID | Requirement | Rationale |
|----|-------------|-----------|
| REQ-1.2.1 | `SubmitClaim` is allowed only for an active quest that is due on `on`; claims against unknown/inactive quests are rejected (`QuestNotFound` / `Inactive`). | PRD FR-C1, FR-Q1. |
| REQ-1.2.1a | The claiming `squire` must be an **assignee** of the quest (`AllSquires` = every active Squire, auto-including Squires added later; `Squires(set)` = explicit membership); a non-assignee claim is rejected `NotAssigned`. | ADR SQUIRE-A-0005. |
| REQ-1.2.1b | For a `Race` quest, the `(quest, on)` occurrence stays **open** until the first completion is *approved*; that approval closes it and is the sole payout. While open, **any** assignee may `SubmitClaim` (multiple pending claims are allowed — a false/early claim must not lock out the real doer). Approving or reviewing a claim against an already-closed occurrence is rejected `OccurrenceTaken`; a **rejected** first claim leaves the occurrence open for a later claim. An `EachAssignee` quest gives each assignee an independent occurrence/claim/payout. | ADR SQUIRE-A-0005; property: a `Race` occurrence yields exactly one approved completion / one payout. |
| REQ-1.2.2 | If the quest's `auto_approve` is true, `SubmitClaim` emits `CompletionClaimed` then immediately `CompletionApproved` with the quest's current `reward` snapshotted. | PRD FR-C3. |
| REQ-1.2.3 | When `repeatable_within_day` is false, a second claim for the **same (squire, quest, `on`)** (while one is open or approved) is rejected `AlreadyClaimedToday`; the uniqueness is per-Squire, so two different Squires may each claim the same quest on the same day. When true, each claim is independent and pays out again. | PRD FR-Q4; ADR SQUIRE-A-0005. |
| REQ-1.2.4 | `ReviewClaim` with `Approve` emits `CompletionApproved` carrying the quest's **current** `reward` snapshotted onto the event; `Reject` emits `CompletionRejected` with the optional reason. Both events carry `actor: Some(knight)` from the command's acting Knight (`None` only when emitted via `auto_approve`). For a `Race` quest, `Approve` against an occurrence already closed by an earlier approval is rejected `OccurrenceTaken` (not committed). | PRD FR-C4, AR-4; ADR SQUIRE-A-0005. |
| REQ-1.2.5 | A claim may be reviewed at most once; reviewing an already-approved/rejected claim is rejected `AlreadyReviewed`. Reviewing an unknown claim is `ClaimNotFound`. | PRD FR-C5. |
| REQ-1.2.6 | Streak and achievement evaluation uses the claim's `on` date (day the chore was done), never the approval timestamp. | PRD FR-C6, FR-S1/S2. |

**Ledger / balance**

| ID | Requirement | Rationale |
|----|-------------|-----------|
| REQ-1.3.1 | `Projections::balance(snap, squire)` = sum over **that Squire's** events in the log of `CompletionApproved (+points)`, `ItemRedeemed (−cost)`, `AchievementUnlocked (+bonus)`, `PointsAdjusted (±amount)`; pending claims/requests contribute nothing. Balance is per-Squire (a filter on `squire`), derived only, never stored. | PRD FR-P1, FR-P3, AR-3; ADR SQUIRE-A-0005. |
| REQ-1.3.2 | `AdjustPoints` carries the acting Knight `actor` and the target `squire`, requires a non-empty reason, and appends `PointsAdjusted { squire, actor: Some(knight), amount, reason }` (`actor: None` only for a Keep-local adjustment); it is the only path by which a Squire's balance can go negative. | PRD FR-P2; property: balance only negative via explicit adjustment. |

**Projections (read derivations)**

| ID | Requirement | Rationale |
|----|-------------|-----------|
| REQ-1.4.1 | `quests_due(snap, squire, on)` returns active quests whose cadence matches `on`, **for which `squire` is an assignee** (`AllSquires` ⇒ every active Squire; `Squires(set)` ⇒ explicit), minus those already satisfied **for that Squire** on `on`. One-off quests are due until that Squire has an approved completion; recurring quests are due each matching scheduled day; `EveryNDays` days are computed from `anchor` and interval `n`. Additionally, a `Race` quest is due only while its `(quest, on)` occurrence is still **open** (no approved completion yet for that quest+date) — once won it drops from every assignee's due list. | PRD FR-Q1, FR-Q2, FR-Q3; ADR SQUIRE-A-0005. |
| REQ-1.4.2 | `current_streak(snap, squire, scope, basis, asof)` computes **that Squire's** streak length over their events: `ScheduledOccurrences` basis counts consecutive completed scheduled occurrences for a quest scope (a non-scheduled-day gap does not break it); `CalendarDays` basis counts consecutive calendar days with ≥1 in-scope completion. A repeatable quest counts as one occurrence per day. Streaks are per-Squire. | PRD FR-S1, FR-S2, FR-S3, FR-S6; ADR SQUIRE-A-0005. |
| REQ-1.4.3 | `is_unlocked(snap, squire, id)` returns true iff an `AchievementUnlocked` for that id **and that Squire** exists in the log; the unlock is sticky per-Squire and survives a later streak break. | PRD FR-S4; ADR SQUIRE-A-0005. |
| REQ-1.4.4 | When a `Criterion` (`Streak` / `TotalCompletions` / `PointsEarned`) is first satisfied **for the approval's Squire** (evaluated as part of `handle`), the engine emits `AchievementUnlocked { squire, … }` exactly once for that Squire, awarding `bonus_points` and thereby unlocking any item gated on it for that Squire. | PRD FR-S4, FR-S5; ADR SQUIRE-A-0005. |
| REQ-1.4.5 | `can_redeem(snap, squire, item, on)` returns `Ok(())` only if, at the moment of evaluation: the item is active, its gating achievement (if any) is unlocked **for that Squire**, **that Squire's** balance covers the cost, and — when `Availability::Once` — **no `ItemRedeemed` for the item already exists in the log** (out-of-stock, derived from the log; no stored counter, AR-3); otherwise `Err(Blocked::…)` with the specific reason (`InsufficientPoints` / `AchievementLocked` / `OutOfStock`). `Availability::Repeatable` imposes no limit; the most-recent `ItemRedeemed` for the item (`last_redeemed`, a trivial log projection) is surfaced on `RewardCard` for the parent to eyeball. There is no rate-limit math. | PRD FR-R4; ADR SQUIRE-A-0006, SQUIRE-A-0005. |

**Redemption commit semantics**

| ID | Requirement | Rationale |
|----|-------------|-----------|
| REQ-1.5.1 | `ReviewRedemption(Approve)` (Squire derived from the request) and direct `RedeemItem` (Squire carried as the target) re-run the `can_redeem` checks **for that Squire at commit**; on pass they emit `ItemRedeemed { squire, actor: Some(knight), … }` (with `request_id` for the request path, `command_id` for the direct path); on fail they return `DomainError::Redeem(Blocked::…)`. No points are reserved while a request is pending. | PRD FR-R2, FR-R3, FR-R4, FR-R5, AC-6; ADR SQUIRE-A-0005. |
| REQ-1.5.2 | `ReviewRedemption(Reject)` emits `RedemptionRejected { actor: Some(knight), … }` with the optional reason; an already-reviewed request is `AlreadyReviewed`, an unknown one `RequestNotFound`. | PRD FR-R2, FR-C5 (analogous). |
| REQ-1.5.3 | **Knight-actor audit.** The five Knight-committed events (`CompletionApproved`, `CompletionRejected`, `ItemRedeemed`, `PointsAdjusted`, `RedemptionRejected`) carry `actor: Option<UserId>` — the acting Knight from the originating command's `actor`, stamped onto the event by `handle`; `None` denotes auto-approve / system commit. Squire-originated events (`CompletionClaimed`, `RedemptionRequested`) carry no actor (the `squire` is the actor). This lets observability (NFR-11) answer *who* committed a fact, not just what. | ADR SQUIRE-A-0005; PRD NFR-11. |

### Non-Functional Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| NFR-1.1.1 | All `on` dates and day boundaries are computed in a single configured household timezone via the `Clock` port; due and streak math is timezone-stable and does not shift with travel or DST. | PRD NFR-7. |
| NFR-1.1.2 | Every per-Squire projection (balance, streaks, due list) computes in well under 100 ms at household scale (a handful of Squires, dozens of shared quests, thousands of events over years) — the per-Squire filter on the union log must not blow the budget. | PRD NFR-8; ADR SQUIRE-A-0005. |
| NFR-1.1.3 | The core is pure and unit-testable against an in-memory `Repository`; it ships property tests for the invariants, evaluated per-Squire: an approved claim is counted exactly once for its Squire; a Squire's balance is never negative except via explicit `PointsAdjusted`; re-submitting the same client id is a no-op; one Squire's activity never moves another Squire's balance/streaks/unlocks. | PRD NFR-9; ADR SQUIRE-A-0005. |
| NFR-1.1.5 | Additional property tests pin the assignment/Race/availability rules: a `Race` occurrence yields **exactly one** approved completion and **one** payout no matter how many assignees claim concurrently; a **rejected** first claim **reopens** the occurrence (a later claim can still win); and an `Availability::Once` item is redeemable **exactly once household-wide** (the first `ItemRedeemed` makes every subsequent `can_redeem` return `OutOfStock`, derived from the log with no stored counter), while a `Repeatable` item is never blocked on availability. | PRD NFR-9; ADR SQUIRE-A-0006, SQUIRE-A-0005. |
| NFR-1.1.4 | The engine and types have no dependency on storage, transport, or UI; swapping SQLite or the API transport must not touch the core. | PRD NFR-10, AR-7. |

## Architecture Framing **[CONDITIONAL: System-Level Spec]**

The types and trait signatures are fixed by `shared_contract.rs`; the genuine open decisions are internal to the implementation.

### Decision Area: Streak & projection computation strategy
- **Context**: Streaks (`ScheduledOccurrences` vs `CalendarDays`), due lists, and balance must be derived from the full event log. The algorithm and any internal indexing must stay correct under schedule-aware gaps while meeting the <100 ms budget over thousands of events.
- **Constraints**: Pure (no I/O); timezone-stable date math; output identical to a naive log replay; types unchanged.
- **Required Capabilities**: Walk/aggregate the event log per scope and date; map a `Schedule` to its occurrence days from an anchor; evaluate criteria for unlock-on-approval; complete within NFR-8.
- **ADR**: TBD.

### Decision Area: Derived-state representation — recompute vs incremental
- **Context**: `handle` and `Projections` reason over a `Snapshot` that the caller supplies. We can recompute everything from the raw log on each call, or maintain incremental/memoized aggregates inside the core's evaluation. This trades simplicity and provable correctness against per-call cost at scale.
- **Constraints**: Must remain pure and stateless across calls (no hidden mutable cache surviving outside the `Snapshot`); AR-3 — nothing derivable persisted as authoritative mutable state.
- **Required Capabilities**: Produce balance/streak/due/unlock results that always equal a fresh log replay; stay within NFR-8.
- **ADR**: TBD.

### Decision Area: Item availability semantics with multiple Squires — SUPERSEDED by ADR SQUIRE-A-0006
- **SUPERSEDED**: This decision area is **superseded by ADR SQUIRE-A-0006**, which simplified `Availability` to `{ Once, Repeatable }` and dropped all rate-limit math (and with it `Blocked::RateLimited` / `LockReason::RateLimited`). The per-Squire-vs-household question it framed (the old `PerDay`/`PerWeek`/`LimitedTotal` variants) no longer exists.
- **Current rule (ADR SQUIRE-A-0006)**: `Availability::Once` is a **single household-wide redemption** — out-of-stock the moment any `ItemRedeemed` for the item exists in the log (derived from the log, no stored counter, AR-3), surfaced as `Blocked::OutOfStock` / `LockReason::OutOfStock`. `Availability::Repeatable` is **unlimited** (never blocked on availability); the UI shows `RewardCard.last_redeemed` (the most-recent `ItemRedeemed` for the item, a trivial log projection) so a parent can eyeball recent use.
- **ADR**: SQUIRE-A-0006 (Decided; supersedes the SQUIRE-A-0005 framing of this area).

## Constraints **[CONDITIONAL: Has Constraints]**

### Technical Constraints
- **Pure / no I/O.** The core performs no storage, transport, UI, or direct clock access; it reads a `Snapshot` (now including `users: Vec<User>`) and a `&dyn Clock` and returns `Change`s. It depends only on the `Clock` and `Repository` ports.
- **Tenant-agnostic.** The core stays per-Squire but tenancy-blind: a `Snapshot` IS exactly one household's data, and no tenant/household identifier appears in any type the core touches (ADR SQUIRE-A-0002). Per-Squire scoping is achieved purely by the `squire: UserId` subject on events and the `squire` argument to projections (ADR SQUIRE-A-0005).
- **Types are FIXED by the updated `shared_contract.rs`.** All identity/domain/event/command/change/snapshot/error types and the `Engine`/`Projections` signatures — now including `User`/`Role`/`UserId`, `squire` on every `Event`, the subject-bearing commands, the per-Squire projection signatures, the `UserNotFound`/`NotASquire` errors, and `Snapshot.users` — are the shared seam every component builds against and must not be changed by this component.
- **User lifecycle is out of scope.** The core does not produce `Change::PutUser`/`SetUserActive`; those come from the Identity component (SQUIRE-S-0007). `Engine::handle` only reads `Snapshot.users` to validate a command's `squire`.
- **Timezone-stable date math.** All `on` dates and day boundaries resolve in the single configured household timezone; logic must not shift with travel or DST.
- **Rust.** The core is implemented in Rust as a portable, dependency-light crate (no storage/transport/UI crates).