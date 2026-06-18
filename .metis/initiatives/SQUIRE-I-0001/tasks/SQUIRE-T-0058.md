---
id: ui-polish-3-app-icon-medieval
level: task
title: "UI polish 3: app icon (medieval crest, adaptive) + the Keep web UI restyle"
short_code: "SQUIRE-T-0058"
created_at: 2026-06-18T03:10:00+00:00
updated_at: 2026-06-18T03:36:50.769711+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# UI polish 3: app icon (medieval crest, adaptive) + the Keep web UI restyle

## Parent Initiative

[[SQUIRE-I-0001]] · Finishes the polish pass started in [[SQUIRE-T-0056]]/[[SQUIRE-T-0057]] (matches the same "playful quest" look)

## Objective

Two finishing pieces of the polish: (1) a real **app icon** (replace the default Android robot) — a medieval crest/shield in the brand palette, as an adaptive icon; (2) restyle the **Keep** web UI (the parent's computer admin) to match the apps' parchment/royal/gold theme.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] Adaptive launcher icon — vector `ic_launcher_foreground` (gold heraldic shield + cream star) on a royal `@color/ic_launcher_background`, wired via `mipmap-anydpi-v26/ic_launcher{,_round}.xml` (+ `monochrome` silhouette for Android 13 themed icons), referenced from the manifest (`android:icon`/`roundIcon`). minSdk 26 ⇒ adaptive-only, no PNG buckets needed. Verified on the launcher drawer (replaces the default robot).
- [x] The Keep (`crates/keep/assets/keep.css`) restyled to the brand: parchment bg, royal serif headings, gold-ruled royal header, carded sections, royal pill buttons, gold nav, themed inputs/selects/fieldsets/lists/code/pre. CSS-only — no markup/JS/behaviour change, same rust-embed asset.
- [x] `:app:assembleDebug` builds with the new icon; `cargo build -p keep` green; screenshots captured (icon in drawer, Keep rendered via headless Chrome).

## Implementation Notes

### Technical Approach
Icon: a vector drawable foreground (`res/drawable/ic_launcher_foreground.xml`) + a color/gradient background, wired via `mipmap-anydpi-v26/ic_launcher.xml` (+ round, + `monochrome` for themed icons). Keep: pure CSS restyle of `keep.css` (CSS variables for the palette, card/button styling, a web font stack with serif headings) + minimal `index.html` class hooks; do not touch `keep.js`.

### Dependencies
[[SQUIRE-T-0056]] (palette), [[SQUIRE-T-0025]]..[[SQUIRE-T-0030]] (the Keep), [[SQUIRE-T-0036]]/[[SQUIRE-T-0045]] (Keep panels to keep working).

### Risk Considerations
Adaptive-icon XML is fiddly (foreground safe-zone, density buckets) — a clean vector + solid/gradient background avoids raster assets. Keep CSS must not break the existing forms/JS (keep ids/classes the JS uses). Web fonts: prefer a system stack (no network) unless a bundled font is wanted.

## Status Updates

**2026-06-17 — Done (icon + Keep, both verified).** (1) **App icon**: new adaptive launcher icon — `res/drawable/ic_launcher_foreground.xml` (gold shield `#D7B43E` + cream five-point star) on royal `@color/ic_launcher_background`, plus a `monochrome` shield silhouette (star punched via evenOdd) for themed icons; wired through `mipmap-anydpi-v26/ic_launcher{,_round}.xml` and the manifest `android:icon`/`roundIcon`. minSdk 26 means adaptive-only suffices (no per-density PNGs). Confirmed on the emulator drawer — the default Android robot is gone, replaced by the royal-and-gold crest. (2) **Keep web UI**: rewrote `crates/keep/assets/keep.css` to the parchment/royal/gold theme (royal header with gold rule, serif royal section titles, cream carded sections, royal pill buttons, gold nav links, themed form controls/lists/code/pre) — CSS-only, no markup or `keep.js` change. `cargo build -p keep` green (rust-embed re-embeds). Rendered the Keep with headless Chrome to verify. Screenshots sent. Polish pass (T-0056/0057/0058) complete — next: bump versionCode and package one OTA update so the running server can hand these builds to the phone.