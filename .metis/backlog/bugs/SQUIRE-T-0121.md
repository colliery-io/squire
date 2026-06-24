---
id: knight-history-show-names-and
level: task
title: "Knight history: show names and subjects, not raw ids (actor + which quest/reward)"
short_code: "SQUIRE-T-0121"
created_at: 2026-06-24T01:17:27.053252+00:00
updated_at: 2026-06-24T01:48:36.477833+00:00
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

# Knight history: show names and subjects, not raw ids (actor + which quest/reward)

## Objective

Make Knight history entries read like sentences a parent understands: **who** did it (by name,
including the acting Knight), to **whom** (by name), and **what about** (the quest/reward/achievement
by title). Today entries leak integer ids and omit the subject — e.g. "Matrim earned 2 coins" with
no quest, and an acting Knight rendered as a bare number.

> Collapsed ticket — covers both the "id instead of name" and "missing subject" defects; they share
> one fix surface (`crates/api/src/history.rs` + Android `HistoryScreen.kt`) and should be one pass.

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P2 - Medium (nice to have)

### Impact Assessment
- **Affected Users**: Any Knight reading the history feed.
- **Reproduction Steps**:
  1. As a Knight, approve a child's quest claim (and/or add funds), then open History.
  2. Entry reads "<name> earned N coins" with no quest named; actor not shown; a bare id may appear.
- **Expected vs Actual**: Expected "Matrim earned 2 coins for *Make your bed* — approved by Dad";
  actual is "earned 2 coins" with no subject and no/!d actor.

## Verified behavior (root cause)

Two defects, same surface. Neither is in the server's behavior proper — both are a thin DTO that
ships ids and a client that doesn't (and sometimes can't) resolve them.

**(A) Names shown as ids.**
- The **subject squire** *does* resolve — `HistoryScreen` gets `squireNames: Map<Long,String>` from
  the household-review (`KnightHomeHost.kt:217-226`, `KnightHomeScreen.kt:221`) and renders
  `squireNames[e.squire] ?: "Squire ${e.squire}"` (`HistoryScreen.kt:87`).
- The **actor** (acting Knight) is **never rendered** — `HistoryEntryDto.actor: Long?` is populated
  server-side (`crates/api/src/history.rs:72,77,82,…`) and in the SDK (`HistoryEntryDto.kt:58-59`),
  but `describe()` ignores it (`HistoryScreen.kt:124-146`).
- The id **leak**: `squireNames` comes from `review.squires`, filtered to **active && Role::Squire**
  (`crates/api/src/knight.rs:548-558`). Any id outside that set — the Knight, or an inactive/renamed
  member referenced by old events — falls back to `"Squire <id>"`.

**(B) Missing subject (which quest/reward).**
- `describe()` emits generic strings (`HistoryScreen.kt:127-132`): `"Approved" -> "$name earned
  ${amt} coins"`, `"Claimed" -> "$name claimed a quest"`, `"Redeemed" -> "…redeemed a reward"`.
- The DTO carries only ids, no titles (`history.rs` `HistoryEntryDto` ~`:31-48`). **Worse:** the
  `Approved` arm of `from_event` (`history.rs:71-75`) doesn't even set `quest_id` (domain
  `CompletionApproved` is keyed by `claim_id`), so the earn event has *no* quest link on the wire at
  all. `Claimed` carries `quest_id` (`:67-70`) but no title, and the client never maps it.
- The client holds `review.quests`/`review.items` lookup tables but only threads `squireNames` into
  `HistoryScreen`.

## Acceptance Criteria

## Acceptance Criteria

- [x] Every person shown resolves to a display name — subject **and** actor — including
      inactive/renamed members referenced by historical events. No bare integers / `"Squire <id>"`.
- [x] The acting Knight is surfaced where meaningful (e.g. "approved by Dad").
- [x] Approve/earn entries name the quest; redeem entries name the reward; achievement entries name
      the achievement.
- [x] The `Approved` entry is linked back to its quest (currently carries no quest reference).
- [x] Existing `reason` display (rejections/adjustments) is preserved.

## Implementation Notes

### Technical Approach (resolve server-side — one pass)
Mirror the snapshot-label pattern `pending_claims`/`pending_requests` already use
(`crates/api/src/knight.rs:562-617`):
- `crates/api/src/history.rs`: enrich `HistoryEntryDto` with `actor_name`, `squire_name`,
  `quest_title`, `item_title` (and achievement name) — all `Option`. Make `from_event`
  snapshot-aware (it currently takes only `&Event`); resolve people from `snap.users` **without** the
  active/role filter, and titles from `snap.quests`/`snap.items`. **Backfill the quest on the
  `Approved` arm** via the originating `CompletionClaimed` (`claim_id` lookup) or thread it through.
  The `history` handler already locks a snapshot (`history.rs:139-147`).
- Android `HistoryScreen.kt` `describe()`: use names + titles — e.g. `"$name earned $amt coins for
  \"$questTitle\""`, `"$name redeemed \"$itemTitle\""`, and surface the actor.
- Regen `crates/api/openapi.json` + Kotlin SDK; new fields nullable for wire back-compat.

### Test Cases
- Rust: an `Approved` entry carries `quest_title`; a redemption carries `item_title`; `actor_name`
  resolves for a known Knight and is `None` when `actor` is `None`; an inactive member still
  resolves (no `"Squire <id>"`).
- SDK decode test: payload with and without the new fields (back-compat).
- Android: a history snapshot with an `Approved` entry asserting the quest title + actor appear.

## Status Updates

### 2026-06-24 — implemented & verified

**Server (resolve once, from the snapshot):** `HistoryEntryDto` gained `squire_name`, `actor_name`,
`quest_title`, `item_title`, `achievement_name` (all `Option`). `from_event` is now snapshot-aware:
resolves people across **all** `snap.users` (no active/role filter, so renamed/inactive members still
resolve), titles from `snap.quests`/`snap.items`/`snap.achievements`, and **backfills the quest on
`Approved`** via `claim_meta(snap, claim_id)` (re-exported from `domain-core`) since
`CompletionApproved` is claim-keyed. One lock for `recent_events` + `snapshot`. Re-froze
`openapi.json`; regenerated the Kotlin SDK (new fields nullable).

**Android:** `describe()` uses the resolved names/titles — "earned N coins for “quest” · approved by
Dad", "redeemed “reward” · by Dad"; `HistoryScreen` prefers `e.squireName`, falling back to the
review map then the id.

**Verification:** 3 Rust unit tests in `history.rs` (actor + quest-backfill on approval, claim/redeem
subjects, inactive-member still resolves). Full `api` + `domain-core` suites + openapi drift green.
Paparazzi `knightHistory` re-recorded + verify green — confirmed "Gawain earned 5 coins for “Make
your bed” · approved by Dad". All acceptance criteria met.
</content>