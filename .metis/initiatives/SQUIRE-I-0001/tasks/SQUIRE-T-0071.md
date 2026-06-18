---
id: keep-refine-e2e-test-the
level: task
title: "Keep: refine + E2E-test the Achievement creation workflow (match the quest creator)"
short_code: "SQUIRE-T-0071"
created_at: 2026-06-18T16:45:11.918321+00:00
updated_at: 2026-06-18T17:15:53.808568+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

## Decisions (confirmed 2026-06-18)

- **Empty Category → reject.** `validate_achievement` rejects `Category("")` (`InvalidDefinition`/400); the Keep form requires a non-blank category when scope=Category.
- **Starter library → yes**, and **built against the quest library** — category-scoped to the quest library's categories (Bedroom/Kitchen/Hygiene/Homework/Pets/Outdoor) plus a couple of Any/Points entries, so the achievements line up with quests imported from the quest library.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] **Domain:** `validate_achievement` rejects an empty `Category` scope (`InvalidDefinition`); the `define_achievement_*` test covers it.
- [x] **Refine** the Achievement form: starter library + a non-blank-category guard (form-level + domain). (The form already used the Keep's themed card + `<select>`s + progressive disclosure, matching the Keep quest form — chips are the *native* app's idiom, not the Keep's, so no chip change here.)
- [x] **Starter library:** `assets/achievements-library.json` (category-scoped to the quest-library categories + Any/points) with one-tap import in the Achievements tab, mirroring the quest library (`loadAchLibrary`).
- [x] **E2E** (Playwright) `keep-achievements.spec.ts` — 5 green: points create; streak+Category create; empty-category → inline error (not created); **new quest → appears in the achievement scope dropdown**; library import → lists + becomes a reward gate. Screenshots.
- [x] **Keep API** (`catalog.rs`) extended: Streak + Category-scoped TotalCompletions create+list; empty-Category and zero-length → 400.
- [x] **Cross-tab dropdown freshness (user ask):** quest create/import/archive now also `loadAchScopeQuests()` (achievement Quest-scope dropdown stays current); member add/toggle now also `loadPairMembers()`; achievement create/import already refreshes the reward-gate dropdown via `loadCatalog`.

## Status Updates

**2026-06-18 — Flows defined + coverage mapped (define-first).** Grounded the expected outcomes in `validate_achievement` and the keep.js wire-builder. Verified coverage: domain earning logic is thoroughly tested; the Keep API tests cover only *PointsEarned* create + gating; **no test covers Streak/TotalCompletions creation, scope variations, or the form workflow**. Awaiting flow sign-off (esp. empty-Category) before writing the missing E2E + refining the form.

**2026-06-18 — Done.** Decisions confirmed (reject empty Category; ship a starter library built against the quest categories). Built: domain reject-empty-Category (+ test); Keep API tests for Streak/Category/empty (catalog.rs now 10 green); the `achievements-library.json` starter set + `loadAchLibrary` import; a non-blank-category form guard; and a Playwright `keep-achievements.spec.ts` (5 green) driving every criterion + the gate + the new-quest-in-scope-dropdown. **Also addressed the user's dropdown-freshness ask:** quest create/import/archive refresh the achievement Quest-scope dropdown, member changes refresh the Pair dropdown (gate dropdown already refreshed). Full suites green (web e2e 15, rust 20 files). Closes the achievement workflow to the quest-creator standard.