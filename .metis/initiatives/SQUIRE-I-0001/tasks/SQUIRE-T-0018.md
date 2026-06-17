---
id: api-trust-boundary-idempotency
level: task
title: "API: trust-boundary, idempotency & HTTP integration tests"
short_code: "SQUIRE-T-0018"
created_at: 2026-06-17T05:13:28.795978+00:00
updated_at: 2026-06-17T05:45:01.364840+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


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

- [ ] Full flow over HTTP: register → Knight login → add Squire → Squire login → Squire `POST /claims` → Knight `GET /household-review` shows it pending → Knight approves → Squire `GET /state` shows the credited balance (AC-1).
- [ ] Trust boundary: a Squire token on `/admin/*` or `/household-review` → 403; a Squire cannot read another Squire's `/state`; no authoring route exists (define/archive → 404); missing/invalid token → 401; wrong handle/tenant → 401/403.
- [ ] Idempotency over HTTP: re-POST of the same `claim_id`/`request_id`/`command_id` yields exactly one event and re-returns the prior outcome (AC-3).
- [ ] AC-6 over HTTP: a redemption request that no longer clears at Knight approval fails (insufficient points) at approval time.
- [ ] `cargo test -p api` green; `cargo test --workspace` green.

## Implementation Notes

### Technical Approach
Build the axum `Router` from real app state — a SQLite store via `Provisioner`, a `DomainEngine`, and the dev `Identity` — and drive it through `oneshot` (or bind `127.0.0.1:0`), asserting both statuses and bodies. The trust-boundary assertions (role enforcement, cross-Squire isolation, absence of any authoring route) are the core of this task.

### Requirements covered
NFR-1.1.1, NFR-1.1.2, NFR-1.1.3; REQ-1.2.1, REQ-1.2.2; AC-1, AC-3, AC-6.

### Dependencies
SQUIRE-T-0014..T-0017 (full API surface, control plane, and dev Identity).

## Status Updates

*To be added during implementation*