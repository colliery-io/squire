---
id: android-knight-gherkin-via
level: task
title: "Android Knight Gherkin via cucumber-jvm"
short_code: "SQUIRE-T-0115"
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

# Android Knight Gherkin (cucumber-jvm)

**DONE** (commit `8e7a299`). cucumber-jvm + a JUnit Platform suite in `:knight-core` (pure JVM, no
emulator), driving the offline `KnightStore`. `knight_actions.feature` covers the new cash payout
([[SQUIRE-T-0111]]) — pay → Cash adjust of −amount — plus the coin grant + positive-amount/non-blank-reason
guards. 3 scenarios green. `angreal test gherkin` runs api+android; `test all` adds `_android_unit`.
Instrumented/Compose interaction deferred (no emulator here; store-level behavior covered on the JVM).
