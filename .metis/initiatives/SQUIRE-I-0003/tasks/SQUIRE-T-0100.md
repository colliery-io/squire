---
id: provision-the-cloud-box-ec2-t4g
level: task
title: "Provision the cloud box: EC2 t4g.micro + encrypted EBS, network, secrets, billing alarms, deploy path"
short_code: "SQUIRE-T-0100"
created_at: 2026-06-20T18:45:03.724063+00:00
updated_at: 2026-06-20T18:45:03.724063+00:00
parent: SQUIRE-I-0003
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0003
---

# Provision the cloud box

Stand up the single free-tier host per [[SQUIRE-A-0015]] (EC2+Caddy) + [[SQUIRE-A-0014]] (SQLite on EBS).
Foundation for everything else in [[SQUIRE-I-0003]].

## Scope
- **EC2 t4g.micro** (Graviton, free 12mo), Amazon Linux 2023; instance **IAM role** (SSM, S3, CloudWatch).
- **Encrypted EBS** data volume mounted for the per-tenant SQLite files (separate from root).
- **Network**: security group **443 in only**; **no open 22** — shell via **SSM Session Manager**.
- **Route 53** hosted zone + a domain/subdomain → the instance (Elastic IP so it's stable).
- **Secrets**: SSM Parameter Store — HMAC token-signing key, any service creds.
- **S3** bucket (private, versioned) for backups + a dist mirror.
- **Billing + free-tier usage alarms** (hard requirement) + a basic CloudWatch log group + CPU/disk alarms.
- **Deploy path**: how `squire-serve` lands + updates — reuse the self-update-from-dist mechanism
  ([[SQUIRE-A-0012]]) under the launchd-equivalent (systemd unit on Linux) or SSM run-command.

## Approach
- Prefer **IaC (Terraform)** committed to the repo over click-ops, so the box is reproducible/teardownable.
- systemd service unit for `squire-serve` (RunAtLoad/Restart=always — the Linux analog of the macOS
  LaunchAgent we built), self-update + (no apk-sync needed in cloud) on.

## Acceptance
- [ ] `terraform apply` (or documented click-ops) yields a reachable host with 443-only ingress, SSM
  shell, encrypted EBS mounted at the data path, Elastic IP + DNS.
- [ ] Billing + free-tier alarms active and tested (alert fires).
- [ ] `squire-serve` runs under systemd (restart-on-crash, start-on-boot) and self-updates from dist.
- [ ] Secrets resolved from SSM at runtime (no secrets in the image/AMI).
- [ ] Teardown documented (no orphaned paid resources).

## Notes
Blocks [[SQUIRE-T-0101]] (Caddy) and [[SQUIRE-T-0106]] (backups need the S3 bucket + EBS).
