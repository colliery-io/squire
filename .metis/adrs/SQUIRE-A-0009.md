---
id: 001-wire-contract-as-openapi-emitted
level: adr
title: "Wire contract as OpenAPI emitted from Rust; phone clients use a generated SDK"
number: 1
short_code: "SQUIRE-A-0009"
created_at: 2026-06-17T12:18:47.509182+00:00
updated_at: 2026-06-17T12:20:58.543288+00:00
decision_date: 
decision_maker: 
parent: 
archived: false

tags:
  - "#adr"
  - "#phase/decided"


exit_criteria_met: false
initiative_id: NULL
---

# ADR-1: Wire contract as OpenAPI emitted from Rust; phone clients use a generated SDK

Resolves the "Deserializing `StateView` on the Kotlin side" decision in [[SQUIRE-S-0005]] (and the analogous one for [[SQUIRE-S-0006]]); complements [[SQUIRE-S-0003]] (the producing API).

## Context

The two phone clients ([[SQUIRE-S-0005]] Squire, [[SQUIRE-S-0006]] Knight) are Kotlin and must consume the Rust-owned wire DTOs (`StateView` + nested views, `HouseholdReview`, the claim/redemption request/response envelopes, the identity DTOs) **without drifting** from the server. The contract is large and enum-heavy (`QuestStatus` incl. `TakenByOther`, `LockReason`, `ClaimState`, `RedemptionState`, …) and is owned authoritatively by the Rust types. Hand-mirroring it in Kotlin invites silent drift. The operator (Dylan) raised generating a client SDK from a Swagger/OpenAPI spec.

## Decision

**The wire contract is an OpenAPI 3 document, emitted from the Rust types, and the phone clients consume a Kotlin SDK generated from it.**

- **Spec is derived from the server types** via `utoipa`: the wire DTOs in `domain-core::contract` get `ToSchema` (behind a feature, like `serde`), and the `api` routes get `utoipa::path` annotations. An aggregate `ApiDoc` produces the document, so the spec matches the server **by construction** (no hand-authored spec to drift).
- **Frozen + conformance-tested.** A committed `openapi.json` is the source of truth for generation; a Rust test regenerates the doc and asserts it equals the committed file (so any contract change must be intentionally re-frozen) and that representative DTO instances validate.
- **IDs are `int64` on the wire, with a "fits in i64" invariant.** The `u128` id newtypes (`UserId`, `QuestId`, …) are schema-typed `integer`/`int64`. The system only ever mints ids from monotonic counters and `Date.now()` (≪ 2⁵³), so they fit `i64` (and JS 2⁵³) with room to spare; this is recorded as an invariant rather than widening the wire type. (Avoids a string change that would churn ~10 test files for no real-world precision gain; revisit only if id-minting ever produces values ≥ 2⁶³.)
- **Generated Kotlin SDK.** `openapi-generator` (Java/JVM) generates a `kotlinx.serialization`-based client from `openapi.json`; both phone apps depend on it. Generation is a build step, not hand-maintained code.

This decision is the contract foundation for BOTH phone specs; the Android UI / Room / sync ADRs in S-0005/S-0006 sit on top of it.

## Alternatives Analysis

| Option | Pros | Cons | Risk Level | Implementation Cost |
|--------|------|------|------------|-------------------|
| OpenAPI emitted from Rust + generated Kotlin SDK (chosen) | Spec matches server by construction; no hand-mirrored Kotlin; serves both phones; ids fixed to strings | `utoipa` annotations across the DTOs; a generator tool in the build | Medium | M |
| Hand-written Kotlin DTOs + a Rust JSON golden/conformance test | No codegen tool; minimal new deps | Kotlin still hand-maintained; drift only caught if goldens are kept exhaustive | Medium | M |
| Codegen Kotlin directly from Rust (typeshare) | Generated Kotlin, in sync | typeshare's enum/representation coverage is narrower than our contract; no HTTP/spec artifact | Medium | M |

## Rationale

The operator's Swagger instinct is the cleanest path: an OpenAPI document is a language-neutral contract artifact that drives **both** Kotlin clients and is independently inspectable. Emitting it from the Rust types (rather than hand-authoring) removes the spec-vs-server drift that a hand-written Swagger file would reintroduce. Fixing ids to strings in the schema closes a real precision hazard before any client is built. `utoipa` annotation cost mirrors the (already-done) `serde` feature work.

## Consequences

### Positive
- One inspectable, language-neutral contract; Kotlin DTOs/clients are generated, not hand-kept.
- Spec matches the server by construction; a conformance test makes any change explicit (re-freeze `openapi.json`).
- No wire churn: the existing JSON shape (numeric ids, etc.) is preserved, so all current tests stay green.

### Negative
- `utoipa` `ToSchema` derives + `path` annotations spread across the contract/handlers (feature-gated).
- A JVM generator (`openapi-generator`) enters the build for the phone apps (the user installs it).
- IDs carry a "fits in i64" invariant rather than a widened wire type — a latent footgun only if id-minting ever changes to produce values ≥ 2⁶³ (documented; revisit then).

### Neutral
- Serving `GET /openapi.json` from the api is optional; the committed file is what generation consumes.
- Does not change the domain types (audit/u128 stay internal); only their wire representation.