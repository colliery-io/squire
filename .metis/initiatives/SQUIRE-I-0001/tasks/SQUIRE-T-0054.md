---
id: merge-squire-knight-into-one-app
level: task
title: "Merge Squire + Knight into one app keyed by the paired role"
short_code: "SQUIRE-T-0054"
created_at: 2026-06-17T23:00:00+00:00
updated_at: 2026-06-18T01:08:39.841789+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

## Acceptance Criteria

## Acceptance Criteria

- [x] One application module `:app` (`com.squire.app`) depending on `:core` + `:knight-core` + `:sdk` + `:pairing`; `:knight-app` removed from `settings.gradle.kts` and deleted — its UI/transport/Room sources `git mv`'d into `:app` (kept under `com.squire.knight.app`), the Knight `MainActivity` stripped to an `internal KnightHomeHost` composable.
- [x] `MainActivity` is session-gated then **role-routed**: no session → `PairingScreen`; `session.role == "Knight"` → `KnightHomeHost`; else → `PlayerHomeHost`. Each `…HomeHost` is now self-contained (builds its own Room db / json / ids) and keeps auto-refresh + relocate + update-banner.
- [x] Both Room DBs coexist (`squire.db` / `knight.db`), each built only inside its role's host. Pairing/Forget/offline-first/auto-refresh/relocate preserved; the update banner now uses the single `"squire"` manifest key.
- [x] One signed release APK builds (`:app:assembleRelease`, R8, `CN=Squire` VALID, ~1.9 MB) and `:core`/`:knight-core`/`:sdk` tests green. **Live (merged debug app, one APK)**: paired as **Knight (user 1)** → review home ("Knight — review"); Forget; paired as **Squire (user 2)** → child home ("Squire — 0 pts"). RUNBOOK updated to one app / one manifest entry.

## Implementation Notes

### Technical Approach
Move `knight-app/src/main/.../com/squire/knight/app/**` into `:app` (e.g. `com.squire.app.knight`), or convert `:knight-app` to a UI library `:app` depends on. Extract each role's host as a composable (`PlayerHomeHost`, `KnightHomeHost`) — already mostly there — and have one `MainActivity` choose by `session.role` ("Knight" vs "Squire"). Keep both Room DBs; build the right store per role. Drop the demo-bypass `demoLogin` user split or pick role by a debug toggle. The release update-manifest key becomes just `squire` (one app); update `UpdateChecker` to a single key.

### Dependencies
[[SQUIRE-T-0040]] (Knight app), [[SQUIRE-T-0037]] (Squire app), [[SQUIRE-T-0046]] (`:pairing`/session role), [[SQUIRE-T-0049]] (release signing), [[SQUIRE-T-0051]]/[[SQUIRE-T-0052]] (banner/relocate to preserve).

### Risk Considerations
Big client refactor — keep `:core`/`:knight-core` pure (UNchanged). Package moves risk import churn; do it module-by-module and keep the app building. The merged app contains both UIs, so the only role enforcement left on-device is "render by session role" — fine because the **server** enforces `RequireKnight` on every privileged call (a Squire session can't successfully call them even if code existed). Demo-bypass: needs a role choice now (debug only). iOS unaffected (none yet).

## Status Updates

**2026-06-18 — Done.** `git mv`'d `knight-app/src/main/kotlin/com/squire/knight` into `:app`; renamed the Knight `MainActivity.kt` → `KnightHomeHost.kt` and stripped it to an `internal @Composable KnightHomeHost(session, discovery, onSessionChanged, onForget)` that builds its own `knight.db`/json/ids (uses `com.squire.app.BuildConfig`, update key `"squire"`). Refactored the Squire `PlayerHomeHost` the same self-contained way. The merged `MainActivity` is tiny: session-gate → `when` on `session.role` ("Knight" → `KnightHomeHost`, else `PlayerHomeHost`). Added `implementation(project(":knight-core"))` to `:app`, removed `:knight-app` from settings + deleted the module. `:core`/`:knight-core` unchanged (pure).

**Verified**: merged debug app builds; paired as Knight (user 1) → review home, Forget, paired as Squire (user 2) → child home — both from the **one** APK. Signed release builds (R8, valid signature). Cores + SDK green. Trust boundary now fully server-side (`RequireKnight`); the app routes UI by `session.role`. Demo bypass pairs as the Squire (user 2); pair manually as user 1 for the Knight UI. Unblocks the assume-Squire mode ([[SQUIRE-T-0055]]).