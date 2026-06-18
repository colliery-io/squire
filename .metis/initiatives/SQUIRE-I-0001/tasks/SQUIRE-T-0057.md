---
id: ui-polish-2-parent-knight-review
level: task
title: "UI polish 2: parent (Knight) review + assume + pairing screens restyle"
short_code: "SQUIRE-T-0057"
created_at: 2026-06-18T03:10:00+00:00
updated_at: 2026-06-18T03:29:52.215746+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# UI polish 2: parent (Knight) review + assume + pairing screens restyle

## Parent Initiative

[[SQUIRE-I-0001]] · Applies the [[SQUIRE-T-0056]] theme to the parent-facing surfaces · **blocked_by [[SQUIRE-T-0056]]**

## Objective

Propagate the "playful quest" theme/components (from T-0056) to the **parent** surfaces — keeping them clean and scannable (a management tool), just themed and tidied: the Knight review home, the per-Squire rows + Add-funds dialog, the "Open / Acting as" flow, and the first-run **pairing screen**.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] `KnightHomeScreen` restyled with the shared components: per-Squire cards (name + gold balance + Open/Mark-done/Redeem/Add-funds), pending-claim and pending-redemption cards with clear Approve/Reject, a themed offline + update banner. Add-funds dialog tidied.
- [x] The **pairing screen** (`:pairing/PairingScreen`) restyled: a friendly themed onboarding (title/crest, clear Scan QR primary + manual fields + Discover + debug button), consistent inputs/buttons.
- [x] The "Acting as <name>" assumed-Squire view (reuses the themed `PlayerHomeScreen`) reads cleanly with the parent-context header/back.
- [x] `:app:assembleDebug` builds; behaviour unchanged; screenshots captured (Knight home, pairing).

## Implementation Notes

### Technical Approach
Reuse the `SquireTheme` + components from T-0056 (gold pill, cards, section header, empty states). Parent screens favour density/scannability over playfulness — same palette, lighter on the game flourishes. `PairingScreen` lives in `:pairing`, which depends on `:sdk`/compose but not `:app` — so either move the theme into `:pairing` (shared) or pass colors; simplest is to give `:pairing` its own light wrapper that matches, or have the app theme it via `MaterialTheme` it's already inside.

### Dependencies
[[SQUIRE-T-0056]] (theme + components), [[SQUIRE-T-0040]] (Knight screen), [[SQUIRE-T-0046]] (pairing screen).

### Risk Considerations
`:pairing` is a separate module (used before the app theme is necessarily applied) — make sure the pairing screen still looks right (it's rendered inside the app's theme today, so it should inherit). Keep the review queue scannable — don't over-decorate the parent's working surface.

## Status Updates

**2026-06-17 — Done (all three parent surfaces restyled + verified live).** Applied the "playful quest" theme to the parent UI:
- **`KnightHomeScreen`** (`:app`): royal app bar "🛡 The Round Table" (serif), `SectionTitle` headers ("Your Squires · N awaiting your seal", "Quests to approve", "Rewards to grant"), parchment Squire **cards** (⚔ name + `GoldPill` balance + tonal "Open" + outlined Mark done/Redeem/Add ★), review cards with a kind `StatusChip` (Quest/Reward) + herald-green **Approve ✓** / outlined Reject, friendly empty states, themed offline `Banner`. Reuses `:app`'s `SquireUi` + `SquireTheme`.
- **`PairingScreen`** (`:pairing`, shared — can't import `:app`; uses raw Material tokens but inherits `SquireTheme`): shield crest + "Welcome, traveller" royal serif headline, royal "📷 Scan the QR", a parchment "Enter it by hand" card wrapping the form, errors surfaced in an `errorContainer` card.
- **Assume-Squire view**: already renders the themed `PlayerHomeScreen` ("⚔ Acting as Gawain") — inherits the T-0056 restyle, no change needed.

`:app:assembleDebug` SUCCESS. Verified live on the emulator: captured the pairing screen, then minted a Knight pairing code against the demo server (login user 1 → POST /pair/codes) and paired as Knight to capture the parent home + the "Open"→assume view. Pure presentation — no callback/transport/`:core` change. Screenshots sent for sign-off. Next: SQUIRE-T-0058 (app icon + Keep web UI), then bump versionCode and package one OTA update.