---
id: runnable-lan-api-server-binary
level: task
title: "Runnable LAN api server binary + Squire app minimal connect (login) for end-to-end demo"
short_code: "SQUIRE-T-0035"
created_at: 2026-06-17T13:49:42.673669+00:00
updated_at: 2026-06-17T14:01:15.821937+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Runnable LAN api server binary + Squire app minimal connect (login) for end-to-end demo

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0003]] (LAN api) / [[SQUIRE-S-0005]] (Squire) · enables the end-to-end demo / eyeball

## Objective

Make the system **runnable end-to-end**: a LAN api server binary (the `api` crate has no `main` yet) that serves the Squire/Knight/control surfaces, plus a **minimal connect** in the Squire app (log in with creds → token) so it actually fetches real `StateView`. This unblocks an emulator "eyeball" and is the first slice of the deferred pairing work.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] `crates/api/src/bin/serve_demo.rs` wires `AppState::local_prod` and serves on `0.0.0.0:DEMO_PORT` (default 8080). Seeds deterministically: household `demo` (Knight `UserId(1)`), Squire `UserId(2)` secret `demo`, quest 100 `auto_approve=true` (+5), quest 101 review (+10), reward 200 (cost 3) via direct `store.apply`; prints creds + the `10.0.2.2` emulator hint.
- [x] `cargo run -p api --bin serve_demo` boots; `login`→token→`GET /state` returns a populated `StateView` over the wire (verified).
- [x] **Squire app connect:** `SquireApiAdapter.login()` via the generated `ControlApi` stores the token in a mutable holder set as the bearer; `MainActivity` logs in once then refreshes. **Also added INTERNET permission + `usesCleartextTraffic` to the manifest** (LAN HTTP, NFR-6) — without them the app couldn't reach the server.
- [x] `./gradlew :app:assembleDebug` builds; `cargo test --workspace` green (43 binaries). **Verified LIVE on an arm64 Android-34 emulator: login → populated home → Mark-done → +5 pts → reward becomes redeemable, end-to-end.**

## Implementation Notes

### Technical Approach
Server: reuse `api::serve` + `AppState::local_prod` (SQLite temp/fixed dir) + `state.identity.register`/`add_member` + direct `store.apply` for authoring (authoring isn't on the wire). App: add a `suspend fun login()` to `SquireApiAdapter` (or a `Connector`) calling the generated `ControlApi`; `MainActivity` logs in once then refreshes. Keep it a thin demo seam.

### Dependencies
[[SQUIRE-T-0034]] (the app), [[SQUIRE-T-0031]] (SDK). The api `serve`/`AppState::local_prod` already exist.

### Risk Considerations / deferred
This is demo/bootstrap convenience + the first connect slice — **not** the full pairing flow (QR/secure token storage, NFR-5) which stays a later task. Keep the demo seeding clearly separated from production wiring.

## Status Updates

**2026-06-17 — Done (`7cbe593`).** End-to-end runnable + verified live on an emulator.

- `crates/api/src/bin/serve_demo.rs` (the api had no runnable binary): seeds the `demo` household + Squire + quests/reward and serves the LAN api on `0.0.0.0:8080`.
- Squire app: `SquireApiAdapter.login()` (generated `ControlApi`) → token holder → bearer; `MainActivity` logs in then refreshes. **Manifest: added INTERNET + `usesCleartextTraffic`** (the missing pieces that were causing "computer unreachable").
- **Eyeball:** installed on an arm64 Android-34 emulator → the player home renders from real `StateView` (quests with Mark-done, rewards with affordability, streaks, claims/requests). Tapping Mark-done on the auto-approve quest credited **+5**, flipped it to "Done today", made *Ice cream* **redeemable**, and showed *"Make your bed — Approved (+5)"* in Recent claims. The whole tap→claim→outbox→sync→auto-approve→refresh loop works.

**Still running for the operator's review:** `serve_demo` on :8080, the Keep on :4920, and the `squire` emulator. Deferred (later tasks): durable Room persistence, real pairing UI + secure token storage, sync trigger.