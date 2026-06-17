---
id: api-openapi-3-contract-emitted
level: task
title: "API: OpenAPI 3 contract emitted from Rust (utoipa) + frozen openapi.json + conformance test"
short_code: "SQUIRE-T-0031"
created_at: 2026-06-17T12:21:30.148945+00:00
updated_at: 2026-06-17T12:22:15.438662+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


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

- [ ] The wire DTOs in `domain-core::contract` derive `utoipa::ToSchema` (behind a feature, e.g. `openapi`, mirroring the `serde` pattern): `StateView` + nested (`QuestCard`/`QuestStatus` incl. `TakenByOther`, `StreakView`, `RewardCard`/`LockReason`, `ClaimStatus`/`ClaimState`, `RedemptionStatus`/`RedemptionState`), `HouseholdReview`/`SquireSummary`/`PendingClaim`/`PendingRequest`, `SubmitClaimReq`/`Resp`, `RequestRedemptionReq`/`Resp`, the Knight quick-action envelopes, and the identity DTOs (`RegisterHouseholdReq`/`Resp`, `LoginReq`/`Resp`, `AddMemberReq`/`Resp`, `HouseholdHandle`, `AuthToken`). The `u128` id newtypes are schema-typed `int64` (A-0009 invariant).
- [ ] The `api` crate annotates its routes (`#[utoipa::path]`) and aggregates an `ApiDoc` (`#[derive(OpenApi)]`) covering the Squire, Knight, and control-plane surfaces with auth (bearer + `X-Household`) documented.
- [ ] A committed **`openapi.json`** (e.g. `crates/api/openapi.json`) is the generation source of truth; a test regenerates `ApiDoc::openapi()` and asserts it **equals** the committed file (re-freeze on intentional change, e.g. via an `UPDATE_OPENAPI=1` escape hatch), so drift fails CI.
- [ ] A conformance check: representative instances of the key DTOs serialize and validate against the emitted schema (at minimum, the doc builds and the frozen file round-trips).
- [ ] No wire change: existing api/keep JSON shapes are unchanged; `cargo test --workspace` stays green and warning-free.

## Implementation Notes

### Technical Approach
Add `utoipa` (v5, axum 0.8-compatible). Feature-gate `ToSchema` derives in `domain-core` under an `openapi` feature that also enables `serde`. In `api`, derive `OpenApi` for an `ApiDoc` and annotate handlers; write a small generator (a test or `examples/gen_openapi.rs`) that produces `openapi.json`, plus the equality/conformance test. Schema-type the id newtypes with `#[schema(value_type = i64)]`. Do NOT change runtime serialization (ids stay numeric on the wire — A-0009 invariant).

### Dependencies
[[SQUIRE-S-0003]] (the api + DTOs), [[SQUIRE-A-0009]]. Blocks the Kotlin SDK generation + the phone apps (S-0005/S-0006).

### Risk Considerations
`utoipa` enum representation must match `serde`'s (externally-tagged) — verify the emitted schema for the data-carrying enums (`ClaimState::Approved{points}`, `LockReason`, `QuestStatus`). Keep the `openapi` feature additive so the default/no-serde store build is unaffected.

## Status Updates

*To be added during implementation*