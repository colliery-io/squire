---
id: api-crate-scaffold-http-framework
level: task
title: "API: crate scaffold, HTTP framework, identity port & auth/tenant middleware"
short_code: "SQUIRE-T-0014"
created_at: 2026-06-17T05:13:23.984800+00:00
updated_at: 2026-06-17T05:23:03.329754+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# API: crate scaffold, HTTP framework, identity port & auth/tenant middleware

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0003]] (Local API & Trust Boundary)

## Objective

Stand up the new `crates/api` crate as an axum HTTP server — the project's only network seam and trust boundary — and establish its foundations: the `Identity` port (with a minimal in-crate dev implementation), the auth + tenant-resolution layer, and the shared app state that wires the per-tenant `Store`, `DomainEngine`, and `Identity`. Every authenticated request is reduced to a verified `Principal{household, user, role}` before any handler logic runs.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] `crates/api` builds with deps domain-core, store, axum 0.8 (+tokio), serde/serde_json; crate added to the workspace `members`.
- [x] HTTP framework resolved to **axum** (rationale in the decision log).
- [x] `Identity` port: `verify(handle, token) -> Result<Principal, AuthError>` (`Principal` = household + `UserId` + `Role`) + register/login/add_member signatures (bodies in T-0017); `DevIdentity` (in-memory token→principal, `seed` helper).
- [x] Auth extractor (`FromRequestParts`): `Authorization: Bearer` + `X-Household`, resolve tenant, `Identity::verify` → `Principal`; `RequireKnight`/`RequireSquire`; missing/invalid → 401, wrong role → 403.
- [x] App state wires `Mutex<Store>` (single-writer) + Engine + Identity; `GET /health` → 200; `serve` binds a configurable LAN host:port.
- [x] `cargo test -p api` green (3 unit + 7 `oneshot` integration, no socket).

## Implementation Notes

### Technical Approach
Keep handler logic testable without sockets by exercising the `Router` through `oneshot`. The sync `Store` lives behind a `Mutex` in app state; call the sync repo/engine inline or via `spawn_blocking`. The dev `Identity` is sufficient for the LAN-local single-tenant MVP; the hosted/hardened identity (real credential hashing, tenant registry, multi-tenant) is owned by SQUIRE-S-0007. This task delivers the port plus the dev impl so downstream tasks can build against a stable seam.

### Requirements covered
REQ-1.2.3, REQ-1.2.4; NFR-1.1.3, NFR-1.1.4; HTTP-framework and token-auth decision areas; ADR A-0002, A-0004.

### Dependencies
domain-core and store (T-0001..T-0013). The production `Identity` is SQUIRE-S-0007; this task provides only the port and the dev implementation.

## Status Updates

### 2026-06-17 — Implemented (T-0014)

**Decision log — HTTP framework = axum.** axum is the de-facto Rust HTTP standard, integrates the tower/tower-http middleware ecosystem, and is testable via `tower::ServiceExt::oneshot` without binding a socket — so the whole auth/handler path is exercised in-process. (Resolves the SQUIRE-S-0003 HTTP-framework decision area.)

Delivered: `crates/api` (added to workspace `members`) — axum 0.8 LAN server scaffold.
- `state.rs`: `AppState` wires `Mutex<Store<SystemClock>>` (single-writer) + `DomainEngine` + `SystemClock` + `Arc<dyn Identity>`, shared as `Arc<AppState>`.
- `identity.rs`: `Identity` port (`verify` + `register`/`login`/`add_member` signatures for T-0017) with `Principal{household,user,role}` / `AuthError`; minimal in-memory `DevIdentity` (token→principal map, `seed` test helper; control-plane methods are TODO stubs for T-0017).
- `auth.rs`: `Auth` / `RequireKnight` / `RequireSquire` extractors (`FromRequestParts<Arc<AppState>>`) reading `Authorization: Bearer` + `X-Household`. Missing/garbled/`BadToken`/`MissingToken`/`WrongTenant` → 401; wrong role (`Forbidden`) → 403.
- `lib.rs`: `router()` (`GET /health` → 200 `"ok"`) and `serve(state, addr)` binding a configurable LAN host:port.

Tests: `cargo test -p api` → 3 unit + 7 integration green. `cargo test --workspace` green (domain-core 70, store 27 unchanged). Warning-free.