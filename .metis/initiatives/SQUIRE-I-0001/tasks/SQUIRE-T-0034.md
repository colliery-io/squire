---
id: squire-app-player-home-ui-compose
level: task
title: "Squire app: player home UI (Compose) + presentation store over the SDK"
short_code: "SQUIRE-T-0034"
created_at: 2026-06-17T13:26:03.813760+00:00
updated_at: 2026-06-17T13:34:37.675357+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Squire app: player home UI (Compose) + presentation store over the SDK

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0005]] · Contract SDK: [[SQUIRE-A-0009]] / [[SQUIRE-T-0031]] / [[SQUIRE-T-0033]] · Engine: [[SQUIRE-T-0032]]

## Objective

Build the Squire (child) **player home UI** in Jetpack Compose, driven by a pure-Kotlin **presentation store** (in `:core`) over the generated SDK and the `:core` sync/outbox engine: render the whole home from `StateView`, let the child submit a completion claim and a redemption request, and degrade to the cached view when the computer is unreachable (REQ-PL1–5, REQ-SY1/4/5).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] **Presentation store (`:core`, pure JVM, tested):** `PlayerStore` exposes `StateFlow<PlayerUiState>` (`Loading`/`Ready(view, fromCache)`/`Error`); offline-first `refresh()` (fetch→cache→Ready; on throw→cached→`Ready(fromCache=true)`; else `Error`; never throws); `submitClaim`/`requestRedemption` mint id → enqueue `Outbox` → `SyncEngine` → refresh. 5 new JVM tests (10 total green).
- [x] **Transport adapter (`:app`):** `SquireApiAdapter` implements `StateFetcher`+`SubmissionApi`+`StatePort` over the generated okhttp `SquireApi` (bearer + `X-Household` via interceptor/per-call; `Dispatchers.IO`); `resolvedIds` from `myClaims/myRequests`.
- [x] **Compose UI (`:app`):** `PlayerHomeScreen` (Material3) — balance, offline banner on `fromCache`, quests (Mark-done only on `Available`; Pending/Done/Taken labels), rewards (Redeem when affordable & unlocked, else lock/out-of-stock/can't-afford), streaks (current/best/next), recent claims/requests with state; refresh action; `@Preview`. `MainActivity` + `SquireViewModel` wire it.
- [x] `./gradlew :core:test` green (PlayerStore + SyncEngine) and `./gradlew :app:assembleDebug` builds the debug APK (~10 MB). Rust workspace untouched + green.

## Implementation Notes

### Technical Approach
Compose via the Kotlin 2.0 Compose plugin (`org.jetbrains.kotlin.plugin.compose` 2.0.20) + Compose BOM (~2024.09) + material3 + lifecycle-viewmodel-compose; coroutines. Keep ALL logic in `:core`'s `PlayerStore` (JVM-tested with fakes; uses the now-clean generated `StateView`); `:app` is thin Compose + Android wiring. The okhttp `SquireApi` is configured with the base URL/token (pairing UI is a later task — inject a config for now).

### Dependencies
[[SQUIRE-T-0032]] (`:core` outbox/sync), [[SQUIRE-T-0033]] (clean SDK enums), [[SQUIRE-T-0031]] (contract). Spec REQ-PL1–5, REQ-SY1/4/5, NFR-1/6.

### Risk Considerations / deferred
Compose plugin/BOM version matching (pin to the matrix). **Deferred to follow-up tasks:** durable Room cache+outbox (currently `:core` in-memory impls), pairing + token storage (REQ-NFR-5 / NFR-5), background sync trigger, instrumented UI tests. Compose UI itself is not unit-tested here (no emulator) — the testable logic lives in `:core`'s `PlayerStore`.

## Status Updates

**2026-06-17 — Done (`6061f15`).** The Squire player-home UI + presentation layer.

- **`:core` `PlayerStore`** (pure JVM): `StateFetcher` port, `PlayerUiState` (`Loading`/`Ready(view,fromCache)`/`Error`), offline-first `refresh()` (never throws), `submitClaim`/`requestRedemption` (mint id → `Outbox` → `SyncEngine` → refresh). 5 new tests; `:core:test` = 10 green.
- **`:app`**: `SquireApiAdapter` (the three `:core` ports over the generated okhttp `SquireApi`, bearer + `X-Household`), `SquireViewModel`, Material3 `PlayerHomeScreen` (balance, offline banner, quests with Mark-done on `Available`, rewards with redeem/lock, streaks, recent claims/requests), `MainActivity`. Compose plugin 2.0.20 + BOM 2024.09.03.
- **Verified independently:** `:core:test` BUILD SUCCESSFUL; `:app:assembleDebug` → ~10 MB debug APK; Rust untouched.

**Deferred (follow-up tasks):** durable Room cache+outbox (`:core` is in-memory now), pairing + secure token storage (placeholder config in `MainActivity`), background sync trigger, instrumented Compose UI tests. `DecisionDto` clean-up is the Knight's (S-0006).