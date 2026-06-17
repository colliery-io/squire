---
id: admin-app-the-keep
level: specification
title: "Admin App (the Keep)"
short_code: "SQUIRE-S-0004"
created_at: 2026-06-17T00:52:29.887561+00:00
updated_at: 2026-06-17T00:52:29.887561+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# Admin App (the Keep)

## Overview **[REQUIRED]**

The Keep is the parent/admin application that runs on the computer — the household's single source of truth and **only writer**. It is the **full** admin control surface and the home of the complete privileged `Command` set: `DefineQuest`/`ArchiveQuest`, `DefineItem`/`ArchiveItem`, `DefineAchievement`/`ArchiveAchievement`, `ReviewClaim{decision}`, `ReviewRedemption{decision}`, `RedeemItem` (direct), and `AdjustPoints`. Authoring (define/archive) originates **only** here. The review/redeem/adjust subset is *also* offered to the parent over the network as quick-actions via the Knight (SQUIRE-S-0006), but those still commit through this same Keep — the Knight submits, the Keep writes.

The Keep is **in-process and engine-direct**: it constructs a `Command`, calls `Engine::handle(&snapshot, cmd, &clock)` directly, and applies the resulting `Vec<Change>` through the single-writer `Repository::apply` (FR-ADM4). It reads the same `Snapshot` the network API reads, but it never routes its own operations through that API. Authoring is not network-reachable in any role (AR-8 retained); the quick-action subset is network-reachable only under the parent credential (AR-8 revised). Either way the Keep is the single writer (AR-1).

Beyond quests, the Keep also **seeds and administers the household's members** — adding Knights (adults/parents) and Squires (children/players) and minting their tenant-scoped tokens via the Identity component (SQUIRE-S-0007). Authoring here means authoring *members* as well as quests/items/achievements. Reviewing is now **cross-Squire**: the Keep works one queue covering every Squire's pending claims and requests, each labeled with its Squire. The Keep runs the **local single-tenant** household (MVP: a single SQLite-backed tenant; A-0002).

This spec owns the parent **workflows and UX** that invoke the rules; the rules themselves (validation, snapshotting, derivations) live in the Domain Core and are out of scope here. Per-user accounts/roles and token issuance follow A-0004; the per-Squire domain and cross-Squire `HouseholdReview` follow A-0005.

## System Context **[CONDITIONAL: System-Level Spec]**

### Actors
- **Parent / Admin**: The human operator, working on the computer, keyboard-driven. Authors definitions, works the review queue, redeems directly, and makes manual adjustments. The same human may also perform the review/redeem/adjust subset from the Knight phone (SQUIRE-S-0006); authoring is exclusive to this surface.

### External Systems
- **Engine (Domain Core)**: Called in-process via `Engine::handle`. Validates every `Command` and returns `Vec<Change>`. The Keep is its in-process driver for privileged commands.
- **Repository (in-process)**: The single-writer store. The Keep applies the engine's `Change`s via `Repository::apply` and reads state via `Repository::snapshot` — the same `Snapshot` the network API reads.

### Boundaries
- **Inside**: authoring forms (quests, items, achievements); **member administration** (adding Knights & Squires and minting their tokens via Identity, SQUIRE-S-0007; A-0004); the batched **cross-Squire** review queue; approve/reject of claims and redemption requests across all Squires; direct redeem; manual point adjustment; the raw event-log inspector. Runs the local single-tenant household (SQLite; A-0002).
- **Outside**: the business rules themselves (Domain Core / `Engine`); persistence (the Store behind the `Repository`); the network API and child-facing surfaces (Player app). The Keep invokes the rules but does not define them, and never crosses the network.

## Requirements **[REQUIRED]**

### Functional Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| REQ-1.1.1 | Authoring — Quest: create/edit/archive a Quest with all §6 fields (title, description, free-form category at definition time, reward, cadence, auto_approve, repeatable_within_day, icon), plus its **`assignment`** (which Squires — `AllSquires` or an explicit subset) and **completion mode** (`EachAssignee` "every assignee does their own" vs `Race` "first approved wins"). Submit as `DefineQuest`; archive as `ArchiveQuest` — never delete. | PRD FR-A1, FR-A4; SQUIRE-A-0005 (quest assignment + completion mode). Archived quests stay referenced by historical events (AR-2); categories are typed inline, no separate management screen. |
| REQ-1.1.2 | Authoring — RedeemableItem: create/edit/archive an item with cost, optional achievement `gate`, and an `availability` of just **`Once`** (a toy — out of stock after its single redemption) or **`Repeatable`** (no limit; surfaces the last redemption) — **no** per-day/per-week/total fields. Submit as `DefineItem`; archive as `ArchiveItem`. | PRD FR-A2; A-0006 (`Availability { Once, Repeatable }`, no rate-limit math; out-of-stock and `last_redeemed` derived from the log). |
| REQ-1.1.3 | Authoring — Achievement: create/edit/archive an achievement, choosing criterion, scope, and bonus points. Submit as `DefineAchievement`; archive as `ArchiveAchievement`. | PRD FR-A3. |
| REQ-1.1.4 | Reward edits are forward-only in the UX: editing a quest's reward affects only future approvals; the Keep surfaces no path to rewrite past payouts. | PRD FR-A5, AR-4. Points are snapshotted onto `CompletionApproved`. |
| REQ-1.2.1 | Batched review queue: a single screen listing all pending `CompletionClaimed` and `RedemptionRequested` items, each approvable/rejectable in place. | PRD FR-ADM1, NFR-12. One queue keeps routine parent effort minimal. |
| REQ-1.2.2 | Review claim: approve or reject any pending claim via `ReviewClaim{decision}`. Approve → `CompletionApproved` with the quest's current reward snapshotted; reject → `CompletionRejected` with optional reason. The Keep stamps the acting Knight as `actor` (per-user audit; SQUIRE-A-0005). | PRD FR-C4. |
| REQ-1.2.3 | Review redemption request: approve or reject via `ReviewRedemption{decision}`. Approve → `ItemRedeemed` carrying the originating `request_id`; reject → `RedemptionRejected` with optional reason. The Keep stamps the acting Knight as `actor` (SQUIRE-A-0005). | PRD FR-R2. |
| REQ-1.3.1 | Direct redeem: redeem an item with no prior request via `RedeemItem{item_id}` → `ItemRedeemed` with no `request_id`. The Keep stamps the acting Knight as `actor` (SQUIRE-A-0005). | PRD FR-R3, FR-ADM3. |
| REQ-1.3.2 | Manual point adjustment: apply a signed adjustment via `AdjustPoints{amount, reason}`; the reason is required and the UI must enforce it before submit. The Keep stamps the acting Knight as `actor` (SQUIRE-A-0005). | PRD FR-P2, FR-ADM3. `PointsAdjusted.reason` is non-optional. |
| REQ-1.4.1 | Event-log inspector: list the raw `Event`s pertaining to a given quest or item, in order, to explain how a balance or streak was reached, including **who** (the `actor`) committed each Knight-committed fact. The definition inspector also surfaces each definition's **last-editor metadata** (`created_by`/`updated_by` + timestamps — "who set / last changed this") and, for a reward, its **`last_redeemed`** (and out-of-stock for a redeemed `Once`). Read-only over the `Snapshot`. | PRD NFR-11; SQUIRE-A-0005 (Knight-actor audit); A-0007 (last-editor metadata); A-0006 (`last_redeemed`, derived). |
| REQ-1.4.2 | Authoring audit: when the Keep commits any authoring `Change` (`DefineQuest`/`DefineItem`/`DefineAchievement`/`Archive…`/`PutUser`/`SetUserActive`), it passes `by` = the acting Knight to `Repository::apply(by, changes)` so the store stamps last-editor metadata; the authoring forms surface "who set / last changed this" for the definition being edited. Domain types stay pure — audit is store-layer only. | A-0007 — definition audit via `apply(by, …)` + last-editor columns; pure domain types (AR-7). |

### Non-Functional Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| NFR-1.1.1 | Observability: the Keep can render the raw, ordered event log scoped to a single quest or item so the parent can trace exactly how a balance or streak was reached, and surfaces each definition's last-editor metadata ("who set / last changed this") and a reward's `last_redeemed`, so "who set the trash reward to 15?" and "when was this last redeemed?" are answerable. | PRD NFR-11; A-0007 (last-editor metadata); A-0006 (`last_redeemed`). Derived values (balance, streaks, `last_redeemed`, out-of-stock) must be explainable from facts. |
| NFR-1.1.2 | Low review friction: auto-approve (handled by the engine) plus the single batched review queue keep routine parent effort minimal; no per-item navigation to approve. | PRD NFR-12. |
| NFR-1.1.3 | Integrity (producer side): **authoring** (define/archive) is local-only and never exposed over the network in any role; the review/redeem/adjust subset is exposed only under the separate parent credential (the Knight), and even then commits through this Keep — the single writer. The **child** phone remains structurally incapable of minting points, approving, or redeeming; the authoritative balance exists only here, derived from the log. | PRD AR-8 (retained for authoring, revised for the quick-action subset), NFR-2 / AR-1 preserved. |

## Architecture Framing **[CONDITIONAL: System-Level Spec]**

### Decision Area: Admin UI form factor
- **Context**: The Keep needs a parent-facing UI, but the form factor is an open implementation choice (PRD §10): CLI, TUI, or a local web page served by the same process. This must be decided before the Keep is built out.
- **Constraints**: Must talk to the engine **directly** (in-process `Engine::handle` + `Repository::apply`) and never through the network API (FR-ADM4, AR-8). Runs on the computer in Rust. Shares the single-writer `Repository` (AR-1) — no second writer. Keyboard-driven; fast authoring is a primary goal.
- **Required Capabilities**: Authoring forms for all three definition types; a batched, in-place-actionable review queue; direct redeem and reason-required adjustment; a read-only event-log inspector. If a local web page is chosen, it must be served by the same process and still invoke the engine in-process (not loop back through the network API).
- **ADR**: TBD

## Constraints **[CONDITIONAL: Has Constraints]**

### Technical Constraints
- **Engine-direct only**: privileged commands are issued in-process via `Engine::handle` and committed via `Repository::apply`; the Keep never reaches the network API for any operation (FR-ADM4, AR-8).
- **Archive never deletes**: definition removal is `ArchiveQuest`/`ArchiveItem`/`ArchiveAchievement` (a `SetXActive(_, false)` change), so historical events remain valid (AR-2).
- **Adjustments require a reason**: `AdjustPoints` carries a non-optional `reason` (`PointsAdjusted.reason: String`); the UI enforces it before submit (FR-P2).
- **Runs on the computer in Rust**: same host and language as the engine and store (AR-6).
- **Single-writer Repository**: the Keep shares the one writer; no concurrent second writer exists (AR-1).