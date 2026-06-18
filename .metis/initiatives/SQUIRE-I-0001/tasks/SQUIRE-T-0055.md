---
id: knight-assume-squire-mode-in-the
level: task
title: "Knight 'assume Squire' mode in the merged app (pick a Squire, operate their home)"
short_code: "SQUIRE-T-0055"
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

- [ ] From the Knight review home, each Squire row (or a dedicated picker) has an **"Open / Act as"** action that navigates into that Squire's player home, fetched via `GET /admin/squire/{id}/state` (T-0053). A back action returns to the review home.
- [ ] In that view, the existing `PlayerHomeScreen` renders the assumed Squire's `StateView`; **Mark done** → `KnightViewModel.markDone(squireId, questId, on)`; **Redeem** → `KnightViewModel.redeem(squireId, itemId)`. The view refreshes after an action (and on the auto-refresh cadence) to show the new balance/state.
- [ ] Actions remain Knight-attributed (single-writer/audit intact); offline-first holds (the assumed-Squire read caches/falls back like the rest). No new server write path (uses the privileged commands that already exist).
- [ ] Merged app assembles; `:knight-core` (+ any new wiring) tests green. Verified on the emulator: pair as Knight → pick a Squire → see their home → Mark done credits that Squire (balance updates) → back to review.

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

*To be added during implementation*
