---
id: knight-history-omits-the-subject
level: task
title: "Knight history omits the subject (which quest/reward) of each action"
short_code: "SQUIRE-T-0125"
created_at: 2026-06-24T01:17:31.971332+00:00
updated_at: 2026-06-24T01:17:31.971332+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/backlog"
  - "#bug"


exit_criteria_met: false
initiative_id: NULL
---

# Knight history omits the subject (which quest/reward) of each action

## Objective

Make Knight history entries say *what* the action was about: "Matrim earned 2 coins for **Make your
bed**", "redeemed **Extra screen time**" — not just "earned 2 coins" / "redeemed a reward".

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P2 - Medium (nice to have)

### Impact Assessment
- **Affected Users**: Any Knight reading history.
- **Reproduction Steps**:
  1. Approve a child's quest claim, then open the Knight History feed.
  2. Entry reads "<name> earned N coins" with no quest named.
- **Expected vs Actual**: Expected the quest/reward subject on each entry; actual shows only the verb
  and amount (the free-text `reason` is shown for rejections/adjustments, but no subject for the
  common earn/claim/redeem rows).

## Verified behavior (root cause)

A **wire-data gap**, not only a render gap:

- `describe()` emits generic strings (`clients/squire-android/.../knight/app/ui/HistoryScreen.kt:127-132`):
  `"Approved" -> "$name earned ${amt} coins"`, `"Claimed" -> "$name claimed a quest"`,
  `"Redeemed" -> "…redeemed a reward"` — no quest/item reference.
- The DTO carries only ids and no titles (`crates/api/src/history.rs`, `HistoryEntryDto` ~`:31-48`).
  **Worse:** the `Approved` arm of `from_event` (`history.rs:71-75`) doesn't even set `quest_id` (the
  domain `CompletionApproved` is keyed by `claim_id`), so the earn event has *no* link to its quest
  on the wire. `Claimed` carries `quest_id` (`:67-70`) but there's no title and the client never maps
  it.
- The client already holds lookup tables it never uses here — `review.quests` (QuestOption
  {questId,title}) and `review.items` (ItemOption {itemId,name}) — but only `squireNames` is threaded
  into `HistoryScreen`.

## Acceptance Criteria

## Acceptance Criteria

- [ ] Approve/earn entries name the quest; redeem entries name the reward; achievement entries name
      the achievement.
- [ ] The `Approved` entry is linked back to its quest (currently it carries no quest reference at
      all).
- [ ] Existing `reason` display (rejections/adjustments) is preserved.

## Implementation Notes

### Technical Approach (recommended: resolve server-side)
Mirror `pending_claims`/`pending_requests`, which already resolve id→title from the snapshot
(`crates/api/src/knight.rs:562-617`):
- `crates/api/src/history.rs`: add `quest_title: Option<String>`, `item_title: Option<String>` (and
  achievement name) to `HistoryEntryDto`; make `from_event` snapshot-aware and resolve from
  `snap.quests`/`snap.items`. **Critically, backfill the quest on the `Approved` arm** by looking up
  the originating `CompletionClaimed` via `claim_id` (or thread the quest through). The `history`
  handler already locks a snapshot (`history.rs:139-147`).
- Android `HistoryScreen.kt` `describe()`: use the titles — e.g. `"$name earned $amt coins for
  \"$questTitle\""`, `"$name redeemed \"$itemTitle\""`.
- Regen `crates/api/openapi.json` + Kotlin SDK; new fields `Option`/nullable for back-compat.

### Dependencies
- Shares the `history.rs` + `HistoryScreen.kt` fix surface with [[SQUIRE-T-0121]] (actor shown as an
  id). **Do both together** — one DTO enrichment (actor_name + quest_title/item_title), one
  `from_event` snapshot-aware rewrite, one `describe()` pass, one SDK regen.

### Test Cases
- Rust: assert an `Approved` entry now carries `quest_title`; a redemption entry carries
  `item_title`.
- SDK decode test: payload with and without the new fields (back-compat).
- Android: a history snapshot with an `Approved` entry asserting the quest title appears.

## Status Updates

*To be added during implementation*
</content>