---
id: parent-phone-app-the-knight
level: specification
title: "Parent Phone App (the Knight)"
short_code: "SQUIRE-S-0006"
created_at: 2026-06-17T01:34:41.259798+00:00
updated_at: 2026-06-17T01:34:41.259798+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# Parent Phone App (the Knight)

## Overview **[REQUIRED]**

The Knight is the parent-facing native Android app (Kotlin + Jetpack Compose, Room/SQLite for cache and outbox; AR-6). It is the parent's pocket remote for the **review loop** — an offline-first, parent-authenticated quick-action client. Where the Squire (SQUIRE-S-0005) is the child's read-and-propose app, the Knight is its sibling: it issues *privileged* quick-actions over a **per-user Knight token** (not a shared parent secret; A-0004), and it reads the cross-Squire `HouseholdReview` — pending claims/requests across **all** Squires, each labeled with its Squire, plus per-Squire balances (A-0005) — for context. It is a new component added after the original PRD; the parent's authoring still lives on the computer (the Keep, SQUIRE-S-0004).

The Knight never commits. The Keep remains the **single writer** (AR-1): the Knight submits privileged `Command`s to the parent surface of the Local API (SQUIRE-S-0003), and the Keep validates every one through `Engine::handle` before committing. The Knight holds no balance-of-record and enforces no rules; it caches the Keep's last word and queues the parent's intents.

The Knight is **quick-actions only, no authoring**. Its command set, drawn from `shared_contract.rs` `Command`, is exactly: `ReviewClaim{Approve/Reject}`, `ReviewRedemption{Approve/Reject}`, `RedeemItem` (direct redeem), `AdjustPoints` (add funds when the amount is positive, with a required reason), and "mark a quest done for the child" (mint a `claim_id`, submit the claim, and immediately approve it — reusing the child claim path so the `claim_id` provides idempotency). Quick-actions that target a child carry the **target `squire`** (`RedeemItem`/`AdjustPoints`); approving a claim or request derives the Squire from the item itself (A-0005). Every privileged command also auto-records the acting Knight as its **`actor`** — the Knight's own identity from its token, filled by the API, not chosen on the client (A-0005). Define/Archive of quests, items, and achievements is **not** here, and **member management (adding Knights/Squires) stays on the Keep** (SQUIRE-S-0004) for MVP — the Knight remains quick-actions only.

Offline is a first-class case (NFR-1 analog for the parent): the parent can act while the Keep is asleep. Privileged commands queue in a durable Room outbox with client-minted idempotency ids and flush on reconnect, retry-safe — no double-approve, double-redeem, or double-credit. "Sync" reduces to *flush the outbox, then re-fetch state*; when the Keep is unreachable, the Knight renders the review queue from cache and keeps accepting actions into the outbox.

## System Context **[CONDITIONAL: System-Level Spec]**

### Actors
- **Parent / Admin (human, on their phone, on the LAN)**: the sole user of the Knight. Works the review loop from their pocket — approves/rejects pending claims and redemption requests, marks a quest done on the child's behalf, redeems an item directly, and adds funds (a positive `AdjustPoints` with a reason). Authenticated by a **per-user Knight token** (not a shared parent secret; A-0004), and works the cross-Squire `HouseholdReview` across all Squires. Does no authoring or member management here.

### External Systems
- **Parent surface of the Local API over LAN (SQUIRE-S-0003)**: the Knight's only collaborator. The Knight is a pure consumer of a parent-scoped surface — it reads the cross-Squire `HouseholdReview` (pending claims + pending redemption requests across **all** Squires, each labeled with its Squire, plus per-Squire balances; A-0005) via a parent state read, and submits privileged quick-action commands carrying the target `squire` where applicable. Every command is validated by the Keep through `Engine::handle`; the Knight never writes the store. Reached over the home LAN with the **per-user Knight token** (A-0004); address/reachability may change at any time.
- **Local Room (SQLite) database**: on-device store holding the last-fetched review-queue/balance state (cache) and a durable outbox of not-yet-acknowledged **privileged** commands with their client-minted idempotency ids. Enables full offline render and durable, retry-safe queuing across app/process restarts.

### Boundaries
**Inside the Knight:** the parent review/quick-action UI (Compose), the cross-Squire `HouseholdReview` cache (per-Squire balances), the idempotent privileged outbox (client-minted ids, retry-safe flush), the sync loop (flush-then-refetch), per-user Knight-token storage, and "computer not found" handling that falls back to cache + outbox.

**Outside the Knight (lives on the computer, the Keep):** all authoring (define/archive — Keep only), all domain rules and the balances/streaks-of-record, the authoritative event log, and **the actual write** — the Keep is the sole writer and validates every command through `Engine::handle`. The Knight proposes privileged commands and displays results; it can never commit them itself (AR-1).

## Requirements **[REQUIRED]**

### Functional Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| REQ-K1 | Render the cross-Squire **`HouseholdReview`** — pending completion claims and pending redemption requests across **all** Squires (each labeled with its Squire), plus per-Squire balances — from a parent state read; render from the Room cache when the Keep is unreachable, with no client-side recomputation. | FR-ADM1; A-0005. The parent's whole working surface is a server-derived view spanning every Squire; the Knight never derives balance itself (AR-3). |
| REQ-K2 | Let the parent approve or reject a pending claim, issuing `ReviewClaim{claim_id, Approve}` / `{Reject{reason}}`. The Squire is derived from the claim itself — no separate target needed. Reviewing a `Race` claim whose occurrence was already won may return `OccurrenceTaken`; surface it gracefully (the sibling already won — no double payout). | FR-C4, FR-ADM1, A-0005. The core of the review loop, now reachable from the parent's phone, across all Squires. |
| REQ-K3 | Let the parent mark a quest done for the child: mint a `claim_id`, submit the claim, and immediately approve it (reusing the child claim path). | FR-C1/FR-C4 (new: parent quick-admin). Lets the parent credit a chore the child did but didn't claim; the minted `claim_id` provides idempotency (FR-SY3/NFR-3 analog). |
| REQ-K4 | Let the parent approve or reject a pending redemption request, issuing `ReviewRedemption{request_id, Approve}` (→ `ItemRedeemed`) / `{Reject{reason}}`. The Squire is derived from the request itself — no separate target needed. | FR-R2, FR-ADM1/3, A-0005. The redemption half of the review loop from the phone, across all Squires. |
| REQ-K5 | Let the parent redeem an item directly without a prior request, issuing `RedeemItem{squire, item_id}` (→ `ItemRedeemed` with no `request_id`). The target **`squire`** is carried on the command. Where rewards are shown, surface the last redemption (`RewardCard.last_redeemed`) and any lock reason — `NeedsAchievement`/`OutOfStock` only (a `Once` item is `OutOfStock` once redeemed; a `Repeatable` never runs out; SQUIRE-A-0006). | FR-R3, FR-ADM3, A-0005. Direct spend on a specific Squire's behalf; affordability/availability re-checked by the Keep at commit (FR-R4, SQUIRE-A-0006). |
| REQ-K6 | Let the parent add funds via `AdjustPoints{squire, amount, reason}` with `amount` positive and a **required** reason; the action is blocked until a non-empty reason is supplied. The target **`squire`** is carried on the command. | FR-P2, FR-ADM3, A-0005. Restates FR-P2's required-reason rule — every manual adjustment must carry a reason for the audit log. |
| REQ-K7 | Persist every privileged command issued offline into a durable Room outbox and submit it when the Keep becomes reachable. | FR-SY2 analog, NFR-1 analog (new: parent quick-admin). The parent must be able to act while the Keep is asleep; the queue survives app restarts. |
| REQ-K8 | Mint a stable client idempotency id on the phone for every privileged command at creation time and reuse it on every retry, so flushing the outbox is retry-safe — no double-approve, double-redeem, or double-credit. | FR-SY3/NFR-3 analog (new: parent quick-admin). Reviews and mark-done key naturally on `claim_id`/`request_id`; `RedeemItem`/`AdjustPoints` use the `command_id` added by ADR SQUIRE-A-0001. |
| REQ-K9 | "Sync" = flush the outbox (submit each pending privileged command idempotently), then re-fetch the parent state; an outbox item is resolved when the refreshed state reflects it (queue item gone / balance changed). | FR-SY4 analog. A single well-defined reconciliation step; pending UI resolves from refreshed server state, not from local guesses. |
| REQ-K10 | Handle "computer not found" / unreachable LAN gracefully: no error state blocks the app — fall back to the cached review queue + balance for rendering and keep accepting actions into the outbox for later flush. | NFR-6, NFR-1 analog. The Keep's address/reachability may change; the parent keeps working regardless. |

### Non-Functional Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| NFR-1 | Offline-first (parent): viewing the cached review queue/balance and queuing privileged commands must function with the Keep unreachable; the parent need not have the Keep online to act. | PRD NFR-1 analog. The Knight mirrors the child's offline-first guarantee for the parent's review loop. |
| NFR-2 | Integrity / single-writer preserved: the Knight proposes commands but never writes the store; the Keep validates every one through `Engine::handle` and is the sole writer. The **child** app remains structurally incapable of any of these privileged actions. | PRD NFR-2, AR-1. Convenience for the parent must not compromise the single-writer invariant or the child's trust boundary. |
| NFR-3 | Idempotency for privileged commands: duplicate submissions of the same client-minted id are no-ops; the outbox may retry freely without double-approve / double-redeem / double-credit. | PRD NFR-3 analog. The whole offline outbox depends on retry-safety; depends on the additive contract id for `RedeemItem`/`AdjustPoints`. |
| NFR-5 | The Knight holds a **per-user Knight token** (not a shared parent secret): a tenant-scoped bearer token issued to the parent's account (by Identity, SQUIRE-S-0007; A-0004), presented on every API call, so neither another LAN device nor any Squire app can perform Knight actions. | PRD NFR-5 analog, A-0004. Per-user accounts and roles (Knight/Squire) replace shared per-role secrets; the parent surface is gated by the Knight role. |
| NFR-6 | Communication is local HTTP over the home LAN; the Knight tolerates a changing/absent Keep address and degrades to cache + outbox rather than failing. | PRD NFR-6. LAN-only, no cloud relay; reachability is intermittent by design. |

## Architecture Framing **[CONDITIONAL: System-Level Spec]**

### Decision Area: Idempotency-id contract extension for `RedeemItem` / `AdjustPoints` — RESOLVED
- **Context**: Reviews are naturally idempotent (they key on `claim_id`/`request_id`, engine rejects double-review via `AlreadyReviewed`), and mark-done keys on a minted `claim_id`. But `RedeemItem` and `AdjustPoints` carried **no client id** — a blind outbox retry would double-spend or double-credit.
- **Decision**: A client-minted `CommandId` is carried **onto the emitted event**. The Knight mints it at enqueue time and reuses it on every retry; the Keep dedupes by finding an event already carrying that id (no processed-commands side table — dedup stays derived from the log). Rejected: a command envelope + side table (breaks AR-3). **Applied additively to `shared_contract.rs`** (`RedeemItem`/`AdjustPoints` gain `command_id`; `ItemRedeemed.command_id: Option<CommandId>`, `PointsAdjusted.command_id: CommandId`).
- **ADR**: SQUIRE-A-0001 (decided; shared with SQUIRE-S-0003)

### Decision Area: Parent state-read shape (cross-Squire `HouseholdReview`)
- **Context**: The Knight needs pending claims + pending redemption requests across **all** Squires (each labeled with its Squire) plus per-Squire balances. A-0005 resolves the shape to the cross-Squire **`HouseholdReview`** (distinct from a single Squire's `StateView`); the open question is the endpoint/wiring on the parent surface.
- **Constraints**: Must render fully from cache offline (REQ-K1/REQ-K10); the read is Knight-credentialed (per-user token; A-0004) and spans every Squire; whatever shape is chosen must be cacheable in Room and reconcilable after a flush (REQ-K9). Decision is **shared with the Local API spec (SQUIRE-S-0003)**, which produces the surface.
- **Required Capabilities**: deliver pending claims + pending redemption requests + balance in one cacheable payload; support resolving outbox items against a refreshed read.
- **ADR**: TBD

### Decision Area: Room schema for review-queue cache + privileged outbox, and retry/backoff
- **Context**: How the last parent state read (review queue + balance) and the pending privileged commands are persisted in Room, and how/when the outbox is drained — retry cadence, backoff, ordering, and terminal-state handling once a server read resolves an item.
- **Constraints**: Room/SQLite is fixed (AR-6); the cache must support full offline render (REQ-K1); the outbox must be durable across restarts (REQ-K7) and safe to retry because privileged commands are idempotent on client-minted ids (REQ-K8/NFR-3) — contingent on the contract extension above.
- **Required Capabilities**: store/read back the parent state payload; enqueue/dequeue privileged commands with their minted ids and state; retry on reachability with bounded backoff; mark items resolved from a refreshed read (REQ-K9).
- **ADR**: TBD

### Decision Area: Establishing and presenting the per-user Knight token
- **Context**: How the **per-user Knight token** is obtained — pairing binds the device to the parent's account and yields a tenant-scoped bearer token (issuance owned by Identity, SQUIRE-S-0007; A-0004) — stored on the phone, and presented on every parent-API call.
- **Constraints**: Per-user account with the Knight role, not a shared parent secret (NFR-5, A-0004); LAN-only (NFR-6). The pairing/auth mechanism is **shared with the Local API spec (SQUIRE-S-0003)** and **Identity (SQUIRE-S-0007)**, which enforce the role gate server-side.
- **Required Capabilities**: a one-time pairing flow yielding a per-user Knight token; secure on-device storage; attach the token to every request; degrade gracefully when the Keep is unreachable (cache + outbox still work).
- **ADR**: TBD

## Constraints **[CONDITIONAL: Has Constraints]**

### Technical Constraints
- Native Android: Kotlin + Jetpack Compose, with Room (SQLite) for the cache and privileged outbox — fixed by AR-6, not an implementation choice.
- Quick-actions only — no authoring, no member management. The Knight issues only `ReviewClaim`, `ReviewRedemption`, `RedeemItem`, `AdjustPoints`, and mark-done (mint `claim_id` → claim+approve). `RedeemItem`/`AdjustPoints` carry the target `squire`; review actions derive the Squire from the claim/request (A-0005). Define/Archive of quests/items/achievements **and adding Knights/Squires** stay on the Keep (SQUIRE-S-0004) for MVP.
- Pure parent-API consumer: the Knight talks only to the parent surface of the Local API (SQUIRE-S-0003) and **never writes the store** — the Keep validates every command through `Engine::handle` and is the sole writer (AR-1).
- LAN-only, no cloud relay: communication is local HTTP over the home LAN; degrade to cache + outbox when the Keep is unreachable (NFR-6).
- Authenticates with a **per-user Knight token** (not a shared parent secret): a tenant-scoped bearer token issued to the parent's account (Identity, SQUIRE-S-0007; A-0004), presented on every call so neither another LAN device nor any Squire app can perform Knight actions (NFR-5).
- Privileged commands must be idempotent (retry-safe outbox): no double-approve / double-redeem / double-credit (NFR-3).
- Uses the **additive contract extension** giving `RedeemItem`/`AdjustPoints` a client-minted `CommandId` carried onto their events — decided in ADR SQUIRE-A-0001 and already applied to `shared_contract.rs` (shared with SQUIRE-S-0003).