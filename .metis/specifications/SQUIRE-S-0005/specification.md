---
id: phone-client-the-squire
level: specification
title: "Phone Client (the Squire)"
short_code: "SQUIRE-S-0005"
created_at: 2026-06-17T00:52:33.314967+00:00
updated_at: 2026-06-17T00:52:33.314967+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# Phone Client (the Squire)

## Overview **[REQUIRED]**

The Squire is the child-facing native Android app (Kotlin + Jetpack Compose, Room/SQLite for cache and outbox; AR-6). It is an offline-first, read-and-propose-only player client: it renders **this Squire's own** per-Squire state entirely from a single `StateView` payload (now carrying `squire`; A-0005) fetched from the Local API (SQUIRE-S-0003) and lets the child *propose* exactly two things — a completion claim (`SubmitClaimReq`) and a redemption request (`RequestRedemptionReq`) — over the only network surface that exists.

The Squire never commits. It is structurally incapable of minting points, approving a claim, or redeeming an item, because the API exposes no such operation (AR-8, NFR-2): trust is a property of the API surface, not of any client-side check. The phone holds no balance-of-record and enforces no rules; it caches the computer's last word and queues the child's intents.

Offline is the normal case (NFR-1): the computer need not be reachable for the child to view progress or to act. The phone renders fully from the last cached `StateView` and writes new claims/requests into a local outbox. Each submission carries a phone-minted id, so flushing the outbox is idempotent (NFR-3) and "Sync" reduces to *flush the outbox, then re-fetch `StateView`*. The UI is age-appropriate and leans on progression feedback — points, streaks, and milestone proximity (FR-PL5).

## System Context **[CONDITIONAL: System-Level Spec]**

### Actors
- **Child / Player (human, on the phone)**: the sole user of the Squire. Views today's quests, balance, streaks, and rewards; submits completion claims and redemption requests. Has no authority to approve, redeem, or mint — only to look and to propose.

### External Systems
- **Local API over LAN (SQUIRE-S-0003)**: the Squire's only collaborator. The phone is a pure consumer — `GET /state` → `StateView`, `POST /claims` (`SubmitClaimReq`/`SubmitClaimResp`), `POST /redemption-requests` (`RequestRedemptionReq`/`RequestRedemptionResp`). Reached over the home LAN with a **per-user Squire token** — pairing binds the device to a `(household, Squire)` and yields a tenant-scoped bearer token (issuance owned by Identity, SQUIRE-S-0007; A-0004), not a shared child secret; address/reachability may change at any time.
- **Local Room (SQLite) database**: on-device store holding the last-fetched `StateView` (cache) and the outbox of not-yet-acknowledged claims/requests. Enables full offline render and durable queuing across app/process restarts.

### Boundaries
**Inside the Squire:** the player UI (Compose), the `StateView` cache, the submission outbox, the sync loop (flush-then-refetch), phone-minted `claim_id`/`request_id` generation, per-user Squire-token storage, and "computer not found" handling that falls back to cache + outbox.

**Outside the Squire (lives on the computer):** all domain rules and validation, the authoritative balance and event log, "quests due" and streak/achievement derivation, and every commit-bearing operation — approve, reject, redeem, define, archive, adjust. The phone consumes the results of these; it can never perform them (AR-8, NFR-2).

## Requirements **[REQUIRED]**

### Functional Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| REQ-PL1 | Render the whole player home — today's quests with status, current balance, streaks, available rewards, and the status of the child's recent claims and requests — from a single `StateView` payload, with no client-side recomputation of any of it. | FR-PL1. One server-derived payload is the only truth the phone shows; the phone never derives balance/streaks/due itself (AR-3). |
| REQ-PL2 | For each `RewardCard`, show whether it is affordable (`affordable`), the last redemption (`last_redeemed: Option<Timestamp>`), and, if locked, the reason taken from `lock: Option<LockReason>` — `NeedsAchievement` (with name) or `OutOfStock` only. Availability is `Once`/`Repeatable`: a `Once` item goes out-of-stock (locked) after its single redemption; a `Repeatable` item never runs out (SQUIRE-A-0006). | FR-PL2. The child must understand why a reward can't be taken yet; the server supplies the reason and the last redemption, the phone only displays them. |
| REQ-PL3 | The player sees only quests **assigned** to them (A-0005); for each `QuestCard`, show its `QuestStatus` — Available / Pending (review) / CompletedToday — and render `TakenByOther` for a `Race` quest a sibling has already claimed/won (so the child doesn't double-effort). | FR-PL3, A-0005. The child sees, per assigned quest, whether to act, wait, rest, or stand down. |
| REQ-PL4 | Let the child submit a completion claim (`SubmitClaimReq`) and a redemption request (`RequestRedemptionReq`) — the only two intents the phone can express. | FR-PL4. These are the child's sole write paths; both are proposals, never commits. |
| REQ-PL5 | Present an age-appropriate UI that emphasizes progression: points, streaks (`StreakView.current/best/alive`), and milestone proximity via `StreakView.next_milestone`. | FR-PL5. Motivation/progression is a product goal for the child audience. |
| REQ-SY1 | Cache the last successfully fetched `StateView` in Room and render the entire home from cache when the computer is unreachable. | FR-SY1, NFR-1. Viewing must never depend on the computer being online. |
| REQ-SY2 | Persist claims and redemption requests created while offline into a durable local outbox; submit them when the computer becomes reachable. | FR-SY2, NFR-1. Queuing must never depend on the computer being online, and must survive app restarts. |
| REQ-SY3 | Mint a stable id on the phone for every submission (`claim_id` / `request_id`) at creation time; reuse the same id on every retry so resubmission is idempotent and creates no duplicate. | FR-SY3, NFR-3. The outbox may retry freely; the API is idempotent on the client-minted id. |
| REQ-SY4 | "Sync" = flush the outbox (POST each pending item, idempotently), then re-fetch `StateView`; an outbox item is considered resolved when its status appears/changes in the refreshed `my_claims` / `my_requests`. | FR-SY4. A single, well-defined reconciliation step; pending UI resolves from refreshed server state, not from local guesses. |
| REQ-SY5 | Handle "computer not found" / unreachable LAN gracefully: no error state blocks the app — fall back to the cached `StateView` for rendering and keep accepting submissions into the outbox for later flush. | NFR-6, NFR-1. The computer's address/reachability may change; the child keeps working regardless. |

### Non-Functional Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| NFR-1 | Offline-first: viewing cached state and queuing submissions must function with the computer unreachable; neither device need be online simultaneously for the child to act. | PRD NFR-1. Core product guarantee for the child experience. |
| NFR-2 | Integrity by construction: the phone is incapable of minting points, approving its own submissions, or redeeming — it exposes/uses no such operation because the API offers none (AR-8). | PRD NFR-2. Trust comes from the API surface, not from client-side checks that could be bypassed. |
| NFR-5 | The phone holds a **per-user Squire token** (not a shared child secret): pairing binds the device to a `(household, Squire)` and yields a tenant-scoped bearer token (issued by Identity, SQUIRE-S-0007; A-0004), presented on every API call so no other LAN device can read state or submit as this Squire. | PRD NFR-5, A-0004. Per-user accounts replace shared pairing secrets; SQUIRE-S-0007 owns token issuance. |
| NFR-6 | Communication is local HTTP over the home LAN; the phone tolerates a changing/absent computer address and degrades to cache + outbox rather than failing. | PRD NFR-6. LAN-only, no cloud relay; reachability is intermittent by design. |
| NFR-UX | The UI is age-appropriate for the child and foregrounds progression feedback (points, streaks, milestone proximity) over administrative detail. | FR-PL5. Suits the player audience and the gamification goal. |

## Architecture Framing **[CONDITIONAL: System-Level Spec]**

### Decision Area: Room schema for cache + outbox, and outbox retry/backoff policy
- **Context**: How the last `StateView` and the pending claims/requests are persisted in Room, and how/when the outbox is drained (retry cadence, backoff, ordering, terminal-state handling once a server status resolves an item).
- **Constraints**: Room/SQLite is fixed (AR-6); cache must support full offline render (FR-SY1); outbox must be durable across restarts (FR-SY2) and safe to retry because submissions are idempotent on phone-minted ids (FR-SY3/NFR-3).
- **Required Capabilities**: store and read back a whole `StateView`; enqueue/dequeue submissions with their minted ids and state; retry on reachability with bounded backoff; mark items resolved from refreshed `StateView` (FR-SY4).
- **ADR**: TBD

### Decision Area: Deserializing `StateView` from the Rust-owned contract on the Kotlin side
- **Context**: `StateView` and all DTOs are defined authoritatively in `shared_contract.rs`; the Kotlin client needs typed access to them. Options: a shared serialized schema / codegen from the Rust types vs hand-written Kotlin DTOs kept in sync by convention.
- **Constraints**: The Rust contract is the single source of truth; Kotlin types must not drift from it. Wire format (e.g. JSON) is set with the API spec (SQUIRE-S-0003). `StateView` is now per-Squire and carries `squire` (A-0005), which must decode. Enums like `QuestStatus` (including the new `TakenByOther`; A-0005), `LockReason`, `ClaimState`, `RedemptionState` must round-trip exactly.
- **Required Capabilities**: decode `StateView` and encode `SubmitClaimReq` / `RequestRedemptionReq` faithfully; a drift-detection or generation story so contract changes surface on the Kotlin side.
- **ADR**: TBD

### Decision Area: Sync trigger strategy
- **Context**: When the flush-then-refetch sync runs — manual pull-to-refresh, automatically on app foreground, periodic background polling, or some combination.
- **Constraints**: Offline-first (NFR-1) — sync is best-effort and never blocks viewing or queuing; LAN-only with intermittent reachability (NFR-6); single-child scale (no aggressive polling needed).
- **Required Capabilities**: trigger sync without freezing the UI; reconcile freshly submitted outbox items against refreshed state; behave sanely when the computer is absent.
- **ADR**: TBD

## Constraints **[CONDITIONAL: Has Constraints]**

### Technical Constraints
- Native Android: Kotlin + Jetpack Compose, with Room (SQLite) for the cache and outbox — fixed by AR-6, not an implementation choice.
- Authenticates with a **per-user Squire token**, not a shared child secret: pairing binds the device to a `(household, Squire)` and yields a tenant-scoped bearer token presented on every call (issuance owned by Identity, SQUIRE-S-0007; A-0004). The rendered `StateView` is this Squire's own per-Squire view and now carries `squire` (A-0005).
- Pure API consumer: the phone uses only the three Local API operations (`GET /state`, `POST /claims`, `POST /redemption-requests`); it has no access to any privileged/commit operation (AR-8, NFR-2).
- Must render fully from the cached `StateView` while the computer is unreachable (FR-SY1, NFR-1); no screen may hard-depend on a live fetch.
- Every submission carries a phone-minted id (`claim_id` / `request_id`) so retries are idempotent (FR-SY3, NFR-3).
- DTOs are fixed by `shared_contract.rs` (`StateView`, `QuestCard`/`QuestStatus`, `StreakView`, `RewardCard`/`LockReason`, `ClaimStatus`/`ClaimState`, `RedemptionStatus`/`RedemptionState`, `SubmitClaimReq`/`Resp`, `RequestRedemptionReq`/`Resp`); the Kotlin side mirrors them and must not diverge.