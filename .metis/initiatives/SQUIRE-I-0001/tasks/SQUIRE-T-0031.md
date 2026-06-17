---
id: api-openapi-3-contract-emitted
level: task
title: "API: OpenAPI 3 contract emitted from Rust (utoipa) + frozen openapi.json + conformance test"
short_code: "SQUIRE-T-0031"
created_at: 2026-06-17T12:21:30.148945+00:00
updated_at: 2026-06-17T12:33:10.449054+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# API: OpenAPI 3 contract emitted from Rust (utoipa) + frozen openapi.json + conformance test

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0003]] (producing API) · ADR: [[SQUIRE-A-0009]] · Foundation for [[SQUIRE-S-0005]] / [[SQUIRE-S-0006]]

## Objective

Emit the Local API's wire contract as an **OpenAPI 3 document derived from the Rust types** (`utoipa`), freeze it as a committed `openapi.json`, and guard it with a conformance test — so the spec matches the server by construction and the phone-client SDKs can be generated from it without drift (A-0009).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] The wire DTOs in `domain-core::contract` derive `utoipa::ToSchema` (behind the `openapi` feature = `dep:utoipa` + `serde`): `StateView` + nested, `HouseholdReview` cluster, claim/redemption envelopes, identity DTOs, id/enum primitives. The `u128` id newtypes are schema-typed `int64` (A-0009 invariant; runtime serde unchanged).
- [x] The `api` crate annotates all 12 routes (`#[utoipa::path]`) and aggregates an `ApiDoc` (`#[derive(OpenApi)]`) covering the Squire, Knight, and control-plane surfaces, with the `bearer_auth` scheme + `X-Household` documented.
- [x] A committed **`crates/api/openapi.json`** (12 paths) is the generation source of truth; `tests/openapi.rs` regenerates `ApiDoc::openapi()` and asserts byte-equality with the committed file (`UPDATE_OPENAPI=1` re-freezes; regen via `cargo run -p api --example gen_openapi`).
- [x] Conformance smoke checks: the doc builds + round-trips, and asserts key schemas/paths, the int64 id invariant, the bearer scheme, and the externally-tagged `ClaimState::Approved` variant.
- [x] No wire change: existing api/keep JSON shapes are unchanged; `cargo test --workspace` green and warning-free (42 binaries); lean `domain-core` (no serde/openapi) still builds.

## Implementation Notes

### Technical Approach
Add `utoipa` (v5, axum 0.8-compatible). Feature-gate `ToSchema` derives in `domain-core` under an `openapi` feature that also enables `serde`. In `api`, derive `OpenApi` for an `ApiDoc` and annotate handlers; write a small generator (a test or `examples/gen_openapi.rs`) that produces `openapi.json`, plus the equality/conformance test. Schema-type the id newtypes with `#[schema(value_type = i64)]`. Do NOT change runtime serialization (ids stay numeric on the wire — A-0009 invariant).

### Dependencies
[[SQUIRE-S-0003]] (the api + DTOs), [[SQUIRE-A-0009]]. Blocks the Kotlin SDK generation + the phone apps (S-0005/S-0006).

### Risk Considerations
`utoipa` enum representation must match `serde`'s (externally-tagged) — verify the emitted schema for the data-carrying enums (`ClaimState::Approved{points}`, `LockReason`, `QuestStatus`). Keep the `openapi` feature additive so the default/no-serde store build is unaffected.

## Status Updates

**2026-06-17 — Done (`296ec58`).** OpenAPI contract foundation for the phone clients.

- **domain-core**: optional `utoipa` v5 dep + `openapi` feature (`= dep:utoipa + serde`); `ToSchema` on every wire DTO + id/enum primitives. `u128` ids carry `#[schema(value_type = i64)]` (numeric on the wire; A-0009 invariant). Fully feature-gated — lean `domain-core` (no serde/openapi) still builds with zero non-dev deps.
- **api**: `#[utoipa::path]` on all 12 handlers (bodies + `bearer_auth` + `X-Household` + statuses); `openapi::ApiDoc` (`#[derive(OpenApi)]`) with the bearer scheme; `openapi_doc()`; `examples/gen_openapi.rs` regenerator.
- **Frozen artifact**: `crates/api/openapi.json` (12 paths). `tests/openapi.rs`: byte-equality vs the committed file (`UPDATE_OPENAPI=1` to re-freeze) + smoke checks (key schemas/paths, int64 ids, bearer scheme, externally-tagged `ClaimState::Approved`).
- utoipa needed **no** extra annotations to match serde's externally-tagged enum representation (verified: `"Pending"` vs `{"Approved":{points}}`, `DecisionDto` honors `snake_case`).

**Verification (independent):** `cargo test --workspace` green & warning-free (42 binaries); `cargo test -p api --test openapi` = 2 passed; all 12 endpoints present in `openapi.json`; lean `domain-core` build OK.

This resolves the S-0005/S-0006 "Kotlin DTO drift" ADR and is the source of truth for the generated Kotlin SDK. **Next (gated on toolchain): Phase B** — generate the Kotlin SDK from `openapi.json` (`openapi-generator`, JVM); **Phase C** — the Android app (needs the Android SDK).