---
id: live-auto-refresh-for-both-phone
level: task
title: "Live/auto-refresh for both phone apps (foreground polling, no manual Refresh)"
short_code: "SQUIRE-T-0041"
created_at: 2026-06-17T17:25:00+00:00
updated_at: 2026-06-17T18:11:40.706302+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Live/auto-refresh for both phone apps (foreground polling, no manual Refresh)

## Parent Initiative

[[SQUIRE-I-0001]] · Specs: [[SQUIRE-S-0005]] (Squire) + [[SQUIRE-S-0006]] (Knight) · siblings [[SQUIRE-T-0037]]/[[SQUIRE-T-0040]]

## Objective

Today both phone apps only refresh on launch/login and the manual **Refresh** button. The user flagged the friction: a claim the child submits doesn't appear in the parent's Knight queue (and an approval doesn't reflect on the child's home) until someone taps Refresh. Make the views feel **live**: while the app is foregrounded, automatically poll `syncNow()` on a cadence so queued work flushes and fresh server state appears without manual action — staying within the offline-first contract (a failed poll silently degrades to cache, never an error).

## Decision (MVP)

**Foreground polling**, not server push. Rationale: LAN-only, intermittent reachability (NFR-6), and the existing `syncNow()` (flush-then-refetch, never throws) already does exactly one poll. A lifecycle-aware loop that calls `syncNow()` every N seconds while `STARTED` and stops when backgrounded is the smallest change that removes the friction, with no server work. True push (SSE/long-poll/WebSocket over the LAN api) is a larger, separate effort — capture as a follow-up only if polling proves insufficient.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] A lifecycle-aware auto-refresh loop in each app (`:app` Squire and `:knight-app`) that calls the existing `syncNow()` (via the ViewModel) every 5s **only while the UI is in the foreground** (`Lifecycle.State.STARTED`), and cancels when backgrounded — `repeatOnLifecycle` handles the cancel, so no background drain.
- [x] A poll that fails (Keep unreachable) does **not** surface an error or clobber a good view: the offline-first `refresh()` keeps the cached view; the stores only set `Loading` once at init then go Ready↔Ready, so polls update in place (no flicker).
- [x] Manual **Refresh** still works and is now redundant-but-harmless; the offline banner still appears when serving from cache.
- [x] Verified live on the emulator: with the Knight open and untouched, a Squire redemption request appeared in the Knight queue within one poll interval (no manual Refresh). `:app`/`:knight-app` assemble; `:core`/`:knight-core` tests unaffected.

## Implementation Notes

### Technical Approach
Use `repeatOnLifecycle(Lifecycle.State.STARTED) { while (true) { viewModel.refresh(); delay(POLL_MS) } }` in each `MainActivity`'s composition (or a `LaunchedEffect` keyed on the lifecycle). `refresh()` already maps to `syncNow()` (flush outbox → refetch) and never throws. Keep the interval a single named constant (~5s) so it's easy to tune. No contract/server change.

### Dependencies
The offline-first stores from [[SQUIRE-T-0037]] (Squire) and [[SQUIRE-T-0039]]/[[SQUIRE-T-0040]] (Knight) — both already expose `syncNow()`.

### Risk Considerations
Avoid the Ready→Loading flicker: the stores only set `Loading` once at init, then transition Ready↔Ready, so in-place updates are fine. Watch battery: the `STARTED`-scoped loop must cancel on background (lifecycle scope handles this). If a 5s cadence feels heavy, widen it; this is a comfort feature, not a correctness one. A future server-push variant (SSE) would be a separate task + likely an ADR.

## Status Updates

**2026-06-17 — Done.** Added a foreground poll loop to both `MainActivity`s: inside the startup `LaunchedEffect`, after the one-time login, `lifecycle.repeatOnLifecycle(Lifecycle.State.STARTED) { while (true) { viewModel.refresh().join(); delay(AUTO_REFRESH_MS) } }` with `AUTO_REFRESH_MS = 5_000`. Made `SquireViewModel.refresh()` return its `Job` (Knight's already did) so the loop paces on completion. No server/contract change. Both apps assemble. Live proof: with the Knight open and untouched, a `POST /redemption-requests` by the Squire surfaced as "Ice cream · Gawain · 3 pts" with Approve/Reject within one interval — no Refresh tap (`/tmp/knight-autorefresh.png`). Manual Refresh retained. Deferred (separate tasks): real server-push (SSE) if polling proves insufficient.