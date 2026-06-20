---
id: backups-restore-drill-scheduled-s3
level: task
title: "Backups + restore drill: per-tenant export → Cloudflare R2 (offsite), rehearsed restore"
short_code: "SQUIRE-T-0106"
created_at: 2026-06-20T18:45:35.480074+00:00
updated_at: 2026-06-20T18:45:35.480074+00:00
parent: SQUIRE-I-0003
blocked_by: ["SQUIRE-T-0102"]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0003
---

# Backups + restore drill (offsite to R2)

[[SQUIRE-A-0014]] makes durability ours; [[SQUIRE-A-0016]] puts the box **at home**, so backups **must be
offsite** — a home disk/power loss can't be the only copy. *(Target shifted S3 → Cloudflare R2; the
export logic is identical.)*

## Scope
- **Scheduled per-tenant export to Cloudflare R2** (reuse [[SQUIRE-T-0012]] single-file export): each
  tenant's SQLite → versioned R2 object on a schedule; integrity-check post-upload.
- **A second offsite copy** is cheap insurance (R2 lifecycle/versioning, or a periodic copy elsewhere) —
  there's no EBS-snapshot equivalent now that the box is a home Mac (a local Time Machine copy is a nice
  on-site adjunct but is NOT the offsite backup).
- **Encryption**: R2 SSE; consider client-side encrypt before upload (home box → third-party storage).
- **Restore drill**: documented + *executed* — pull a tenant's file from R2 onto a scratch machine, boot
  squire-serve, verify household + balances match. Time it (record RTO).
- Alarm on backup failure / staleness (no successful export in N hours).

## Acceptance
- [ ] Per-tenant exports land in R2 on schedule, versioned + integrity-checked + encrypted.
- [ ] A **restore drill has been run** end-to-end and documented (RTO noted), not just scripted.
- [ ] Backup-failure/staleness alarm fires on a forced failure.

## Notes
Blocked by [[SQUIRE-T-0102]] (per-tenant data layout); R2 bucket comes from [[SQUIRE-T-0100]]. Reuses the
existing export/import — low new code, high ops value. Especially important under [[SQUIRE-A-0016]]
(single home box = single point of loss without offsite backup).
