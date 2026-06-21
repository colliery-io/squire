---
id: pre-release-hardening-surface
level: initiative
title: "Pre-release hardening: surface parity + Gherkin acceptance suite for all interfaces"
short_code: "SQUIRE-I-0004"
created_at: 2026-06-21T23:04:15.327984+00:00
updated_at: 2026-06-21T23:04:15.327984+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/decompose"


exit_criteria_met: false
estimated_complexity: L
initiative_id: pre-release-hardening-surface
---

# Pre-release hardening: surface parity + Gherkin acceptance suite

Gate before the next "make it solid" release. Two workstreams, operator-directed 2026-06-21.

## Context
A full audit of the four interfaces (Keep desktop, Android Knight, Android Squire, the LAN/cloud API)
found the parent surfaces **mostly at parity** but with a few desktop-only gaps, and the test suite
(~190 tests) **strong but with zero Given/When/Then structure** and gaps in cross-stack coverage.
Operator decisions:
- **Format:** true **Gherkin / Cucumber** feature files (shared scenario language).
- **Coverage:** **comprehensive** — restructure existing into GWT *and* add new phone→API→store
  integration + on-device/instrumented Android interaction tests.
- **Parity:** close **all three** gaps on the Android Knight app: cash payout, event-log/history,
  timezone setting.

## Goals & Non-Goals
**Goals:**
- Android Knight reaches parity with the Keep on the three flagged gaps.
- Every interface has a passing Gherkin acceptance suite, wired into `angreal test all`.
- New integration coverage exercises the real phone→API→store path end-to-end.

**Non-Goals:**
- The child (Squire) experience stays Android-only (Keep has no kid UI — intentional, not a gap).
- "Assume-Squire / act-as-child" stays Android-only (not adding it to the Keep).
- No new product features beyond the three parity closes.

## Parity findings (baseline)
| Capability | Keep | Android Knight | Action |
|---|---|---|---|
| Authoring (quests/rewards/achievements/hazards), members, pairing, review, redeem, adjust-coins | ✅ | ✅ | parity — keep |
| **Cash/dollar payout ("Pay")** | ✅ | ❌ | **close** → [[SQUIRE-T-0111]] |
| **Event log / history** | ✅ | ❌ | **close** → [[SQUIRE-T-0112]] |
| **Timezone setting** | ✅ | ❌ | **close** → [[SQUIRE-T-0113]] |
| Assume-Squire / mark-done-for-kid | ❌ | ✅ | accept (Android-only) |

Note: cash payout backend already exists (`/admin/adjust` accepts `currency=Cash`); history + timezone
need **new Knight-facing API endpoints** in `crates/api` (the Keep has them engine-direct via `/api/log/*`
and `/api/config`, but the LAN API does not expose them yet) → SDK regen → Android UI.

## Decomposition
**Workstream B — Gherkin acceptance suite (foundation first):**
- [[SQUIRE-T-0110]] Gherkin harness + feature catalog: cucumber-rs for domain-core + API; shared
  `.feature` files; wrap existing covered behavior so it's green; wire into `angreal test`.
- [[SQUIRE-T-0114]] Keep web Gherkin: Cucumber over Playwright for the Keep flows (wrap the 21 specs).
- [[SQUIRE-T-0115]] Android Gherkin: cucumber-jvm (or structured GWT) for Squire + Knight, incl.
  on-device/instrumented interaction tests (not just Paparazzi snapshots).
- [[SQUIRE-T-0116]] New full-stack integration: phone client → live API → store path, as Gherkin.

**Workstream A — Parity (Android Knight):**
- [[SQUIRE-T-0111]] Cash payout / settle action (backend exists; SDK + Android UI).
- [[SQUIRE-T-0112]] Event-log/history: add Knight API endpoint + SDK + Android history view.
- [[SQUIRE-T-0113]] Timezone setting: add Knight config GET/PUT to the LAN API + SDK + Android settings.

## Implementation Plan (phased)
1. **Foundation** — [[SQUIRE-T-0110]] (cucumber-rs for domain+API). Proves the Gherkin approach on the
   shared contract; everything else reuses the scenario vocabulary.
2. **Parity backend+SDK** — [[SQUIRE-T-0111]]/[[SQUIRE-T-0112]]/[[SQUIRE-T-0113]] backend endpoints +
   one SDK regen (batch the regen).
3. **Parity Android UI** — the three Android surfaces.
4. **Client Gherkin** — [[SQUIRE-T-0114]] (Keep), [[SQUIRE-T-0115]] (Android), [[SQUIRE-T-0116]]
   (integration).
5. Green `angreal test all`; then hand back for the bundled release (with [[SQUIRE-T-0108]]/[[SQUIRE-T-0109]]).

## Alternatives Considered
- *GWT-structured tests in existing frameworks* (no Cucumber) — rejected by operator in favor of true
  Gherkin feature files.
- *Restructure-only, no new coverage* — rejected; operator wants comprehensive incl. integration/E2E.

## Open Risks
- Cucumber across three stacks (Rust/Playwright/JVM) is real glue; keep one shared `.feature` vocabulary
  to avoid three dialects.
- On-device Android tests add an emulator/instrumented harness to CI (`angreal`) — net-new infra.
- Batch the SDK regen once after all three API endpoints land (avoid repeated codegen churn).
