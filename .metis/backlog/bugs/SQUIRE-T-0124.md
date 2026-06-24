---
id: squire-recent-activity-is-grouped
level: task
title: "Squire 'Recent activity' is grouped by type, not ordered by recency"
short_code: "SQUIRE-T-0124"
created_at: 2026-06-24T01:17:31.096951+00:00
updated_at: 2026-06-24T01:58:58.961756+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Squire 'Recent activity' is grouped by type, not ordered by recency

## Objective

Make the child "Recent activity" feed a single list ordered strictly newest-first by event time,
interleaving adjustments, claims, and redemptions — instead of three type-grouped blocks.

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P2 - Medium (nice to have)

### Impact Assessment
- **Affected Users**: Every child viewing the Activity tab.
- **Expected vs Actual**: Expected most-recent-first across all activity; actual shows all
  adjustments, then all claims, then all requests — so a coin grant from this morning sorts above a
  quest approved five minutes ago.

## Verified behavior (root cause)

The feed is **three separate StateView lists rendered back-to-back, never merged or globally
sorted** — `clients/squire-android/app/src/main/kotlin/com/squire/app/ui/PlayerHomeScreen.kt:349-359`:

```
items(adjustments,     …) { AdjustmentRow(it) }   // view.adjustments
items(view.myClaims,   …) { ClaimRow(it) }        // view.myClaims
items(view.myRequests, …) { RequestRow(it) }      // view.myRequests
```

Each list is newest-first *within itself* (server `events.iter().rev()`), but they're concatenated
by type. No client sort exists (`SquireViewModel.kt`/`SquireApiAdapter.kt` have no `sortedBy`).

**Blocking sub-issue — two DTOs can't even be time-sorted:** `ClaimStatus`
(`crates/domain-core/src/contract/api.rs:193-198`) and `RedemptionStatus` (`:238-243`) carry **no
timestamp** — only ids. `claim_id`/`request_id` are **phone-minted** (`api.rs:275-280`), so they are
not monotonic with event time and can't be used as a time proxy. Only `AdjustmentView` carries `at`
(`:80`). So a correct global sort is impossible with the current wire shape.

(Note: `view.myCashouts` is assembled server-side but not rendered in this feed at all — decide
whether it belongs.)

## Acceptance Criteria

## Acceptance Criteria

- [x] "Recent activity" is one list, strictly descending by event timestamp, across all entry types.
- [x] Each surfaced entry carries an event timestamp (the new `ActivityEntry.at`).
- [x] A unified `RECENT_LIMIT` cap applies across the merged list, not per-type.

## Implementation Notes

### Technical Approach (recommended: merge + sort server-side)
1. Carry timestamps: add `at: Timestamp` to `ClaimStatus` and `RedemptionStatus` (`api.rs:193`,
   `:238`), sourced from the originating `CompletionClaimed`/`RedemptionRequested` event in
   `my_claims` (`squire.rs:559`) / `my_requests` (`squire.rs:602`). `#[serde(default)]` for
   back-compat.
2. Prefer a single `recent_activity: Vec<ActivityEntry>` on `StateView` — a tagged union
   {Adjustment | Claim | Request} each with `at` — built in `assemble_state` (`squire.rs:242-275`)
   by collecting candidates, sorting by `at` descending, then `take(RECENT_LIMIT)` (`:298`). A merged
   list is needed for a true "10 most recent across all types" (today each list is capped
   independently).
3. Regen `crates/api/openapi.json` + Kotlin SDK.
4. Android `PlayerHomeScreen.kt:349-359`: render the single merged list (replace the three `items`
   blocks).

### Test Cases
- Rust over `assemble_state`: append events interleaved in time (adjustment@T1, claim@T2,
  request@T3, adjustment@T4) and assert the feed returns `[T4, T3, T2, T1]` — an adjustment correctly
  sandwiched between a request and a claim. The current code cannot satisfy this.
- Android: assert rendered row order is descending by `at`.

## Status Updates

### 2026-06-24 — implemented & verified

**Chosen approach: merge server-side (no client sort needed).** The event log is already in
chronological order, so a new `recent_activity(snap, squire)` walks it `.rev()`, emits one tagged
`ActivityEntry` per activity event (adjustment / claim / request), and `take(RECENT_LIMIT)` — yielding
the global newest-first feed with a single unified cap, no explicit sort. (Walking a per-type-capped
set would also be correct since any global-top-N item is in its own type's top-N, but the single-walk
is simpler and exact.)

- `domain-core/contract/api.rs`: new `ActivityEntry { at, kind, adjustment?, claim?, request? }` +
  `ActivityKind` (flat-tagged, codegen-friendly); `StateView.recent_activity` (`#[serde(default)]`).
  The legacy `adjustments`/`my_claims`/`my_requests` lists stay for older clients.
- Registered both schemas in `openapi.rs`; re-froze `openapi.json`; regenerated the Kotlin SDK.
- Android `PlayerHomeScreen.kt` Activity tab renders `view.recentActivity` via `when(kind)`, reusing
  the existing `AdjustmentRow`/`ClaimRow`/`RequestRow`; falls back to the legacy three-list rendering
  for an older server.

**Verification:** new Rust test `recent_activity_is_one_globally_time_ordered_feed` — events at
T100/200/300/400 (adjust/claim/request/adjust) come back `[400,300,200,100]` with an adjustment
sandwiched between a request and a claim. Full `api`+`domain-core` suites + openapi drift green.
Paparazzi `squirePlayerHomeHistory` fixture given a merged feed, re-recorded + verify green —
confirmed the Activity tab interleaves: "Take out the trash" (pending) → "gophering" (+10) → "Movie
night" (rejected) → "Tidy your room" (+10). All acceptance criteria met.
</content>