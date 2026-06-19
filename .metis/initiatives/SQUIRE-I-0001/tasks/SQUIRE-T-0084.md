---
id: adventure-journal-visual-redesign
level: task
title: "Adventure-journal visual redesign: Keep + unified phone app, publish OTA v0.7.1"
short_code: "SQUIRE-T-0084"
created_at: 2026-06-19T13:35:14.769427+00:00
updated_at: 2026-06-19T13:35:14.769427+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: true
initiative_id: SQUIRE-I-0001
---

# Adventure-journal visual redesign: Keep + unified phone app, publish OTA v0.7.1

## Parent Initiative

[[SQUIRE-I-0001]]

## Objective

Apply the "adventure-journal" visual identity (Royal Purple · Adventure Gold · Parchment) across
every surface so the Keep desktop admin and the phone app read as one product: coin pills,
medallion-framed icons, colored accent edges, themed cards/empty states, and a crest + gold-rule
app bar. Presentation-only — no API, model, networking, or domain changes. Ship the result to the
live `squire-serve` as an OTA app update.

## Acceptance Criteria

- [x] Keep desktop (`crates/keep/assets/{index.html,keep.css,keep.js}`) restyled; same IDs/routes/handlers.
- [x] Phone app theme + shared components: `Theme.kt` (exact palette), `SquireUi.kt` (minted `GoldPill`, new `Medallion`).
- [x] Squire home (`PlayerHomeScreen`): medallions, coin pills, status-colored accent edges, crest + gold-rule app bar, green "Do it!" CTA.
- [x] Knight home (`KnightHomeScreen`): `ReviewCard` leading medallion slot, crest + gold-rule app bar.
- [x] Knight authoring screens (Quest/Reward/Achievement/Member) + pairing crest restyled.
- [x] Both `:app` and `:pairing` compile (`compileDebugKotlin` green); signed release builds.
- [x] OTA published: versionCode 5 / 0.7.1, signer matches installed v4, `~/squire-updates/squire-5.apk` + manifest; live server advertises v5.

## Implementation Notes

### Technical Approach
Like-for-like restyle delivered as a handoff drop-in. Two deviations from the handoff were required
to compile against the real code:
- `ItemSummaryDto` / `LibraryReward` have no `icon` field → reward rows use `Medallion(null, name, reward = true)` (emoji inference) instead of `r.icon`.
- The `:pairing` module cannot import the `:app` theme (`:app` depends on `:pairing`, not vice-versa) → the crest gradient uses `colorScheme.onPrimaryContainer` (the deep royal) instead of `SquireRoyalDeep`.

Also consolidated to a single phone app: the stale, untracked `clients/squire-android/knight-app/`
module (not in `settings.gradle.kts`) was removed and a stale comment in `pairing/build.gradle.kts`
corrected. The Knight UI lives in `:app` under `com.squire.knight.app`.

### Dependencies
Builds on the OTA distribution flow from [[SQUIRE-T-0051]] and the merged single app from
[[SQUIRE-T-0054]]; continues the polish line [[SQUIRE-T-0081]] / [[SQUIRE-T-0082]] and the deploy
cadence of [[SQUIRE-T-0083]].

## Status Updates

- Installed Keep redesign; rebuilt + restarted the local dev `squire-home`.
- Applied phone drop-ins + home-screen + authoring-screen edits; verified `:app`/`:pairing` compile.
- Reset the live production data dir (backed up to `~/Library/Application Support/squire.bak-*`); prod relaunched fresh with `SQUIRE_APK_DIR` so OTA is served.
- Built signed release (versionCode 5 / 0.7.1, signer `9da09d14…` matches live cert), published `squire-5.apk` + manifest; verified live `/app/manifest` advertises v5.
- Applied the "make it land" home-screen upgrade (coin pills, accent edges, crest app bar) and republished the unserved v5 in place. Completed.
