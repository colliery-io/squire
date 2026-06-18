---
id: keep-rich-quest-authoring-assign
level: task
title: "Keep: rich quest authoring (assign to squires, cadence, repeat, completion)"
short_code: "SQUIRE-T-0062"
created_at: 2026-06-18T12:14:10.343059+00:00
updated_at: 2026-06-18T12:27:17.946540+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Keep: rich quest authoring (assign to squires, cadence, repeat, completion)

## Parent Initiative

[[SQUIRE-I-0001]] · The quest-creation cycle (user reqs: assign to squire(s) default-all; repeatable; one-off+due; multiple-per-day). Builds on [[SQUIRE-T-0061]] (Quests tab). The domain already models all of this — this exposes it in the Keep form.

## Objective

Replace the Keep's minimal quest form (title/reward/auto-approve, everything else hardcoded) with the full authoring surface the domain already supports: **assignment** (All squires — default — or specific squires), **cadence** (Every day / Weekly pick-days / One-time + optional due date), **completion** (each-assignee vs race), **multiple-times-per-day**, and optional category. The quest list shows a cadence/assignment summary.

## Acceptance Criteria

## Acceptance Criteria

- [x] Assignment: radio **All squires** (default, `"AllSquires"`) vs **Specific squires** → a checkbox list of active Squires (loaded from `/api/members`, filtered `role==Squire && active`), building `{Squires:[ids]}` (numbers). Validation: ≥1 squire when "specific".
- [x] Cadence select → **Every day** (`{Recurring:"Daily"}`), **Weekly** (weekday checkboxes → `{Recurring:{Weekly:{days:[…]}}}`, ≥1 day), **One-time** (`{OneOff:{due}}`) with an optional date input converted to the domain's Monday-aligned day-count.
- [x] **Completion** select (EachAssignee / Race) and a **"Can be earned multiple times per day"** checkbox (`repeatable_within_day`). Optional **Category** text. Progressive disclosure (weekday picker for Weekly, due for One-time, squire list for "specific").
- [x] Quest list shows a summary (e.g. "Daily · all squires · auto-approve", "Mon/Wed/Fri · Gawain · race"). `cargo build -p keep` + `node --check` green; payload shapes verified against a live server (all cadence/assignment combos → 200; string ids → 400, confirming numbers are required).

## Implementation Notes

### Technical Approach
`index.html`: quest form gains category, a `How often` select with a weekday `<fieldset>` + a `due` date input, a completion select, an `Assign to` fieldset (radios + a `#quest-squires` checkbox box), and a multiple-per-day checkbox. `keep.js`: `loadQuestSquires()` (wired into the Quests tab loader) populates the picker + `squiresById`; `syncQuestFields()` does progressive disclosure; submit builds the externally-tagged cadence/assignment JSON; `toDomainDate()` converts yyyy-mm-dd → `floor(unixDays)+3`; `questSummary()` labels the list. `keep.css`: inline checkbox rows + `[hidden]{display:none!important}` so the `hidden` attribute beats `label{display:block}`.

### Dependencies
[[SQUIRE-T-0061]] (Quests tab). The domain already supports `Assignment`/`Cadence`/`Completion`/`repeatable_within_day` — no contract change.

### Risk Considerations
UserId serializes as a **string** in the members DTO but `Assignment::Squires` deserializes UserId from **numbers** — `.map(Number)` is required (verified: `["2"]` → 400, `[2]` → 200). Weekly needs ≥1 day and "specific" needs ≥1 active squire or the domain returns InvalidDefinition (400). Due-date day-count uses the current UTC convention; will shift with the local-tz clock in [[SQUIRE-T-0060]].

## Status Updates

**2026-06-18 — Done.** Built the full quest authoring form in the Keep Quests tab and verified payloads against a throwaway live server. All cadence shapes (Daily / Weekly{days} / OneOff{due|null}) and assignment shapes (AllSquires / Squires[numbers]) validate (200); string squire ids correctly 400. Form renders verified via headless Chrome (Daily/Weekly/One-time states, progressive disclosure, rich list summaries). Fixed a `[hidden]` vs `label{display:block}` CSS bug. No contract change. Next: [[SQUIRE-T-0063]] starter library + import.