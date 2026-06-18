---
id: ui-polish-3-app-icon-medieval
level: task
title: "UI polish 3: app icon (medieval crest, adaptive) + the Keep web UI restyle"
short_code: "SQUIRE-T-0058"
created_at: 2026-06-18T03:10:00.000000+00:00
updated_at: 2026-06-18T03:10:00.000000+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# UI polish 3: app icon (medieval crest, adaptive) + the Keep web UI restyle

## Parent Initiative

[[SQUIRE-I-0001]] · Finishes the polish pass started in [[SQUIRE-T-0056]]/[[SQUIRE-T-0057]] (matches the same "playful quest" look)

## Objective

Two finishing pieces of the polish: (1) a real **app icon** (replace the default Android robot) — a medieval crest/shield in the brand palette, as an adaptive icon; (2) restyle the **Keep** web UI (the parent's computer admin) to match the apps' parchment/royal/gold theme.

## Acceptance Criteria

- [ ] Adaptive launcher icon (`mipmap`/`drawable` vector foreground + a brand background; `ic_launcher` + round + monochrome), a simple crest/shield/⚔ mark in the palette — replacing the default. Shows on the home screen + recents.
- [ ] The Keep (`crates/keep/assets/keep.css` + light `index.html` structure tweaks) restyled to the brand: parchment background, royal headings (display serif), gold accents/buttons, carded sections, tidy forms — without changing the JS/behaviour. Still one self-contained embedded asset (rust-embed), no new build step.
- [ ] `:app:assembleDebug` builds with the new icon; `cargo build -p keep` green (assets embed); screenshots captured (icon on launcher, Keep restyled).

## Implementation Notes

### Technical Approach
Icon: a vector drawable foreground (`res/drawable/ic_launcher_foreground.xml`) + a color/gradient background, wired via `mipmap-anydpi-v26/ic_launcher.xml` (+ round, + `monochrome` for themed icons). Keep: pure CSS restyle of `keep.css` (CSS variables for the palette, card/button styling, a web font stack with serif headings) + minimal `index.html` class hooks; do not touch `keep.js`.

### Dependencies
[[SQUIRE-T-0056]] (palette), [[SQUIRE-T-0025]]..[[SQUIRE-T-0030]] (the Keep), [[SQUIRE-T-0036]]/[[SQUIRE-T-0045]] (Keep panels to keep working).

### Risk Considerations
Adaptive-icon XML is fiddly (foreground safe-zone, density buckets) — a clean vector + solid/gradient background avoids raster assets. Keep CSS must not break the existing forms/JS (keep ids/classes the JS uses). Web fonts: prefer a system stack (no network) unless a bundled font is wanted.

## Status Updates

*To be added during implementation*
