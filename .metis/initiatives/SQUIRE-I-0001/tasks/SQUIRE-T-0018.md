---
id: api-trust-boundary-idempotency
level: task
title: "API: trust-boundary, idempotency & HTTP integration tests"
short_code: "SQUIRE-T-0018"
created_at: 2026-06-17T05:13:28.795978+00:00
updated_at: 2026-06-17T05:49:45.937633+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# API: trust-boundary, idempotency & HTTP integration tests

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0003]] (Local API & Trust Boundary)

## Objective

Prove the trust boundary, idempotency, and core flows end to end over a real axum app (real Store + Engine + dev Identity). The trust-boundary tests are the crux deliverable of this initiative.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] AC-1 full flow over HTTP: register → add Squire → Squire login → `POST /claims` (Pending) → Knight `GET /household-review` lists it labeled → Knight `/admin/review-claim` approve → Squire `GET /state` shows `balance:5` + `Approved{points:5}`.
- [x] Trust boundary: Squire token → 403 on all 5 `/admin/*` + `/household-review`; two squires each `GET /state` see only their own data (squire from token, no cross-Squire read); no authoring route (`/admin/define-quest`, `/quests`, … → 404); no token / garbage → 401; wrong `X-Household` → 401.
- [x] Idempotency over HTTP (AC-3): duplicate `claim_id`, `request_id`, and adjust `command_id` each → 200 twice, exactly one claim/request/credit (balance 10, not 20).
- [x] AC-6 over HTTP: fund Squire → request (affordable) → drain via negative adjust → Knight approve redemption → **409** (insufficient), request stays pending.
- [x] `crates/api/tests/integration.rs` (9), all over the wire via `oneshot`; `cargo test -p api` (38) + `cargo test --workspace` (135) green, 0 warnings.

## Implementation Notes

### Technical Approach
Build the axum `Router` from real app state — a SQLite store via `Provisioner`, a `DomainEngine`, and the dev `Identity` — and drive it through `oneshot` (or bind `127.0.0.1:0`), asserting both statuses and bodies. The trust-boundary assertions (role enforcement, cross-Squire isolation, absence of any authoring route) are the core of this task.

### Requirements covered
NFR-1.1.1, NFR-1.1.2, NFR-1.1.3; REQ-1.2.1, REQ-1.2.2; AC-1, AC-3, AC-6.

### Dependencies
SQUIRE-T-0014..T-0017 (full API surface, control plane, and dev Identity).

## Status Updates

**2026-06-17 — Completed.** `crates/api/tests/integration.rs` (9 tests) drives the whole component **over the wire** via `oneshot` against `router(state)` with real `Store`+`DomainEngine`+`DevIdentity`: tokens obtained through `POST /register` → `POST /members` → `POST /login` (only quest/item *authoring* is seeded directly into the store, since authoring is deliberately not a wire endpoint). Helpers `post_json`/`get`/`register`/`add_squire_and_login`/`fund`. Proven: **AC-1** end-to-end (claim→review-via-household-review→approve→credited state); **trust boundary** — Squire→403 on all 5 `/admin/*` + `/household-review`, per-Squire state isolation (token-derived squire), no authoring route (404), missing/garbage token→401, wrong `X-Household`→401; **AC-3** idempotency (dup claim_id/request_id/command_id → one effect); **AC-6** (drain-before-approve → 409, request stays pending).

Results: `cargo test -p api` → 38 (health 7, squire 5, knight 7, control 4, integration 9, lib 6) green; `cargo test --workspace` → **135** (domain-core 70, store 27, api 38), 0 warnings. Committed. **This completes the Local API & Trust Boundary component (T-0014..T-0018).**