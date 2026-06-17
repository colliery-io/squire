---
id: squire-app-gradle-multi-module
level: task
title: "Squire app: Gradle multi-module scaffold + :core offline sync/outbox engine (JVM-tested)"
short_code: "SQUIRE-T-0032"
created_at: 2026-06-17T12:54:01.124283+00:00
updated_at: 2026-06-17T13:02:38.835998+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Squire app: Gradle multi-module scaffold + :core offline sync/outbox engine (JVM-tested)

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0005]] (the Squire phone) · Contract: [[SQUIRE-A-0009]] / [[SQUIRE-T-0031]]

## Objective

Scaffold the **Gradle multi-module Android project** for the Squire phone and build the pure-Kotlin/JVM **`:core`** module — the offline-first sync engine + idempotent outbox + reconciliation (REQ-SY1–SY5) — with passing JVM unit tests (no emulator). Establishes the iterative-dev substrate; the Compose UI + Room land in later tasks.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] A Gradle project (under `clients/squire-android/`) with the committed wrapper, a version catalog, and `local.properties` (sdk.dir, gitignored) — builds with `./gradlew` against the installed SDK (Android 34, JDK 17).
- [x] `:sdk` module generates the Kotlin client from `crates/api/openapi.json` (openapi-generator-gradle-plugin) and **compiles** in-build (`:sdk:compileKotlin` BUILD SUCCESSFUL).
- [x] `:core` (pure `kotlin("jvm")`, depends on `:sdk` only for `SubmitClaimReq`/`RequestRedemptionReq`): idempotent `Outbox` (id-keyed `LinkedHashMap`), `StateCache`, `SyncEngine` (flush-then-refetch, reconcile-by-id, unreachable → `Offline` with the outbox intact). Insulated from the mangled data-enums.
- [x] `:core` JVM tests pass via `./gradlew :core:test` — **5 tests, 0 failures**: outbox idempotency; flush posts + reconciles out; offline keeps items pending without throwing; reconciliation resolves iff the id is in refreshed state; cache round-trip.
- [x] `:app` minimal Android module (Activity + TextView) compiles + `assembleDebug` produces a debug APK against android-34; full Compose UI deferred.
- [x] Rust workspace untouched (no `crates/` changes) and still green.

## Implementation Notes

### Technical Approach
Version matrix (JDK 17): Gradle 8.11.1 (installed), AGP 8.5.x, Kotlin 2.0.20, kotlinx-serialization 1.7.x, coroutines 1.9.x, okhttp 4.12, openapi-generator-gradle-plugin 7.11. `:sdk` runs `openApiGenerate` (generator `kotlin`, library `jvm-okhttp4`, `kotlinx_serialization`) from `../../crates/api/openapi.json` into its build dir and adds it as a source set. `:core` is transport-agnostic: ports (`SubmissionApi`, `StatePort` yielding resolved claim/request id sets) injected, fakes in tests. Source env via `clients/android-env.sh`.

### Dependencies
[[SQUIRE-T-0031]] (openapi.json) + the installed toolchain (Android SDK + Gradle, this session). Spec REQ-SY1–SY5, NFR-1/3/6.

### Risk Considerations
**openapi-generator mangles externally-tagged enums** (`ClaimState`/`RedemptionState`/`LockReason` → merged data classes that drop variants and won't round-trip the real JSON). It does NOT affect `:core` (ids only). **Follow-up (separate task):** make these enums generator-friendly at the schema level (e.g. an adjacently/internally-tagged representation) OR hand-map just these few types — needed before the UI deserializes `GET /state`. Tracked as a risk to resolve before `:app` parses `StateView`.

## Status Updates

**2026-06-17 — Done (`ff7b361`).** The Squire Android substrate + `:core` engine.

- **Toolchain (this session, out-of-band):** Android SDK at `~/Library/Android/sdk` (build-tools 34, platform-tools, android-34) + Gradle 8.11.1 at `~/.local/bin` + JDK 17; env in `~/.zshrc` + `clients/android-env.sh`.
- **`clients/squire-android/`** — Gradle 8.11.1 multi-module (wrapper committed; `local.properties`/build dirs gitignored). AGP 8.5.2, Kotlin 2.0.20, kotlinx-serialization 1.7.3, coroutines 1.9.0, okhttp 4.12, openapi-generator-gradle-plugin 7.11; minSdk 26 / sdk 34.
- **`:sdk`** generates the Kotlin client from `crates/api/openapi.json` at build (in-build pipeline proven).
- **`:core`** (pure JVM): `Outbox`/`InMemoryOutbox` (id-keyed, idempotent), `StateCache`, ports (`SubmissionApi`, `StatePort`/`ResolvedIds`), `SyncEngine` (flush-then-refetch, reconcile-by-id, unreachable → `Offline` outbox-intact). **5 JVM tests green** — verified independently (`./gradlew :core:test` = tests=5 failures=0).
- **`:app`** minimal Activity compiles + `assembleDebug` packages a debug APK.

**Iterative dev loop now live:** `source clients/android-env.sh && cd clients/squire-android && ./gradlew :core:test`.

**Known follow-up (filed):** openapi-generator mangles the externally-tagged enums (`ClaimState`/`RedemptionState`/`LockReason`) — fine for `:core` (ids only), but must be fixed before the UI deserializes `GET /state`. See the backlog item.