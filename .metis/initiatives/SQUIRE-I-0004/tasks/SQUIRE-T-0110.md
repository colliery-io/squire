---
id: gherkin-harness-feature-catalog
level: task
title: "Gherkin harness + feature catalog: cucumber-rs for domain-core + API"
short_code: "SQUIRE-T-0110"
created_at: 2026-06-21T23:05:26.113320+00:00
updated_at: 2026-06-21T23:05:26.113320+00:00
parent: SQUIRE-I-0004
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: true
initiative_id: SQUIRE-I-0004
---

# Gherkin harness + feature catalog (cucumber-rs: domain-core + API)

Foundation of [[SQUIRE-I-0004]]. Stand up `cucumber-rs` and author the **shared `.feature` vocabulary**
that the Keep ([[SQUIRE-T-0114]]), Android ([[SQUIRE-T-0115]]), and integration ([[SQUIRE-T-0116]])
suites will reuse. First feature files wrap behavior already proven by the existing ~190 tests, so the
suite is **green from day one**.

## Scope
- Add `cucumber` dev-dep; a `tests/features/*.feature` tree + a Rust `World` + step defs.
- **Domain-core** features (rules): claim submit/approve/reject, idempotency, Race single-payout,
  redemption affordability, achievement unlock, cash accrual — map from existing
  `crates/domain-core/tests/*`.
- **API** features (HTTP contract via tower oneshot, no socket): role gating (Squire/Knight/control),
  `/state`, `/claims`, `/admin/*` review/adjust, pairing, **+ regression for the null-currency adjust
  bug ([[SQUIRE-T-0109]])** as a first-class scenario.
- One shared **Given** vocabulary (a household with quests/squires/balances) reused across both.
- Wire a `cargo test` cucumber target into **`angreal test`** (e.g. `angreal test gherkin` + folded into
  `test all`).

## Acceptance
- [ ] `.feature` files exist for domain-core + API with passing step defs; `angreal test` runs them green.
- [ ] The null-currency adjust regression is a named scenario (Given app sends `currency: null` …).
- [ ] Shared Given-steps documented so client suites reuse the same scenario language.

## Notes
Keep features behavior-focused (one scenario = one rule), not endpoint-by-endpoint. cucumber-rs runs on
the existing in-memory/oneshot harness — no socket, fast, deterministic.
