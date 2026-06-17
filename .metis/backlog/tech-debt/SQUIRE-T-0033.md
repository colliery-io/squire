---
id: make-data-carrying-enums
level: task
title: "Make data-carrying enums (ClaimState/RedemptionState/LockReason) generator-friendly for the Kotlin SDK"
short_code: "SQUIRE-T-0033"
created_at: 2026-06-17T13:02:45.219655+00:00
updated_at: 2026-06-17T13:02:45.219655+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


exit_criteria_met: false
initiative_id: NULL
---

# Make data-carrying enums (ClaimState/RedemptionState/LockReason) generator-friendly for the Kotlin SDK

Spec: [[SQUIRE-S-0005]] / [[SQUIRE-S-0006]] · Contract: [[SQUIRE-A-0009]] · Surfaced by [[SQUIRE-T-0032]].

## Objective

Make the contract's **data-carrying enums** round-trip cleanly through `openapi-generator` so the Kotlin SDK models them faithfully — required before either phone UI deserializes `GET /state` / `GET /household-review`.

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P1 - High (important for user experience) — blocks the phone UI layer (but NOT `:core`, which uses ids only).

### Technical Debt Impact
- **Current Problems**: utoipa emits our serde **externally-tagged** enums (`ClaimState` = `"Pending"` | `{"Approved":{points}}` | `{"Rejected":{reason?}}`, plus `RedemptionState`, `LockReason`, `DecisionDto`) as an OpenAPI `oneOf`. `openapi-generator` 7.11 (kotlin/kotlinx) **collapses each `oneOf` into a single merged data class** (e.g. `ClaimState(approved, rejected)`), dropping the unit variant (`"Pending"`) and producing types that (a) won't deserialize the real JSON and (b) are nearly impossible to construct in tests. Discovered in T-0032 (`:core` sidesteps them by using ids only).
- **Benefits of Fixing**: the generated SDK faithfully models claim/redemption/lock state, so the phone UIs can render `GET /state` / `HouseholdReview` directly from generated types — preserving the A-0009 "no hand-written DTOs / no drift" guarantee for the whole payload, not just the id-bearing parts.
- **Risk Assessment**: if unfixed, the UI layer must hand-map these few enums (reintroducing localized drift) or parse raw JSON. Acceptable short-term for `:core`; not acceptable for the rendering layer.

## Acceptance Criteria

- [ ] The generated Kotlin SDK represents `ClaimState`, `RedemptionState`, `LockReason`, and `DecisionDto` as faithful sum types (sealed classes / discriminated unions) that **round-trip the real server JSON** (incl. the unit variants like `Pending`).
- [ ] A round-trip test: a real `GET /state` (and `HouseholdReview`) JSON sample from the Rust server decodes via the generated SDK and re-encodes to equivalent JSON.
- [ ] The Rust wire JSON either stays unchanged OR, if a discriminator is introduced, the change is deliberate, conformance-re-frozen (`openapi.json`), and all existing api/keep tests updated.
- [ ] `cargo test --workspace` green; `./gradlew :sdk:compileKotlin` + a new SDK decode test green.

## Implementation Notes

### Technical Approach
Options to evaluate (pick one):
1. **Adjacently-tagged wire form** — change these enums' serde to `#[serde(tag = "kind", content = "data")]` (or internally-tagged) so the OpenAPI is a discriminated object `openapi-generator` maps to a sealed class with a `discriminator`. Cleanest for codegen, but a **wire change** (re-freeze `openapi.json`; update api/keep tests that assert the current shape).
2. **utoipa schema hints** — annotate the schemas with an OpenAPI `discriminator`/`oneOf` shape the generator handles, without changing serde (if achievable).
3. **Hand-authored Kotlin mappers** for just these 4 enums, layered over the generated models (localized, drift-prone — least preferred).
4. **A different generator/template** that handles externally-tagged `oneOf` correctly.

### Dependencies
[[SQUIRE-A-0009]], [[SQUIRE-T-0031]] (openapi.json + conformance), [[SQUIRE-T-0032]] (where it surfaced). Needed before the phone **UI** tasks (S-0005 render, S-0006 review).

### Risk Considerations
Option 1 ripples through existing api/keep tests (numeric/string-shape assertions on `ClaimState` etc.). Sequence it before building either phone's rendering layer; `:core` (sync/outbox) is unaffected and can proceed.

## Status Updates

**2026-06-17 — Filed** from [[SQUIRE-T-0032]] (discovered while inspecting the generated SDK). Not started; `:core` proceeds without it.