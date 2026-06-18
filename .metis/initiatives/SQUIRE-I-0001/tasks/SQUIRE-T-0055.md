---
id: knight-assume-squire-mode-in-the
level: task
title: "Knight 'assume Squire' mode in the merged app (pick a Squire, operate their home)"
short_code: "SQUIRE-T-0055"
created_at: 2026-06-17T23:00:00+00:00
updated_at: 2026-06-18T01:17:37.801867+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Knight 'assume Squire' mode in the merged app (pick a Squire, operate their home)

## Parent Initiative

[[SQUIRE-I-0001]] · Spec [[SQUIRE-S-0006]] · **blocked_by [[SQUIRE-T-0053]]** (server read) + [[SQUIRE-T-0054]] (merged app)

## Objective

Let a Knight **assume any Squire**: from the review home, pick a Squire and drop into that Squire's
full player home (their quests/rewards/streaks/balance), operating it on their behalf. Reuses the
child `PlayerHomeScreen` for display (fed by the Knight-gated state read, T-0053) with its actions
wired to the **Knight's privileged surface** — "Mark done" → Knight mark-done for that Squire,
"Redeem" → Knight direct-redeem — so everything is audited as the Knight (no impersonation token).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] Each Squire row in the review home has an **"Open"** button → navigates into that Squire's player home, fetched via `KnightApi.squireState` (`GET /admin/squire/{id}/state`, T-0053). A **Back** action (top bar) returns to the review home.
- [x] The reused `PlayerHomeScreen` renders the assumed `StateView` with header **"Acting as <name>"**; **Mark done** → `KnightViewModel.markDone(squireId, questId, on)`; **Redeem** → `KnightViewModel.redeem(squireId, itemId)`. A `tick` counter refetches after each action and on the auto-refresh cadence.
- [x] Actions are Knight-attributed (the existing privileged commands — single-writer/audit intact); no new write path. The assumed read fails gracefully to an error+Retry (it's a Knight-token'd live read; offline shows the error, not a stale cache — noted as acceptable for MVP).
- [x] Merged app assembles (+ signed release); `:core`/`:knight-core`/`:sdk` + `cargo test --workspace` green. **Verified live**: Knight → Open Gawain → "Acting as Gawain — 0 pts" → Mark done "Make your bed" → server credits Gawain **+5**, header → "Acting as Gawain — 5 pts", claim shows Approved (+5) → Back → review home.

## Implementation Notes

### Technical Approach
Add a Knight-side store/fetcher for the assumed Squire: a `ReviewFetcher`-style call to
`KnightApi.squireState(household, id)` producing a `StateView`, cached offline-first (a small
`AssumedSquireStore` or reuse the `PlayerStore` shape with a Knight-token fetcher + read-only outbox-
less path). Wire `PlayerHomeScreen`'s `onMarkDone`/`onRedeem` to the `KnightViewModel`'s
`markDone`/`redeem` for the selected squire id. Navigation: a simple `selectedSquire: Long?` state in
the Knight `…HomeHost` switching between the review list and the assumed home. Keep it read-via-Knight
+ act-via-Knight; no squire token.

### Dependencies
[[SQUIRE-T-0053]] (the state read), [[SQUIRE-T-0054]] (merged app + Knight UI present), [[SQUIRE-T-0040]] (Knight `markDone`/`redeem`), [[SQUIRE-T-0034]] (`PlayerHomeScreen`).

### Risk Considerations
Reusing the child `PlayerHomeScreen` with Knight-backed actions keeps UI duplication low but the
actions differ semantically (a Knight acting *for* a Squire) — make the screen header clearly say
"Acting as <name>" so it's not confused with the Knight's own view. The assumed-Squire read is
Knight-token'd (T-0053). Offline: a cached assumed-Squire view is fine; queuing offline acts uses the
Knight privileged outbox already. Out of scope: literal squire-token impersonation, per-squire
notifications.

## Status Updates

**2026-06-18 — Done.** `KnightApiAdapter.squireState(id)` (over `KnightApi.squireState`); `PlayerHomeScreen` gained optional `headerLabel` + `onBack` (Back replaces Forget in assume mode); `KnightHomeScreen.SquireRow` gained an **"Open"** button + `onOpenSquire` callback. `KnightHomeHost` hoists the adapter (shared by the review store + the assume read), holds an `assumed: Pair<Long,String>?` state, and renders `AssumedSquireHome` when set — a `PlayerHomeScreen` fed by `squireState`, actions wired to `KnightViewModel.markDone`/`redeem` for that squire, refetching via a `tick` after each action + on cadence.

**Live**: paired as Knight → Open Gawain → "Acting as Gawain — 0 pts" → Mark done "Make your bed" → server `balance 0→5` (the `/admin/squire/2/state` read confirmed it), header → "Acting as Gawain — 5 pts", Recent claims shows Approved (+5), Ice cream now affordable → Back → review home. (Hit the recurring gotcha again — had to rebuild/restart `squire-home` for the T-0053 route; the app correctly showed an error+Retry until then, then loaded on Retry.) Closes the merge+assume work (T-0053/54/55). `cargo test --workspace` green; merged signed release builds.