---
id: refine-the-keep-review-tab-inline
level: task
title: "Refine the Keep Review tab: inline reason/adjust entry (no browser prompts) + E2E"
short_code: "SQUIRE-T-0080"
created_at: 2026-06-18T23:04:19.233250+00:00
updated_at: 2026-06-18T23:10:23.001462+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Refine the Keep Review tab: inline reason/adjust entry (no browser prompts) + E2E

## Parent Initiative

[[SQUIRE-I-0001]]

## Objective

Bring the Keep's **Review** tab up to the quest-creator standard. Today it uses blocking browser `prompt()` calls for reject reasons and adjust amount/reason (`keep.js` `loadReview`) — jarring, unstyled, unscreenshot­able, and untested. Replace them with **inline form entry**, matching the reject-reason dialog we just shipped on the phone (T-0078), and add Playwright coverage. (Members/Pair/Log stay as-is for now — Review is where the real UX debt + the core loop live.)

## Define-First: the flows we expect

### Flow V1 — Reject a claim/request with an inline reason
- Tapping **Reject** reveals an inline reason field + **Confirm** / **Cancel** in that row (no `prompt()`). Confirm posts the reject with the typed reason (blank allowed); the child sees it in their "Recent" (already wired).

### Flow V2 — Adjust a balance inline
- Tapping **Adjust** reveals an inline amount (±) field + a **required** reason field + **Apply**. A blank reason disables Apply (the engine 400s an empty reason; the form prevents it).

### Flow V3 — No regression
- Approve (claims + requests) and the cross-Squire queue list are unchanged; the queue refreshes after each action.

## Current State

- `keep.js` `loadReview()` builds rows with `prompt("Reason?")` (reject claim + reject redemption) and `prompt("Adjust by (+/-)")` + `prompt("Reason (required)")` (adjust). The endpoints (`/api/review/claim`, `/api/review/redemption`, `/api/adjust`) and their wire shapes are unchanged.
- No Playwright spec exercises the Review tab.

## Acceptance Criteria

- [x] `keep.js` Review tab: **no `prompt()`** — Reject reveals an inline reason field (`attachRejectEditor`, Confirm/Cancel); Adjust reveals an inline amount + required-reason field (Apply disabled until reason non-blank). + `keep.css` `.inline-editor` styling.
- [x] Stable classes for E2E selectors (`button.reject`, `input.reject-reason`, `button.confirm-reject`, `button.adjust`, `input.adjust-amount`, `input.adjust-reason`, `button.apply-adjust`). Editors built in JS (no new HTML).
- [x] Playwright `e2e/tests/keep-review.spec.ts` (3): reject-with-inline-reason, adjust-with-required-reason (+ the disabled-until-reason guard), approve-no-regression — each screenshotted.
- [x] Seeded **two pending claims** in the `squire-home` demo (Gawain · "Tidy your room" + "Walk the dog") so the Review tab demonstrates triage and the E2E can reject one / approve another independently.
- [x] `cargo test -p keep` (11) + full Playwright suite (19) pass; refreshed Review tab image-validated.

## Implementation Notes

- Vanilla JS (ADR A-0008) — no framework. Build the inline editors in `loadReview` (toggle a hidden `<form>`/`<div>` per row, or replace the action buttons with the editor on click). Reuse the existing `reviewAction(path, body)` helper.
- Set up a small amount of pending state via the engine-direct seam in the Playwright `webServer` demo, or submit a claim through the API as part of the test.

## Dependencies
- Parallels T-0078 (phone reject-reason). Same reason→child path (T-0076/0077).

## Status Updates

**2026-06-18 — Done.**
- `keep.js`: replaced all `prompt()` in `loadReview` with inline editors — `attachRejectEditor(li, btn, onConfirm)` (reason field + Confirm/Cancel) for claim + request rejects; an inline adjust editor (± amount + required reason, Apply disabled until reason non-blank). Reuses `reviewAction`. `keep.css`: `.inline-editor` + crimson confirm/apply buttons.
- `squire-home` demo: seeded two pending claims for Gawain (quests 101 "Tidy your room" + new 102 "Walk the dog") via `Change::Append(Event::CompletionClaimed)` so the Review tab always has triage and the E2E tests can act on distinct rows.
- `e2e/tests/keep-review.spec.ts` (3 tests) — all pass; reject screenshot shows the inline reason editor (no browser prompt). Full Playwright suite 19/19; `cargo test -p keep` 11/11.
- Image-validated the refreshed Review tab. Members/Pair/Log left as-is (functional; Review held the real UX debt + the core loop).