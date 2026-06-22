---
id: full-stack-durability-gherkin
level: task
title: "Full-stack durability Gherkin (phone to API to store)"
short_code: "SQUIRE-T-0116"
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

# Full-stack durability Gherkin (phone → API → store)

**DONE** (commit `9e85034`). Extends the api cucumber harness with a "the server restarts" step that
reopens a fresh SQLite connection over the same tenant file (reusing the identity so tokens stay valid).
`durability.feature`: a granted balance and an approved claim's credit both survive a restart — a fresh
connection only sees on-disk events, so a correct post-restart `/state` proves the log is durable, not
in-memory. api cucumber: 4 features / 10 scenarios / 50 steps green.
