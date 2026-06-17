---
id: make-decisiondto-generator
level: task
title: "Make DecisionDto generator-friendly (flat struct) + re-freeze openapi.json + regen SDK"
short_code: "SQUIRE-T-0038"
created_at: 2026-06-17T16:58:52.860702+00:00
updated_at: 2026-06-17T17:05:41.849415+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Make DecisionDto generator-friendly (flat struct) + re-freeze openapi.json + regen SDK

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0006]] (REQ-K2/K4) · unblocks [[SQUIRE-T-0039]]/[[SQUIRE-T-0040]] · mirrors [[SQUIRE-T-0033]]

## Objective

The Knight app submits review decisions via `ReviewClaimReq`/`ReviewRedemptionReq`, whose `decision: DecisionDto` is an externally-tagged enum (`Approve` | `Reject{reason}`). openapi-generator 7.11 mangles it (a `oneOf` of a bare `"approve"` string and a `{reject:{…}}` object) into a Kotlin `DecisionDto` data class with a single **required** `reject` field — it literally cannot encode `Approve`, so the Knight can't approve anything. Apply the **same flat-struct fix as T-0033**: turn the wire `DecisionDto` (in `crates/api/src/knight.rs`) into a discriminant enum + optional payload field, re-freeze `openapi.json`, and regenerate the Kotlin SDK so `DecisionDto` round-trips. Keep's own `review.rs` `DecisionDto` (urlencoded forms, not on the SDK) is out of scope.

## Acceptance Criteria

## Acceptance Criteria

- [x] Wire `DecisionDto` (api) is a flat struct: a `DecisionKind { Approve, Reject }` discriminant plus an optional `reason` (skipped when None), with `approve()`/`reject(reason)` constructors and the `From<DecisionDto> for Decision` mapping preserved. Serializes to `{"verdict":"approve"}` / `{"verdict":"reject","reason":"…"}`.
- [x] `openapi.json` re-frozen (regen via `cargo run -p api --example gen_openapi`); the conformance test passes; `DecisionDto` schema is now a plain object (no `oneOf`).
- [x] Kotlin SDK regenerated (`:sdk:openApiGenerate` from the frozen spec); generated `DecisionDto` is a single decodable data class (no `DecisionDtoOneOf*`), and a round-trip decode/encode test covers approve + reject-with-reason.
- [x] `cargo test --workspace` green (9 api test bodies updated `"decision":"approve"` → `{"verdict":"approve"}`); `./gradlew :sdk:test :sdk:assemble` green.

## Implementation Notes

### Technical Approach
In `crates/api/src/knight.rs`: replace `enum DecisionDto { Approve, Reject{reason} }` with `enum DecisionKind { Approve, Reject }` (+ `ToSchema`) and `struct DecisionDto { verdict: DecisionKind, #[serde(skip_serializing_if="Option::is_none", default)] reason: Option<String> }` (+ `ToSchema`, `approve()`/`reject()` helpers). Update `From<DecisionDto> for Decision` to match on `verdict`. The handlers already do `req.decision.into()`, so they're unaffected. Re-freeze and regen the SDK; the `openapi.rs` `components(schemas(...))` list keeps `DecisionDto` and gains `DecisionKind`.

### Dependencies
[[SQUIRE-T-0031]] (OpenAPI pipeline), [[SQUIRE-T-0033]] (the flat-struct precedent + SDK decode test harness).

### Risk Considerations
Wire-breaking change to the review request body — safe because there is no released Knight client yet (this task exists to unblock the first one). Keep `review.rs`'s separate `DecisionDto` unchanged (different surface; urlencoded, not SDK).

## Status Updates

**2026-06-17 — Done.** `crates/api/src/knight.rs`: `DecisionDto` is now `{ verdict: DecisionKind, reason: Option<String> }` (flat), `DecisionKind { Approve, Reject }` (snake_case), `approve()`/`reject()` helpers, `From<DecisionDto> for Decision` matches on `verdict`. Registered `DecisionKind` in `openapi.rs` schemas. Re-froze `openapi.json` via the `gen_openapi` example — schema is a plain object now (no `oneOf`). Updated 9 api test bodies (`"decision":"approve"` → `{"verdict":"approve"}`). `cargo test --workspace` all green (incl. the openapi conformance/freeze test). Regenerated `:sdk` (clean, no build cache) → only `DecisionDto.kt` + `DecisionKind.kt` (the stale `DecisionDtoOneOf*` were build-cache leftovers, gone after `--no-build-cache`). Added 3 round-trip cases to `SdkDecodeTest` (encode-approve, decode-approve, round-trip reject+reason); note the SDK's Json emits `"reason":null` which the server reads as `None`. `:sdk:test` + `:sdk:assemble` green.