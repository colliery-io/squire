---
id: ui-polish-2-parent-knight-review
level: task
title: "UI polish 2: parent (Knight) review + assume + pairing screens restyle"
short_code: "SQUIRE-T-0057"
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

# UI polish 2: parent (Knight) review + assume + pairing screens restyle

## Parent Initiative

[[SQUIRE-I-0001]] · Applies the [[SQUIRE-T-0056]] theme to the parent-facing surfaces · **blocked_by [[SQUIRE-T-0056]]**

## Objective

Propagate the "playful quest" theme/components (from T-0056) to the **parent** surfaces — keeping them clean and scannable (a management tool), just themed and tidied: the Knight review home, the per-Squire rows + Add-funds dialog, the "Open / Acting as" flow, and the first-run **pairing screen**.

## Acceptance Criteria

- [ ] `KnightHomeScreen` restyled with the shared components: per-Squire cards (name + gold balance + Open/Mark-done/Redeem/Add-funds), pending-claim and pending-redemption cards with clear Approve/Reject, a themed offline + update banner. Add-funds dialog tidied.
- [ ] The **pairing screen** (`:pairing/PairingScreen`) restyled: a friendly themed onboarding (title/crest, clear Scan QR primary + manual fields + Discover + debug button), consistent inputs/buttons.
- [ ] The "Acting as <name>" assumed-Squire view (reuses the themed `PlayerHomeScreen`) reads cleanly with the parent-context header/back.
- [ ] `:app:assembleDebug` builds; behaviour unchanged; screenshots captured (Knight home, pairing).

## Implementation Notes

### Technical Approach
Reuse the `SquireTheme` + components from T-0056 (gold pill, cards, section header, empty states). Parent screens favour density/scannability over playfulness — same palette, lighter on the game flourishes. `PairingScreen` lives in `:pairing`, which depends on `:sdk`/compose but not `:app` — so either move the theme into `:pairing` (shared) or pass colors; simplest is to give `:pairing` its own light wrapper that matches, or have the app theme it via `MaterialTheme` it's already inside.

### Dependencies
[[SQUIRE-T-0056]] (theme + components), [[SQUIRE-T-0040]] (Knight screen), [[SQUIRE-T-0046]] (pairing screen).

### Risk Considerations
`:pairing` is a separate module (used before the app theme is necessarily applied) — make sure the pairing screen still looks right (it's rendered inside the app's theme today, so it should inherit). Keep the review queue scannable — don't over-decorate the parent's working surface.

## Status Updates

*To be added during implementation*
