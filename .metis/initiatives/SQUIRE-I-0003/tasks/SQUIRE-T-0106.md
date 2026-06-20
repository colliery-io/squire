---
id: backups-restore-drill-scheduled-s3
level: task
title: "Backups + restore drill: scheduled S3 per-tenant export + EBS snapshots, rehearsed restore"
short_code: "SQUIRE-T-0106"
created_at: 2026-06-20T18:45:35.480074+00:00
updated_at: 2026-06-20T18:45:35.480074+00:00
parent: SQUIRE-I-0003
blocked_by: ["SQUIRE-T-0100", "SQUIRE-T-0102"]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0003
---

# Backups + restore drill

[[SQUIRE-A-0014]] makes durability **our** responsibility (SQLite on single-AZ EBS). This task makes the
backup story real *and rehearsed* — the ADR calls the restore drill mandatory, not optional.

## Scope
- **Scheduled per-tenant export to S3** (reuse [[SQUIRE-T-0012]] single-file export): each tenant's SQLite
  → versioned S3 object on a schedule; verify integrity post-upload.
- **EBS volume snapshots** on a schedule (whole-box point-in-time); lifecycle/retention policy.
- **Encryption**: S3 SSE + encrypted EBS snapshots.
- **Restore drill**: a documented, *executed* procedure — pick a tenant, restore its file from S3 into a
  scratch instance, boot squire-serve, verify the household + balances match. Time it.
- Alarm on backup failure / staleness (no successful export in N hours).

## Acceptance
- [ ] Per-tenant exports land in S3 on schedule, versioned + integrity-checked.
- [ ] EBS snapshots scheduled with retention.
- [ ] A **restore drill has been run** end-to-end and documented (RTO noted), not just scripted.
- [ ] Backup-failure/staleness alarm fires on a forced failure.

## Notes
Blocked by [[SQUIRE-T-0100]] (S3 + EBS) and [[SQUIRE-T-0102]] (per-tenant data layout). Reuses the existing
export/import — low new code, high ops value.
