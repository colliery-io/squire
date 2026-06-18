---
id: keep-tabbed-shell-quests-review
level: task
title: "Keep: tabbed shell (Quests / Review / Rewards / Members / Pair / Log)"
short_code: "SQUIRE-T-0061"
created_at: 2026-06-18T12:14:08.926515+00:00
updated_at: 2026-06-18T12:15:20.505916+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Keep: tabbed shell (Quests / Review / Rewards / Members / Pair / Log)

## Parent Initiative

[[SQUIRE-I-0001]] · First step of the Keep UX cycle (user: "Quest management should be in a tab of its own"). Foundation for [[SQUIRE-T-0062]] (rich quest authoring) and [[SQUIRE-T-0063]] (library).

## Objective

Turn the Keep's "un-hide every panel at once" layout into a real **tabbed shell**: the nav switches which single panel is visible, so each area (Quests, Review, Rewards, Achievements, Members, Pair, Log) is its own tab. Quest management gets its own dedicated tab. Pure front-end (keep.js + index.html + keep.css) — no API/behaviour change.

## Acceptance Criteria

## Acceptance Criteria

- [x] After login, exactly one panel shows at a time; the nav is a tab bar that switches panels (active tab highlighted). Quests is its own tab and the default landing tab.
- [x] Tabs: **Quests · Review · Rewards · Achievements · Members · Pair · Log** (Rewards = the existing items panel, relabelled; Achievements now has its own tab too). Deep-link via `#hash` works (`currentTab()` reads the hash on entry; `hashchange` switches) and survives reload.
- [x] Per-tab data loads on activation via `TAB_LOADERS` (Review refreshes each time); cross-tab dropdowns (item-gate ← achievements, ach-scope ← quests) eager-loaded once in `enterShell`. No regressions to existing forms/lists.
- [x] Styled to the parchment/royal/gold theme (active tab royal-filled); `cargo build -p keep` green; rendered both Quests + Rewards tabs via headless Chrome.

## Implementation Notes

### Technical Approach
Replace the two `for (const id of [...]) hidden=false` blocks with a `showTab(name)` switch: hide all `*-panel`, show the chosen one, mark the active nav link, lazy-load that tab's data once. Drive from nav `click` + `hashchange`; default to `#quests`. Keep all existing ids/forms intact (the authoring/review/etc. logic is unchanged). Restyle the `nav` into a tab strip in keep.css. The login/register handlers call `showTab(currentHashOr('quests'))` instead of un-hiding everything.

### Dependencies
[[SQUIRE-T-0058]] (Keep theme). Independent of the timezone/api/native tasks.

### Risk Considerations
Don't break the HttpOnly same-origin fetch flow or any form ids the JS binds. Keep it dependency-free vanilla JS (ADR A-0008). Ensure first-run register path and normal login path both land on the tabbed shell.

## Status Updates

**2026-06-18 — Done.** Converted the Keep shell from "un-hide every panel" to a real tab bar. `keep.js`: added `TABS` + `TAB_LOADERS` + `showTab(name)` + `enterShell(who, fallback)` + a `hashchange` listener; both login and first-run register now call `enterShell` (one shared path) which reveals `#shell`, eager-loads the two cross-tab dropdown feeds, and shows the active tab (default `#quests`). `index.html`: nav → `.tabs` strip with `data-tab`/`href="#..."` links incl. a new **Achievements** tab and **Rewards** (the items panel, relabelled). `keep.css`: tab strip with royal-filled active tab. Vanilla JS, no API change. Verified via headless-Chrome renders of the Quests and Rewards tabs (switching toggles a single panel + active highlight). `cargo build -p keep` green, `node --check keep.js` clean. Next: [[SQUIRE-T-0062]] rich quest authoring in the Quests tab.