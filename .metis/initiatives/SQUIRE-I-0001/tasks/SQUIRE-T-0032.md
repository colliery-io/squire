---
id: squire-app-gradle-multi-module
level: task
title: "Squire app: Gradle multi-module scaffold + :core offline sync/outbox engine (JVM-tested)"
short_code: "SQUIRE-T-0032"
created_at: 2026-06-17T12:54:01.124283+00:00
updated_at: 2026-06-17T12:55:17.193964+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


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

- [ ] A Gradle project (under `clients/squire-android/`) with the committed wrapper, a version catalog, and `local.properties` (sdk.dir, gitignored) — builds with `./gradlew` against the installed SDK (Android 34, JDK 17).
- [ ] `:sdk` module generates the Kotlin client from `crates/api/openapi.json` (openapi-generator-gradle-plugin) and **compiles** in-build — proving the contract→SDK pipeline runs under Gradle (not just the standalone script).
- [ ] `:core` (pure `kotlin("jvm")`, depends on `:sdk` only for the clean request DTOs `SubmitClaimReq`/`RequestRedemptionReq`): an idempotent `Outbox` (dedupe by phone-minted id), a `StateCache` abstraction, a `SyncEngine` implementing **flush-then-refetch** with reconciliation by id, and graceful "computer unreachable" handling (fall back to cache + keep queuing). Insulated from the mangled data-enums (see risk).
- [ ] `:core` JVM tests (kotlin-test + coroutines-test) pass via `./gradlew :core:test`: outbox idempotency (same id enqueued twice → one pending), flush posts pending + reconciles resolved items out, offline keeps items pending without error, reconciliation marks an item resolved iff its id appears in refreshed state.
- [ ] `:app` minimal Android module (compiles against android-34, depends on `:core`/`:sdk`) — a skeleton entry point; full Compose UI is a later task. Best-effort `:app` compile; **`:core:test` green is the bar.**
- [ ] The Rust workspace is untouched and still green; the new project does not interfere with `cargo test --workspace`.

## Implementation Notes

### Technical Approach
Version matrix (JDK 17): Gradle 8.11.1 (installed), AGP 8.5.x, Kotlin 2.0.20, kotlinx-serialization 1.7.x, coroutines 1.9.x, okhttp 4.12, openapi-generator-gradle-plugin 7.11. `:sdk` runs `openApiGenerate` (generator `kotlin`, library `jvm-okhttp4`, `kotlinx_serialization`) from `../../crates/api/openapi.json` into its build dir and adds it as a source set. `:core` is transport-agnostic: ports (`SubmissionApi`, `StatePort` yielding resolved claim/request id sets) injected, fakes in tests. Source env via `clients/android-env.sh`.

### Dependencies
[[SQUIRE-T-0031]] (openapi.json) + the installed toolchain (Android SDK + Gradle, this session). Spec REQ-SY1–SY5, NFR-1/3/6.

### Risk Considerations
**openapi-generator mangles externally-tagged enums** (`ClaimState`/`RedemptionState`/`LockReason` → merged data classes that drop variants and won't round-trip the real JSON). It does NOT affect `:core` (ids only). **Follow-up (separate task):** make these enums generator-friendly at the schema level (e.g. an adjacently/internally-tagged representation) OR hand-map just these few types — needed before the UI deserializes `GET /state`. Tracked as a risk to resolve before `:app` parses `StateView`.