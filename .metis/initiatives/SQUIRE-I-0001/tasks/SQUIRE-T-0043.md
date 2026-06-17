---
id: surface-knight-redeem-mark-done
level: task
title: "Surface Knight redeem + mark-done quick-actions in the UI"
short_code: "SQUIRE-T-0043"
created_at: 2026-06-17T17:25:00+00:00
updated_at: 2026-06-17T19:47:07.599174+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Surface Knight redeem + mark-done quick-actions in the UI

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0006]] (REQ-K3 mark-done, REQ-K5 direct redeem) · follow-up of [[SQUIRE-T-0040]]

## Objective

`:knight-core`'s `KnightStore` already implements `redeem(squire, itemId)` (REQ-K5 direct redeem) and `markDone(squire, questId, on)` (REQ-K3), and `KnightViewModel` exposes them — but the Knight UI (`KnightHomeScreen`) currently only wires Approve/Reject + Add-funds. Surface the two missing quick-actions so the parent can spend on a child's behalf and credit a chore the child did but didn't claim, completing the REQ-K3/K5 surface in the app.

## Open question (small)

The Knight's `HouseholdReview` read carries per-Squire balances and the pending queues, but **not** the catalog of redeemable items or the child's quests-of-the-day — which `redeem`/`markDone` need (an `item_id` / `quest_id` to target). Options: (a) add a lightweight per-Squire "items + today's quests" read to the parent surface (small API addition), or (b) reuse the existing Squire `GET /state` for a selected Squire if the Knight token is allowed to read it. Resolve cheaply — (a) is cleaner and keeps the Knight on its own surface.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] The Knight can **mark a quest done** for a chosen Squire from the UI (Squire row → "Mark done" → pick a quest), routing through `KnightViewModel.markDone(squire, questId, today)`; credit appeared after sync (live: Make your bed → +5).
- [x] The Knight can **directly redeem** an item for a chosen Squire from the UI (Squire row → "Redeem" → pick an item), routing through `KnightViewModel.redeem`; balance debited after sync (live: Ice cream → −3).
- [x] Both actions go through the durable privileged outbox (offline-safe, idempotent on the minted `command_id`/`claim_id`) — same path as the other quick-actions; the engine re-checks affordability/availability at commit.
- [x] Resolved the open question with option (a): additively extended `HouseholdReview` with `items` (active catalog), `quests` (active), and `today` (date number for mark-done) — keeps the Knight on its own surface. `cargo test --workspace` green (openapi re-frozen, SDK regenerated); `:knight-app:assembleDebug` builds; live emulator demo captured (`/tmp/knight-quickactions.png`).

## Implementation Notes

### Technical Approach
Add the picker UI (a dialog per Squire row, or a small per-Squire detail screen) listing that Squire's redeemable items and today's quests, then call the already-wired `viewModel.redeem` / `viewModel.markDone`. The plumbing (`KnightStore`, outbox, sync, adapter mapping) is done — this is mostly UI + the read that backs the pickers.

### Dependencies
[[SQUIRE-T-0040]] (Knight app + `redeem`/`markDone` already in `:knight-core`/`KnightViewModel`). May need a small parent-surface read (decide per the open question).

### Risk Considerations
Smallest of the deferred items if we reuse an existing read; slightly larger if a new parent-surface endpoint is added (then re-freeze `openapi.json` + regen SDK, per the T-0031/T-0038 pipeline). Keep the Knight on its own surface rather than overloading the Squire `GET /state`.

## Status Updates

**2026-06-17 — Done.** Chose open-question option (a): additively extended `HouseholdReview` (`crates/domain-core/src/contract/api.rs`) with `items: Vec<ItemOption{item_id,name,cost}>`, `quests: Vec<QuestOption{quest_id,title}>`, and `today: Date`; the `knight.rs` (api) and `review.rs` (keep) assemblers populate them from the active catalog + `clock.today()`. Registered the two new schemas in `openapi.rs`, re-froze `openapi.json`, regenerated the SDK (new `ItemOption`/`QuestOption`; `HouseholdReview` gains `items`/`quests`/`today`). `cargo test --workspace` green.

Knight UI: `SquireRow` now shows three actions (Mark done · Redeem · Add funds); "Mark done" and "Redeem" open a `PickDialog` listing `review.quests` / `review.items` — tapping an option fires `viewModel.markDone(squire, questId, review.today)` / `viewModel.redeem(squire, itemId)` through the existing durable outbox. `:knight-app:assembleDebug` builds.

**Live demo:** Gawain 0 pts → Mark done "Make your bed" → **+5** (balance 5) → Redeem "Ice cream" → **−3** (balance 2). Screenshot `/tmp/knight-quickactions.png`. The REQ-K3 (mark-done) and REQ-K5 (direct redeem) surfaces are now complete in the app.