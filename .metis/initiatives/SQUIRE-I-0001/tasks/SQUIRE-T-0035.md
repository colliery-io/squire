---
id: runnable-lan-api-server-binary
level: task
title: "Runnable LAN api server binary + Squire app minimal connect (login) for end-to-end demo"
short_code: "SQUIRE-T-0035"
created_at: 2026-06-17T13:49:42.673669+00:00
updated_at: 2026-06-17T13:50:27.780097+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


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

- [ ] An `api` server entrypoint (a `src/bin/serve.rs` or `examples/serve_demo.rs`) wires `AppState::local_prod` and serves on a configurable `0.0.0.0:PORT`. For the demo it **seeds deterministically**: register household `demo` (admin Knight = `UserId(1)`, secret `demo`), add a Squire (`UserId(2)`, secret `demo`), author 2–3 quests (incl. one `auto_approve=true` so the child immediately sees points) + a reward (direct store `apply`), and prints the base URL + the Squire's `(household, user, secret)`.
- [ ] `cargo run -p api --bin serve` (or the example) boots and answers `GET /state` for the seeded Squire over the wire (verified with a `login` → token → `curl /state`).
- [ ] **Squire app minimal connect:** the app (`MainActivity`/a small `Connect` step) logs in via the generated `ControlApi.login(household, user, secret)` to obtain a tenant token, then drives `PlayerStore` — so it self-connects with no baked/expiring token. Demo config (`http://10.0.2.2:PORT`, `demo`/`2`/`demo`) is clearly marked as placeholder; the full pairing UI + secure storage remain a later task.
- [ ] `./gradlew :app:assembleDebug` builds; `cargo test --workspace` stays green; the seed path uses only existing public APIs (no new wire endpoints).

## Implementation Notes

### Technical Approach
Server: reuse `api::serve` + `AppState::local_prod` (SQLite temp/fixed dir) + `state.identity.register`/`add_member` + direct `store.apply` for authoring (authoring isn't on the wire). App: add a `suspend fun login()` to `SquireApiAdapter` (or a `Connector`) calling the generated `ControlApi`; `MainActivity` logs in once then refreshes. Keep it a thin demo seam.

### Dependencies
[[SQUIRE-T-0034]] (the app), [[SQUIRE-T-0031]] (SDK). The api `serve`/`AppState::local_prod` already exist.

### Risk Considerations / deferred
This is demo/bootstrap convenience + the first connect slice — **not** the full pairing flow (QR/secure token storage, NFR-5) which stays a later task. Keep the demo seeding clearly separated from production wiring.

## Status Updates

*To be added during implementation*