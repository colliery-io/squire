---
id: knight-app-knight-core-privileged
level: task
title: "Knight app: :knight-core privileged outbox + ack-based sync engine + HouseholdReview cache (JVM-tested)"
short_code: "SQUIRE-T-0039"
created_at: 2026-06-17T16:58:52.860702+00:00
updated_at: 2026-06-17T17:09:47.754347+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Knight app: :knight-core privileged outbox + ack-based sync engine + HouseholdReview cache (JVM-tested)

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0006]] (REQ-K1/K7/K8/K9/K10, NFR-1/3) · blocked_by [[SQUIRE-T-0038]] · sibling of [[SQUIRE-T-0032]]

## Objective

Build the Knight's pure-Kotlin/JVM offline engine — the parent analog of the Squire `:core`. The Knight's command set is **privileged quick-actions** (`ReviewClaim`, `ReviewRedemption`, `RedeemItem`, `AdjustPoints`, mark-done) and its read is the cross-Squire `HouseholdReview`. Crucially the sync model differs from the Squire's reconcile-by-id-in-state: privileged commands are **synchronous and terminal**, so an outbox item resolves on its **ack** (HTTP 2xx, or a benign terminal `AlreadyReviewed`/409 = someone already did it), not by polling refreshed state. Keep it pure (no Android), JVM-tested with fakes.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] A new `:knight-core` Gradle module (pure Kotlin/JVM, depends on `:sdk`) added to `settings.gradle.kts`.
- [x] A `PrivilegedOutbox` of `KnightCommand` variants (ReviewClaim/ReviewRedemption/Redeem/Adjust/MarkDone), each carrying its client-minted idempotency id (`claim_id`/`request_id`/`command_id`); idempotent `enqueue` keyed by a composite `kind:id` (REQ-K8/NFR-3), `pending()`, `markResolved(keys)`.
- [x] An ack-based `KnightSyncEngine`: for each pending command submit via `KnightSubmit`; on **Ack**(2xx) or **AlreadyDone**(409/403 occurrence-taken) mark resolved; on **Offline** stop and report `Offline` (outbox intact); on **Poison**(other 4xx) drop with an `onPoison` callback so it can't retry forever. Never throws (REQ-K10/NFR-1).
- [x] A `ReviewCache` (save/load the last `HouseholdReview` JSON) and a `KnightStore` presentation store exposing a `KnightUiState` (Loading / Ready(review, fromCache) / Error) with offline fallback to cache; quick-action methods mint ids, enqueue, sync, refetch — never throw (`adjust` requires a non-blank reason).
- [x] JVM unit tests (fakes) cover: idempotent enqueue, cross-kind no-collision, ack-resolves, already-done-resolves, offline-stops-drain, poison-drops, offline-render-from-cache, error-when-no-cache, quick-action paths. `./gradlew :knight-core:test` green (12 tests).

## Implementation Notes

### Technical Approach
Mirror `:core`'s structure (`Ports.kt`/`Outbox.kt`/`StateCache.kt`/`SyncEngine.kt`/`PlayerStore.kt`) but for the privileged surface. Outbound port surfaces typed results so the engine can distinguish ack / terminal-benign / network-fail / poison — model it as a `KnightSubmit` port returning a sealed `SubmitResult` (the `:app` adapter maps okhttp/`ClientException` status codes onto it, keeping `:knight-core` free of okhttp). `KnightCommand` wraps the SDK request DTOs (`ReviewClaimReq`, `ReviewRedemptionReq`, `RedeemReq`, `AdjustReq`, `MarkDoneReq`) so no hand-rolled JSON.

### Dependencies
[[SQUIRE-T-0038]] (decodable `DecisionDto`), the generated `:sdk` (`HouseholdReview`, request DTOs), and the existing Gradle multi-module setup from [[SQUIRE-T-0032]].

### Risk Considerations
The ack-based reconcile is deliberately simpler than the Squire's id-in-state reconcile — the risk is a command that 2xx-acks but a later refetch doesn't reflect; acceptable because the privileged commands are server-idempotent and terminal (the engine commits before acking). Mark-done is two server commands behind one ack — the minted `claim_id` keeps a retry idempotent (submit no-ops, re-approve → 409 = resolved).

## Status Updates

**2026-06-17 — Done.** New `:knight-core` pure-Kotlin/JVM module (`build.gradle.kts` mirrors `:core`; depends on `:sdk` + coroutines, serializes SDK types so no kotlinx plugin needed). Files under `com.squire.knight.core`: `KnightCommand` (sealed, wraps the 5 SDK request DTOs; composite `kind:id` key so cross-kind id reuse can't collide), `PrivilegedOutbox` (+`InMemory`), `Ports` (`KnightSubmit`→`SubmitResult{Ack,AlreadyDone,Offline,Poison}`, `ReviewFetcher`), `ReviewCache` (+`InMemory`), `KnightSyncEngine` (ack-based drain with `onPoison` logger + `SyncOutcome{Synced(acked,dropped,remaining),Offline}`), `KnightStore` (offline-first `refresh`, `syncNow`, and the 7 quick-actions — approve/reject claim, approve/reject request, redeem, adjust [reason-required], markDone). 12 JVM tests across 2 files, all green; `:core` still green after the SDK regen. Key design note recorded in the task: the Knight resolves by **ack**, not by id-in-state (privileged commands commit before acking) — simpler and correct because every command is server-idempotent on its minted id.