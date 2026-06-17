---
id: squire-app-durable-room
level: task
title: "Squire app: durable Room persistence for the StateView cache + outbox (offline-first)"
short_code: "SQUIRE-T-0037"
created_at: 2026-06-17T16:30:51.022168+00:00
updated_at: 2026-06-17T16:45:00.604025+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Squire app: durable Room persistence for the StateView cache + outbox (offline-first)

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0005]] (REQ-SY1/SY2, NFR-1) · builds on [[SQUIRE-T-0032]]/[[SQUIRE-T-0034]]

## Objective

Make the Squire app's offline-first guarantee real: persist the last `StateView` (cache) and the submission **outbox** in **Room (SQLite)** so they **survive app/process restart** (REQ-SY1/SY2). Today both are in-memory (`:core` `InMemoryStateCache`/`InMemoryOutbox`), so killing the app loses queued claims. The `:core` ports stay the contract; Room impls live in `:app`.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] A Room database in `:app` with: a single-row **state cache** table (the last `StateView` JSON blob) and an **outbox** table (rows = `{id, kind, payload_json}` keyed by the phone-minted id). DAOs for both. (`data/db/SquireDb.kt`)
- [x] `RoomStateCache` implements `:core`'s `StateCache` (save/load the blob); `RoomOutbox` implements `:core`'s `Outbox` (idempotent `enqueue` by id, `pending`, `markResolved`) — both backed by the DAOs. `MainActivity` wires these in place of the in-memory impls.
- [x] **Durable across restart**: a claim queued while the computer is unreachable, and the cached state, survive an app kill + relaunch; the outbox flushes when reachable again (verified live on the emulator: queue offline → kill → relaunch → still pending → comes back online → flushes/reconciles).
- [x] `:core` JVM tests stay green (ports unchanged); `./gradlew :app:assembleDebug` builds; `cargo test --workspace` unaffected. Room DAO correctness verified via the emulator demo (instrumented/Robolectric tests optional).

## Implementation Notes

### Technical Approach
Add Room (room-runtime/-ktx + the compiler via **KSP**, version-matched to Kotlin 2.0.20) to `:app`. Entities: `CachedStateEntity(id=0, json)` and `OutboxEntity(id, kind, payloadJson)`. `RoomOutbox.enqueue` uses `INSERT OR IGNORE` (idempotent by primary-key id); `pending()` reads all; `markResolved(ids)` deletes. `RoomStateCache` upserts/reads the single row. The `:core` `OutboxItem` ↔ `OutboxEntity` mapping serializes the `SubmitClaimReq`/`RequestRedemptionReq` payload via kotlinx-serialization. Keep `:core` pure (Room is `:app`-only).

### Dependencies
[[SQUIRE-T-0034]] (`PlayerStore`/ports), [[SQUIRE-T-0032]] (`:core` Outbox/Cache interfaces). The home server (`squire-home`) for the live offline/online demo.

### Risk Considerations
KSP/Room version matching (pin to Kotlin 2.0.20 → KSP 2.0.20-1.0.x, Room 2.6.x). Room DAOs aren't trivially JVM-unit-testable without Robolectric — rely on the `:core` tests for logic + the emulator for the durability demo. Migrations: a fresh schema (no prior versions) — `fallbackToDestructiveMigration` is fine for now.

## Status Updates

**2026-06-17 — Done.** Room layer landed in `:app`: `data/db/SquireDb.kt` (`CachedStateEntity` single-row blob + `OutboxEntity{id,kind,payloadJson}`, `CacheDao`/`OutboxDao`, `@Database` v1 with `fallbackToDestructiveMigration`), `data/RoomStateCache.kt` and `data/RoomOutbox.kt` (the latter `INSERT OR IGNORE` for idempotent enqueue-by-id), wired in `MainActivity` via `SquireDb.build(this)`.

Plumbed a "Sync" path so a submission queued in a *previous* session is delivered on reconnect, not just on the next manual submit: `PlayerStore.syncNow()` = `sync.sync()` then `refresh()`; `SquireViewModel.refresh()` and startup both call it.

**Bug found & fixed during the live demo:** an app that *started offline* never re-authenticated — `MainActivity`'s startup `login()` failed (no network), `tokenHolder` stayed empty, and the later flush POSTed with no `Authorization` header (rejected). Fixed in `SquireApiAdapter` with `ensureLoggedIn()` (lazy login when the token is empty) gating `fetchState`/`submitClaim`/`requestRedemption`/`resolvedIds`; if still offline, login throws and the caller degrades to the cache / leaves the row on the durable outbox.

**Live durability demo passed on the emulator (against `squire-home`):** online → state cached; offline → "Mark done" on *Make your bed* queued in Room `outbox` (`1781714313717|CLAIM`, verified via `run-as … sqlite3`); force-stop → row survived; relaunch offline → rendered from cache, row intact; reconnect → outbox drained to 0, server credited **balance=5**, claim `1781714313717` → `Approved (+5)`, *Ice cream* now affordable. `:core` JVM tests green; `:app:assembleDebug` builds.