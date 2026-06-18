---
id: merge-squire-knight-into-one-app
level: task
title: "Merge Squire + Knight into one app keyed by the paired role"
short_code: "SQUIRE-T-0054"
created_at: 2026-06-17T23:00:00.000000+00:00
updated_at: 2026-06-17T23:00:00.000000+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Merge Squire + Knight into one app keyed by the paired role

## Parent Initiative

[[SQUIRE-I-0001]] · Specs [[SQUIRE-S-0005]]/[[SQUIRE-S-0006]] · supersedes the two-app split from [[SQUIRE-T-0040]]

## Objective

Replace the two separate apps (`:app` Squire / `:knight-app` Knight) with **one app** that shows the
child or parent UI based on the **paired session's role** (`Session.role`). The role/trust boundary
moves fully to the server — every privileged call is still `RequireKnight`-gated — so a Squire-paired
device simply never renders the parent UI (and the server rejects privileged calls regardless). This
reverses the deliberate binary split (the server was always the real gate; the binary separation was
defence-in-depth) per an explicit product decision.

## Acceptance Criteria

- [ ] One application module (`com.squire.app`) depending on `:core` + `:knight-core` + `:sdk` + `:pairing`; the `:knight-app` application module is removed from `settings.gradle.kts`/the build (its UI/transport/Room code folds into the merged app under a `knight` package).
- [ ] `MainActivity` is session-gated then **role-routed**: no session → `PairingScreen`; session role `Squire` → the player home; role `Knight` → the review home. Each role's `…HomeHost` (transport + Room + auto-refresh + relocate + update-banner) is preserved.
- [ ] Both Room DBs coexist (`squire.db` / `knight.db`), each used only by its role's store. Pairing, Forget, offline-first, auto-refresh, the update banner (T-0051; one manifest entry now), and relocation (T-0052) all work in the merged app for both roles.
- [ ] One signed release APK builds (`assembleRelease`), installs, and: pairing as a Squire → child home; pairing as a Knight → review home. `:core`/`:knight-core`/`:sdk` tests green. The update manifest is reduced to a single app entry (`squire`).

## Implementation Notes

### Technical Approach
Move `knight-app/src/main/.../com/squire/knight/app/**` into `:app` (e.g. `com.squire.app.knight`), or convert `:knight-app` to a UI library `:app` depends on. Extract each role's host as a composable (`PlayerHomeHost`, `KnightHomeHost`) — already mostly there — and have one `MainActivity` choose by `session.role` ("Knight" vs "Squire"). Keep both Room DBs; build the right store per role. Drop the demo-bypass `demoLogin` user split or pick role by a debug toggle. The release update-manifest key becomes just `squire` (one app); update `UpdateChecker` to a single key.

### Dependencies
[[SQUIRE-T-0040]] (Knight app), [[SQUIRE-T-0037]] (Squire app), [[SQUIRE-T-0046]] (`:pairing`/session role), [[SQUIRE-T-0049]] (release signing), [[SQUIRE-T-0051]]/[[SQUIRE-T-0052]] (banner/relocate to preserve).

### Risk Considerations
Big client refactor — keep `:core`/`:knight-core` pure (UNchanged). Package moves risk import churn; do it module-by-module and keep the app building. The merged app contains both UIs, so the only role enforcement left on-device is "render by session role" — fine because the **server** enforces `RequireKnight` on every privileged call (a Squire session can't successfully call them even if code existed). Demo-bypass: needs a role choice now (debug only). iOS unaffected (none yet).

## Status Updates

*To be added during implementation*
