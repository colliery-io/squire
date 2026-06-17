---
id: squire-app-player-home-ui-compose
level: task
title: "Squire app: player home UI (Compose) + presentation store over the SDK"
short_code: "SQUIRE-T-0034"
created_at: 2026-06-17T13:26:03.813760+00:00
updated_at: 2026-06-17T13:26:53.266900+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


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

- [ ] **Presentation store (`:core`, pure JVM, tested):** a `PlayerStore` exposing a `StateFlow<PlayerUiState>` (`Loading` / `Ready(view, fromCache)` / `Error`). `refresh()` fetches `StateView` (via a `StateFetcher` port), caches it (`StateCache`), and on failure falls back to the cached view flagged `fromCache=true` (never throws — NFR-1/6). `submitClaim(...)`/`requestRedemption(...)` mint an id, enqueue to the `Outbox`, and run a `SyncEngine` pass. JVM tests cover: refresh→Ready; refresh-offline→Ready(fromCache) from cache; submit enqueues + a sync reconciles.
- [ ] **Transport adapter (`:app`):** a `SquireApiAdapter` implementing `:core`'s `StateFetcher` + `SubmissionApi` + `StatePort` over the generated okhttp `SquireApi` (configured base URL + bearer token + `X-Household`). Reconciliation ids come from `StateView.myClaims/myRequests`.
- [ ] **Compose UI (`:app`):** a `PlayerHomeScreen` rendering balance, today's quests (`QuestCard` with `QuestStatus`; a "Mark done" action on Available quests → submit claim; `TakenByOther`/`Pending`/`CompletedToday` shown, not actionable), rewards (`RewardCard` affordable / lock reason / request action), streaks (current/best/next milestone), and recent claims/requests with their state. An offline banner when `fromCache`. A refresh action. Hosted by `MainActivity` via a thin `SquireViewModel` wrapping `PlayerStore`.
- [ ] `./gradlew :core:test` green (incl. the new `PlayerStore` tests) and `./gradlew :app:assembleDebug` builds a debug APK. Rust workspace untouched + still green.

## Implementation Notes

### Technical Approach
Compose via the Kotlin 2.0 Compose plugin (`org.jetbrains.kotlin.plugin.compose` 2.0.20) + Compose BOM (~2024.09) + material3 + lifecycle-viewmodel-compose; coroutines. Keep ALL logic in `:core`'s `PlayerStore` (JVM-tested with fakes; uses the now-clean generated `StateView`); `:app` is thin Compose + Android wiring. The okhttp `SquireApi` is configured with the base URL/token (pairing UI is a later task — inject a config for now).

### Dependencies
[[SQUIRE-T-0032]] (`:core` outbox/sync), [[SQUIRE-T-0033]] (clean SDK enums), [[SQUIRE-T-0031]] (contract). Spec REQ-PL1–5, REQ-SY1/4/5, NFR-1/6.

### Risk Considerations / deferred
Compose plugin/BOM version matching (pin to the matrix). **Deferred to follow-up tasks:** durable Room cache+outbox (currently `:core` in-memory impls), pairing + token storage (REQ-NFR-5 / NFR-5), background sync trigger, instrumented UI tests. Compose UI itself is not unit-tested here (no emulator) — the testable logic lives in `:core`'s `PlayerStore`.

## Status Updates

*To be added during implementation*