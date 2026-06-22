---
id: keep-web-gherkin-playwright-bdd
level: task
title: "Keep web Gherkin via playwright-bdd"
short_code: "SQUIRE-T-0114"
created_at: 2026-06-22T02:11:00+00:00
updated_at: 2026-06-22T02:11:00+00:00
parent: SQUIRE-I-0004
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: true
initiative_id: SQUIRE-I-0004
---

# Keep web Gherkin (playwright-bdd)

**DONE** (commits `60b8dfc`, `f25a57a`). True Gherkin over Playwright via **playwright-bdd v9.2** (v8 was
incompatible with Playwright 1.61), reusing the existing demo `webServer`. A `bdd` project beside the
original `specs`; `npm test` runs `bddgen` first. Converted `keep-review.spec.ts` → `keep-review.feature`
(approve / reject-with-reason / adjust), same vocabulary as the api suite ([[SQUIRE-T-0110]]). Full e2e
green: 23 passed. **Bonus bug fixed:** `Quest.cash` had no serde default → the Keep library import (omits
cash) 400'd (the cash twin of [[SQUIRE-T-0109]]); `#[serde(default)]` added.

Follow-up (not blocking): convert the remaining specs (keep-admin/achievements/rewards/quests) to
`.feature` the same way.
