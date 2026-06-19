---
id: cosmetic-pass-on-the-keep-members
level: task
title: "Cosmetic pass on the Keep Members / Pair / Log tabs (role badges, framed pairing, polish)"
short_code: "SQUIRE-T-0082"
created_at: 2026-06-19T00:15:51.798334+00:00
updated_at: 2026-06-19T00:19:58.896913+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Cosmetic pass on the Keep Members / Pair / Log tabs (role badges, framed pairing, polish)

## Parent Initiative

[[SQUIRE-I-0001]]

## Objective

The last unrefined surface: the Keep's **Members / Pair / Log** tabs. They work but read plainly — member rows are bare `name — role` text, the minted pairing token is unstyled, and Pair/Log are utilitarian. Bring them up to the parchment-card standard with a small reusable badge/row vocabulary. CSS + vanilla-JS rendering only — no endpoint/contract change.

## Define-First: the visual outcomes we want

### Flow A1 — Members read as a roster
- Each member row: name in `<strong>`, a **role badge** (Knight = royal, Squire = gold), an **Inactive** badge when deactivated (and the row dimmed), with the De/Reactivate action right-aligned on the same line (not dropped below).
- The freshly-minted token after "Add member" shows as a **success notice** with the token in a `code` chip (copy-friendly), not a plain sentence.

### Flow A2 — Pair reads as a hand-off card
- The pairing result is a centered, framed card: the QR, the code prominently, and the reach/expiry as a caption — so it's obviously "show this to the device".

### Flow A3 — Log reads as an inspector
- A friendly placeholder before a query is run ("Pick a scope + id to trace how a balance/streak was reached."), and the result framed as today.

## Current State

- `keep.css` has cards/buttons/tabs but no badge/pill or right-aligned list-row helper; list buttons inherit `margin-top` and drop below the text.
- `keep.js` `loadMembers` renders `li.textContent = name — role (inactive)` + a button; the member token is a `.sub` sentence; the pair result is a loose stack.

## Acceptance Criteria

- [x] `keep.css`: added `.badge` (+ `.badge-knight` / `.badge-squire` / `.badge-muted`), `.list-row` (content left / actions right), `.btn-sm` + `.btn-ghost`, `.notice` / `.notice-success`, and `.pair-card`. Reused the palette vars.
- [x] `keep.js` `loadMembers`: name `<strong>` + role badge + Inactive badge (row dimmed via `.list-row.inactive`), action right-aligned on one line.
- [x] Member-add: minted token renders as a `.notice-success` with the token in a `code.tok`.
- [x] Pair tab: `#pair-result` framed as a centered `.pair-card` (QR + `.pair-code` chip + reach/expiry caption).
- [x] Log tab: `#log-hint` placeholder shown until a query runs (then the `<pre>` reveals).
- [x] Playwright `e2e/tests/keep-admin.spec.ts` (4): Members roster+badges, add-member token notice, Pair card QR, Log hint→result — each screenshotted. Full suite 23/23 green.

## Implementation Notes

- Vanilla JS / CSS only (ADR A-0008). Build the member row with elements (badge `<span>`s) instead of `textContent`. Keep all endpoints + wire shapes unchanged.
- The demo seeds admin Knight "Arthur" + Squire "Gawain", so the Members + Pair tabs have real rows to screenshot.

## Dependencies
- Completes the Keep-tab refinement started in T-0061/0062/0071/0073/0080.

## Status Updates

**2026-06-19 — Done.**
- `keep.css`: added a small reusable vocabulary — `.badge` (+role/muted variants), `.list-row` (+`.inactive` dim), `.btn-sm`/`.btn-ghost`, `.notice`/`.notice-success`, `.pair-card`.
- `keep.js` `loadMembers`: rebuilt rows from elements (name + role badge + optional Inactive badge, ghost action right-aligned). Member-add token → `.notice-success` with a `code.tok`. Log handler reveals `<pre>` + hides the hint.
- `index.html`: `#pair-result` → `.pair-card` (+ `.pair-code` chip); Log gains a `#log-hint` placeholder, `#log-output` hidden until a query.
- `e2e/tests/keep-admin.spec.ts` (4 tests) — all pass; image-validated Members roster (role badges + green token notice), the Pair hand-off card (QR + code + reach/expiry caption), and the Log hint→result. Full Playwright suite 23/23.
- CSS + vanilla-JS only — no endpoint/contract change. Completes the Keep-tab refinement (Quests/Review/Rewards/Achievements already done).