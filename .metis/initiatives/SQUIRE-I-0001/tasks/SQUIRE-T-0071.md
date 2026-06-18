---
id: keep-refine-e2e-test-the
level: task
title: "Keep: refine + E2E-test the Achievement creation workflow (match the quest creator)"
short_code: "SQUIRE-T-0071"
created_at: 2026-06-18T16:45:11.918321+00:00
updated_at: 2026-06-18T16:45:11.918321+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Keep: refine + E2E-test the Achievement creation workflow

## Parent Initiative

[[SQUIRE-I-0001]] · Apply the quest-creator's rigor (define flows → test → refine) to **Achievement creation**. **Define-first**: nail the flows + expected outcomes, check current coverage, *then* fill gaps + polish the UI.

## Objective

Define the Achievement-creation flows and their expected outcomes, verify what the current tests actually cover, then close the gaps (E2E) and refine the Keep form to the quest-creator's standard.

## Domain ground-truth (so outcomes are real, not guessed)

`Achievement { name, criterion, bonus_points, active }`. **Criterion** (externally-tagged on the wire):
- `PointsEarned { total }` — `total ≥ 1` else `InvalidDefinition`.
- `TotalCompletions { scope, count }` — `count ≥ 1` else `InvalidDefinition`.
- `Streak { scope, length, basis }` — `length ≥ 1`; `basis ∈ {ScheduledOccurrences, CalendarDays}`.

**Scope** = `Any` | `Quest(<existing quest>)` | `Category(<text>)`. `scope = Quest(missing)` → `QuestNotFound`. `bonus_points` may be `0` (pure unlock). *(Open question: empty `Category("")` is currently **accepted** — likely should be rejected.)*

## Flows & expected outcomes (DRAFT — confirm/adjust)

- **F1 Points-earned** — name + total(≥1) + bonus → 200; listed as "PointsEarned · N · +bonus"; gate-eligible. total=0 → **400**.
- **F2 Total-completions** — name + scope + count(≥1) + bonus → 200; listed. count=0 → **400**; scope=Quest(missing) → **404/QuestNotFound**.
- **F3 Streak** — name + scope + length(≥1) + basis + bonus → 200; listed. length=0 → **400**; scope=Quest(missing) → **404**.
- **F4 Scope picker** — Any (no extra) · Quest (from existing quests dropdown) · Category (free text). Builds the externally-tagged `{Quest:id}` / `{Category:str}` / `"Any"`.
- **F5 Gate a reward** — a created achievement appears in the Rewards "Requires achievement" dropdown; a gated reward is **locked until earned** in the player UI.
- **F6 Archive** — archive-not-delete; inactive; history stays valid.
- **F7 Progressive disclosure (UX)** — only the fields for the chosen criterion/scope are shown.
- **F8 Earning/evaluation** — meeting the criterion awards the bonus + unlocks the gate (domain behavior).

## Current coverage (verified 2026-06-18)

| Flow | Domain `achievements.rs` (31) | Keep API `catalog.rs` | E2E / UI (Playwright) |
|------|---|---|---|
| F1 points create | — | ✅ create+list+audit; ✅ total=0→400 | ❌ |
| F2 total-completions create | — | ❌ (only *points* tested) | ❌ |
| F3 streak create (length+basis) | — | ❌ | ❌ |
| F4 scope Quest / Category | — | ❌ | ❌ |
| F5 gate a reward | — | ✅ gate valid + missing→404 | ❌ (gate dropdown untested) |
| F6 archive | — | ~ (generic catalog archive) | ❌ |
| F7 progressive disclosure | — | n/a | ❌ |
| F8 earning/evaluation | ✅✅ thorough | — | n/a |

**Gap:** earning logic + points-create + gating are covered. **Streak & TotalCompletions creation, scope variations (Quest/Category), the form workflow, and the externally-tagged wire shapes for those are NOT** — exactly where a wire bug (cf. the quest 422) could hide. No UI-level test of the Achievement form exists.

## Acceptance Criteria (proposed — pending flow sign-off)

- [ ] Flows F1–F8 confirmed (incl. the empty-`Category` decision).
- [ ] E2E (Playwright) `keep-achievements.spec.ts`: drive each criterion (points/total/streak) incl. a Quest scope; assert it lists; assert invalid (total/count/length = 0) shows an error; assert the new achievement appears as a reward gate. Screenshots.
- [ ] (If flows demand) Keep API tests in `catalog.rs` extended to Streak + TotalCompletions + Scope=Quest/Category.
- [ ] Refine the Achievement form to the quest-creator standard (themed card, chips for criterion/scope/basis, progressive disclosure, friendly labels). Before/after screenshots.
- [ ] Any bug surfaced (e.g. empty category, a wire shape) fixed.

## Status Updates

**2026-06-18 — Flows defined + coverage mapped (define-first).** Grounded the expected outcomes in `validate_achievement` and the keep.js wire-builder. Verified coverage: domain earning logic is thoroughly tested; the Keep API tests cover only *PointsEarned* create + gating; **no test covers Streak/TotalCompletions creation, scope variations, or the form workflow**. Awaiting flow sign-off (esp. empty-Category) before writing the missing E2E + refining the form.
