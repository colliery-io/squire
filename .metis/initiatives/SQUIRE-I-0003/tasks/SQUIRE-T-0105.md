---
id: tenant-isolation-hardening-cross
level: task
title: "Tenant-isolation hardening + cross-tenant leakage tests (shared-process security boundary)"
short_code: "SQUIRE-T-0105"
created_at: 2026-06-20T18:45:30.218739+00:00
updated_at: 2026-06-20T18:45:30.218739+00:00
parent: SQUIRE-I-0003
blocked_by: ["SQUIRE-T-0024"]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0003
---

# Tenant-isolation hardening + cross-tenant leakage tests

The **security-critical** task. On LAN, isolation was incidental (one process per household). In a shared
multi-tenant process it's a real attack boundary — prove no tenant can read/write another's data.

## Scope
- **Adversarial test suite**: a valid token for tenant A must NEVER reach tenant B's data via any path —
  forged/swapped `X-Household`, token-for-A + handle-for-B, id guessing across tenants, the provision race,
  the per-tenant store cache returning the wrong store.
- **Auth-before-tenant ordering** verified: handle is derived from the *verified principal*, never trusted
  from a raw header.
- **Store-cache safety** ([[SQUIRE-T-0024]]): the per-tenant `SharedStore` keyed correctly; eviction can't
  hand tenant A a store still bound to B.
- **File-path isolation** ([[SQUIRE-A-0014]]): per-tenant SQLite paths derive only from the verified handle;
  no traversal/escape.
- Run on both backends (SQLite file-per-tenant, PG schema-per-tenant) to keep the scale-path honest.
- Consider a fuzz/property test over interleaved multi-tenant request sequences.

## Acceptance
- [ ] A red-team test set covering the vectors above is green and lives in CI (regressions caught forever).
- [ ] Cross-tenant access is structurally impossible at the routing layer, not just "not currently exposed."
- [ ] Passes on SQLite + Postgres.

## Notes
Blocked by [[SQUIRE-T-0024]] (the concurrency/routing this hardens). The single highest-stakes item in
[[SQUIRE-I-0003]] — gate the MVP launch on it.
