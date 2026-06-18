---
id: playwright-e2e-for-the-keep-self
level: task
title: "Playwright E2E for the Keep (self-driving, screenshots)"
short_code: "SQUIRE-T-0069"
created_at: 2026-06-18T14:56:51.657938+00:00
updated_at: 2026-06-18T14:57:33.234189+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Playwright E2E for the Keep (self-driving, screenshots)

## Parent Initiative

[[SQUIRE-I-0001]] · User: replace the heavy manual server back-and-forth (curl + adb taps + headless-Chrome shims) with **self-driving browser tests** that exercise the Keep and produce visual artifacts.

## Objective

Stand up a Playwright project that **boots a throwaway demo server itself**, drives the Keep web UI in a real browser (login, tabs, quest authoring, library import, timezone settings), asserts outcomes, and captures screenshots/video/trace — repeatable in seconds, no manual setup.

## Acceptance Criteria

## Acceptance Criteria

- [x] `e2e/` Playwright project: `webServer` builds + launches the `squire-home` demo (loopback Keep `19920` / api `19080`, `SQUIRE_TZ` seeded), waits for `/health`, tears down after. Uses the **system Chrome** (`channel: "chrome"`) — no browser download.
- [x] Tests (`tests/keep.spec.ts`): login → Quests tab; **create** a weekly quest for a specific squire (asserts title + "Mon/Wed/Fri" + "Gawain" labels); **import** from the starter library; **Settings** read/change timezone (persists across reload) + reject an invalid zone. Explicit step screenshots to `screens/`.
- [x] `npm test` green (5/5) booting the server itself; `.gitignore` for `node_modules`/artifacts; `README.md` documents run + the Android caveat (Playwright can't drive native Compose).

## Implementation Notes

### Technical Approach
`@playwright/test` only (system Chrome avoids the chromium download). `webServer` command `cd .. && cargo build -p squire-home && … target/debug/squire-home` (debug reads assets from disk, so the live Keep JS/HTML/CSS/library are served). Demo seeds Knight 1/`demo` + Squire 2 "Gawain". Selectors target the stable ids the Keep already uses (`#quest-form`, `#quest-cadence`, `#library-list`, `#settings-tz`, …).

### Dependencies
The Keep tabbed shell + authoring + library + Settings ([[SQUIRE-T-0061]]/[[SQUIRE-T-0062]]/[[SQUIRE-T-0063]]/[[SQUIRE-T-0068]]).

### Risk Considerations
Tests share one server per run (state accumulates across tests in a run, but the demo wipes on each start). Native Android is out of scope — needs a Compose UI/screenshot harness later. Keep selectors in sync if the Keep markup ids change.

## Status Updates

**2026-06-18 — Done.** Scaffolded `e2e/` (Playwright + system Chrome, self-booting demo via `webServer`). 5 tests green in ~11s: login/tabs, create-weekly-for-squire (labels asserted), library import, timezone change (+persist) + invalid-zone rejection. Screenshots saved to `e2e/screens/` (e.g. the created "Walk the dog · Mon/Wed/Fri · Gawain" row). This replaces the manual curl/adb/headless loop for the **web** Keep; the native app still needs a separate Compose test harness (documented in the README).