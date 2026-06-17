---
id: knight-app-knight-core-privileged
level: task
title: "Knight app: :knight-core privileged outbox + ack-based sync engine + HouseholdReview cache (JVM-tested)"
short_code: "SQUIRE-T-0039"
created_at: 2026-06-17T16:58:52.860702+00:00
updated_at: 2026-06-17T16:58:52.860702+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Knight app: :knight-core privileged outbox + ack-based sync engine + HouseholdReview cache (JVM-tested)

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0006]] (REQ-K1/K7/K8/K9/K10, NFR-1/3) · blocked_by [[SQUIRE-T-0038]] · sibling of [[SQUIRE-T-0032]]

## Objective

Build the Knight's pure-Kotlin/JVM offline engine — the parent analog of the Squire `:core`. The Knight's command set is **privileged quick-actions** (`ReviewClaim`, `ReviewRedemption`, `RedeemItem`, `AdjustPoints`, mark-done) and its read is the cross-Squire `HouseholdReview`. Crucially the sync model differs from the Squire's reconcile-by-id-in-state: privileged commands are **synchronous and terminal**, so an outbox item resolves on its **ack** (HTTP 2xx, or a benign terminal `AlreadyReviewed`/409 = someone already did it), not by polling refreshed state. Keep it pure (no Android), JVM-tested with fakes.

## Acceptance Criteria

- [ ] A new `:knight-core` Gradle module (pure Kotlin/JVM, depends on `:sdk`) added to `settings.gradle.kts`.
- [ ] A `PrivilegedOutbox` of `KnightCommand` variants (ReviewClaim/ReviewRedemption/Redeem/Adjust/MarkDone), each carrying its client-minted idempotency id (`claim_id`/`request_id`/`command_id`); idempotent `enqueue` by id (REQ-K8/NFR-3), `pending()`, `markResolved(ids)`.
- [ ] An ack-based `KnightSyncEngine`: for each pending command POST via the outbound port; on **2xx** or **terminal-benign** (`AlreadyReviewed`/409, `OccurrenceTaken`/403) mark resolved; on **network failure** stop and report `Offline` (outbox intact); on other **4xx** (poison, e.g. blank reason/404) drop with a logged terminal outcome so it can't retry forever. Never throws (REQ-K10/NFR-1).
- [ ] A `ReviewCache` (save/load the last `HouseholdReview` JSON) and a `KnightStore` presentation store exposing a `KnightUiState` (Loading / Ready(review, fromCache) / Error) with offline fallback to cache; quick-action methods mint ids, enqueue, sync, refetch — never throw.
- [ ] JVM unit tests (fakes) cover: idempotent enqueue, ack-resolves-on-2xx, 409-resolves, offline-keeps-outbox, poison-drops, and offline-render-from-cache. `./gradlew :knight-core:test` green.

## Implementation Notes

### Technical Approach
Mirror `:core`'s structure (`Ports.kt`/`Outbox.kt`/`StateCache.kt`/`SyncEngine.kt`/`PlayerStore.kt`) but for the privileged surface. Outbound port surfaces typed results so the engine can distinguish ack / terminal-benign / network-fail / poison — model it as a `KnightSubmit` port returning a sealed `SubmitResult` (the `:app` adapter maps okhttp/`ClientException` status codes onto it, keeping `:knight-core` free of okhttp). `KnightCommand` wraps the SDK request DTOs (`ReviewClaimReq`, `ReviewRedemptionReq`, `RedeemReq`, `AdjustReq`, `MarkDoneReq`) so no hand-rolled JSON.

### Dependencies
[[SQUIRE-T-0038]] (decodable `DecisionDto`), the generated `:sdk` (`HouseholdReview`, request DTOs), and the existing Gradle multi-module setup from [[SQUIRE-T-0032]].

### Risk Considerations
The ack-based reconcile is deliberately simpler than the Squire's id-in-state reconcile — the risk is a command that 2xx-acks but a later refetch doesn't reflect; acceptable because the privileged commands are server-idempotent and terminal (the engine commits before acking). Mark-done is two server commands behind one ack — the minted `claim_id` keeps a retry idempotent (submit no-ops, re-approve → 409 = resolved).

## Status Updates

*To be added during implementation*
