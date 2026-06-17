---
id: keep-crate-scaffold-engine-direct
level: task
title: "Keep: crate scaffold, engine-direct command seam, loopback web server & operator login"
short_code: "SQUIRE-T-0025"
created_at: 2026-06-17T11:09:54.911758+00:00
updated_at: 2026-06-17T11:16:17.035430+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Keep: crate scaffold, engine-direct command seam, loopback web server & operator login

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0004]] (Admin App / the Keep) · ADR: [[SQUIRE-A-0008]]

## Objective

Stand up the `keep` binary crate: an embedded, **loopback-only** admin web server whose handlers drive the Domain Core **in-process** (the engine-direct command seam), plus operator (Knight) login for audit. No authoring/review features yet — this is the spine every later Keep task builds on, and the place the A-0008 invariants (loopback-only, no network-API loopback) are established and tested.

## Acceptance Criteria

## Acceptance Criteria

- [ ] New `keep` crate (binary) depends on `domain-core`, `store`, `identity` — and **NOT** on `api` for its command path (no network-API loopback; A-0008 / AR-8).
- [ ] `KeepState` holds the shared single-writer store (`Arc<Mutex<Store<SystemClock>>>`), the `DomainEngine`, the clock, and the `identity` port; a `commit(by, cmd)` helper does snapshot → `Engine::handle` → `Repository::apply(by, changes)`, maps domain errors → HTTP status, and returns the engine result.
- [ ] The admin HTTP server binds to **127.0.0.1 only** (loopback); a test asserts the bound address is loopback (never `0.0.0.0` / LAN).
- [ ] The web UI shell (HTML/CSS/JS) is **embedded in the binary** (e.g. `rust-embed`) and served at `/`; a health route returns 200.
- [ ] Operator login: a parent authenticates as a specific **Knight** via `identity` (login → session), and the acting Knight is available to handlers for `by`/`actor` stamping; unauthenticated admin actions are refused.
- [ ] Tests (oneshot, no socket): loopback-bind assertion; shell + health served; operator login round-trip; `commit` applies a change through the single writer.
- [ ] `cargo test --workspace` green and warning-free.

## Implementation Notes

### Technical Approach
axum server (reuse) over a `127.0.0.1:PORT` `TcpListener`; embed assets with `rust-embed` + a static handler. `KeepState` mirrors the api's `AppState` but exposes a direct `commit` over the engine (NO HTTP self-call). Operator session: simplest is reusing `identity` tokens carried in a cookie/header for the local app; the acting Knight's `UserId` threads into `commit(Some(knight), …)`. Drive tests via `tower::ServiceExt::oneshot`.

### Dependencies
domain-core (`Engine`/`DomainEngine`), store (`Store`/`SharedStore`/`Provisioner`), identity (`Identity`/`ProdIdentity` for operator login). ADR [[SQUIRE-A-0008]]; spec FR-ADM4, AR-8, AR-1.

### Requirements covered
REQ-1.4.2 (the `apply(by)` seam), NFR-1.1.3 (local-only authoring), and the A-0008 form-factor decision.

## Status Updates

*To be added during implementation*