---
id: keep-refine-e2e-test-the-rewards
level: task
title: "Keep: refine + E2E-test the Rewards creation workflow (match the quest creator)"
short_code: "SQUIRE-T-0073"
created_at: 2026-06-18T18:15:34.909409+00:00
updated_at: 2026-06-18T18:21:05.984613+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Keep: refine + E2E-test the Rewards creation workflow (match the quest creator)

## Parent Initiative

[[SQUIRE-I-0001]]

## Objective

Bring the Keep's **Rewards** tab up to the quest-creator standard (parallel to T-0063 quests / T-0071 achievements): a curated starter library with one-tap import, a cross-object gate dropdown that stays fresh, and Playwright E2E coverage with screenshots. The domain (`RedeemableItem`, `DefineItem`/`ArchiveItem`, `validate_item`) and the Keep cookie-API (`/api/items` GET/POST/archive) already exist — this task refines the **authoring UX + library + tests**, it does not change the model.

## Define-First: Expected Flows & Outcomes

These are the flows we expect to hold; the test suite must exercise each.

### Flow R1 — Create a reward (Knight, Keep)
- **Inputs**: name (required), cost in points (required, ≥ 1), availability (`Once` | `Repeatable`), optional gate (requires an achievement to be unlocked), optional description.
- **Outcome**: `DefineItem` persists; the reward appears in the catalog list; it becomes immediately visible in each child's reward shop (`StateView.rewards`).
- **Validation**: blank name or cost < 1 → blocked at the form. A gate referencing a non-existent achievement → 400 from the engine (`validate_item` → `AchievementNotFound`).

### Flow R2 — Import from the starter library
- A curated `rewards-library.json` of common household rewards (screen time, movie pick, dessert choice, stay-up-late, day-trip pick, small allowance, …) each with a sensible cost + availability.
- One-tap **Import** → `DefineItem`; the imported reward joins the catalog and the child reward shop.

### Flow R3 — List / archive
- The catalog shows every reward (active + archived), with `Once` items flagged out-of-stock once redeemed, and last-redeemed where known.
- **Archive** hides it from the child shop (never deletes; audit preserved).

### Flow R4 — Gate-by-achievement stays fresh (cross-object)
- A reward may require an achievement to be unlocked. The gate `<select>` must repopulate whenever an achievement is added/archived in the Achievements tab — mirroring the T-0071 quest→achievement-scope dropdown refresh — so a just-created achievement is selectable without a page reload.

## Acceptance Criteria

- [x] `crates/keep/assets/rewards-library.json` exists (10 common rewards: name, cost, availability, description), shaped like `library.json` / `achievements-library.json`.
- [x] Rewards tab gains a "🎁 Add from the starter library" section with per-row Import (mirrors quest/achievement library UI).
- [x] The item form validates name non-blank and cost ≥ 1 (JS inline error, matching the form's pattern); surfaces the engine's gate error (404 → "pick another"). Added an optional description field.
- [x] The gate dropdown refreshes after an achievement is created/archived (no reload) — already wired via `loadCatalog("achievements")` → `populateItemGate()`; now covered by a test.
- [x] `crates/keep/tests/catalog.rs` covers create + list + archive of items, gate validation (gate→unknown achievement = 404), and a new `rewards_library_entries_all_import` round-trip (11 tests pass).
- [x] Playwright `e2e/tests/keep-rewards.spec.ts`: create / cost<1 reject / library import / archive / "a newly added achievement appears in the reward gate dropdown" — 5 tests pass, screenshots under `screens/`.
- [x] `cargo test -p keep` (11) and the Playwright suite (5) pass; refreshed Rewards tab screenshot captured.

## Implementation Notes

### Technical Approach
- Keep cookie-API already complete (`crates/keep/src/items.rs`, routes in `lib.rs`) — no Rust handler changes expected beyond tests.
- UI work is in `crates/keep/assets/{index.html,keep.js}`: add the library section + `loadRewardsLibrary()` (mirror `loadLibrary`/`loadAchLibrary`), wire `loadItemGate()` refresh into the achievement create/archive paths, add the cost/name guard.
- Tests: extend `crates/keep/tests/catalog.rs`; add `e2e/tests/keep-rewards.spec.ts`.

### Dependencies
- Mirrors T-0063 (quest library) and T-0071 (achievement library + dropdown refresh + E2E).
- Sibling of T-0074 (Rewards authoring on the phone), which adds the LAN api + native screen.

## Status Updates

**2026-06-18 — Done.**
- Added `crates/keep/assets/rewards-library.json` (7 Repeatable privileges + 3 Once treats).
- `index.html`: optional description field; cost `min=0` so the JS guard owns the ≥1 rule with a friendly inline message; `🎁 Add from the starter library` `<details>` section.
- `keep.js`: `loadRewardsLibrary()` + `rewardLibToWire()`/`rewardLibSummary()`, wired into `TAB_LOADERS.rewards`; name/cost guards + gate-404 message + description in the POST body.
- **Define-first verification**: Flow R4 (gate dropdown stays fresh) was already satisfied by `loadCatalog("achievements") → populateItemGate()`; confirmed with a Playwright test rather than new code.
- Tests: `cargo test -p keep --test catalog` → 11 pass (added `rewards_library_entries_all_import`); `npx playwright test keep-rewards` → 5 pass with screenshots.
- Image-validated the refreshed Rewards tab (form + library + catalog incl. a gated reward).
- No Rust handler changes — the Keep cookie-API for items was already complete.