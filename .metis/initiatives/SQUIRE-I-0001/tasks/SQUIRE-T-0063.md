---
id: starter-quest-library-one-tap
level: task
title: "Starter quest library + one-tap import"
short_code: "SQUIRE-T-0063"
created_at: 2026-06-18T12:14:11.748742+00:00
updated_at: 2026-06-18T12:32:09.375021+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Starter quest library + one-tap import

## Parent Initiative

[[SQUIRE-I-0001]] · User: "We should ship a common set of quests (library) that all tenants can import from." Closes the Keep quest-creation cycle ([[SQUIRE-T-0061]]/[[SQUIRE-T-0062]]).

## Objective

Ship a curated, categorized starter quest library with the Keep, and a one-tap **Import** in the Quests tab that creates the chosen quest for all squires (parents then edit/archive). The library is a shipped asset every household gets.

## Acceptance Criteria

## Acceptance Criteria

- [x] `assets/library.json` — a categorized set (Bedroom, Kitchen, Hygiene, Homework, Pets, Outdoor; ~3 each) with title, reward (★), and cadence (`daily` or `weekly`+days). Served at `/static/library.json` (embedded; `json` MIME already mapped).
- [x] Quests tab gains a collapsible **"📚 Add from the starter library"** section listing entries grouped by category, each with an **Import** button. Import → POST a quest (AllSquires, EachAssignee, cadence from the entry) → button becomes "Imported ✓" and the quest list refreshes. Monotonic `freshId()` avoids same-millisecond id collisions on rapid imports.
- [x] Verified on a live server: `/static/library.json` served + valid; daily and weekly library imports → 200. Rendered via the real login→`enterShell`→`loadLibrary` path (mocked fetch) — categories + Import buttons display. `node --check` + `cargo build -p keep` green.

## Implementation Notes

### Technical Approach
`assets/library.json` ({comment, quests:[{category,title,reward,cadence,days?}]}). `keep.js`: `loadLibrary()` (called once from `enterShell`) fetches it, groups by category, renders rows + Import buttons; Import builds the same quest payload as the form (cadence `daily`→`{Recurring:"Daily"}`, `weekly`→`{Recurring:{Weekly:{days}}}`), POSTs, marks imported, refreshes. `index.html`: a `<details id="library">` in the Quests panel. Reuses T-0062's verified payload shape.

### Dependencies
[[SQUIRE-T-0062]] (quest payload shape + Quests tab). Static asset pipeline (rust-embed `/static/{*path}`).

### Risk Considerations
Library is plain data — parents own their quests after import (edit/archive). Import uses AllSquires/daily defaults to stay safe; weekly entries carry their days. Same-ms id collisions avoided via `freshId()`. No contract change.

## Status Updates

**2026-06-18 — Done.** Shipped `assets/library.json` (18 chores across 6 categories) + a collapsible library section in the Quests tab with per-quest Import. Verified against a live throwaway server: asset served at `/static/library.json` (valid JSON), daily + weekly imports → 200, and the section renders correctly through the real `loadLibrary` path. Closes the Keep quest-creation cycle (tabs + authoring + library). Next: deploy the refreshed Keep to the live server, then [[SQUIRE-T-0060]] (local-midnight timezone) before the native phone tasks.