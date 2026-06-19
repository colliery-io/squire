---
id: fix-add-member-id-counter-restart
level: task
title: "Fix add_member id-counter restart collision (seed from store high-water mark) + add_admin escape hatch"
short_code: "SQUIRE-T-0086"
created_at: 2026-06-19T16:32:51.253308+00:00
updated_at: 2026-06-19T16:32:51.253308+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: true
initiative_id: SQUIRE-I-0001
---

# Fix add_member id-counter restart collision + add_admin escape hatch

## Parent Initiative

[[SQUIRE-I-0001]]

## Objective

Fix a data-corruption bug: `ProdIdentity::shared_local` seeded its in-memory id counter
(`Mutex::new(0)`) at construction, and `next()` increments it for new member ids. The counter is
NOT persisted and NOT seeded from the store, so on every process restart it resets to 0 — the first
post-restart `add_member` allocates `UserId(1)` and `apply(PutUser{ id: 1, .. })` **overwrites the
existing member at id 1** (plus their credential). This silently clobbers a real household member.

## Bug

- `crates/identity/src/prod.rs`: `shared_local` (and `local`/`hosted`) did `counter: Mutex::new(0)`.
- `next()` returns `*counter += 1`, so the first id after any restart = 1.
- Impact: the live household's `UserId(1)` (an active Squire) was overwritten when a member was added
  after a restart. Matches the user's report that admin "Dad" vanished "as a result of the restart" —
  same root cause: an earlier add-after-restart had already overwritten the Knight at id 1.

## Fix

- Seed the counter from the store's id high-water mark at construction: new `max_user_id(&SharedStore)`
  reads `snapshot().users` and takes `max(id)` (0 if empty). `shared_local` uses it. Empty store → 0,
  so first-run `register` still bootstraps the admin as `UserId(1)`. `local`/`hosted` keep 0 (registry /
  multi-tenant test+hosting shapes where this production restart path doesn't apply) — noted in code.
- A stronger fix (persist the counter, or derive id live as `max+1` per call) is possible later; this
  seeding fix removes the corruption for the production local-api path.

## Escape hatch (kept)

`crates/squire-home/src/bin/add_admin.rs`: one-off operator tool to insert a Knight admin into an
existing household via the identity component (proper argon2), authorised by an existing Knight.
Usage: `ADMIN_NAME=Dad ADMIN_SECRET=pass ACTING_KNIGHT=2 cargo run -p squire-home --bin add_admin`.
Run with the server stopped (single SQLite writer). Kept per operator request for recovery.

## Incident + recovery (operational, not in-repo)

While diagnosing, the pre-fix tool itself hit the bug and overwrote the live `UserId(1)`
("Matrim Oakenfury", active Squire) with a Knight. Recovered:
- Backed up the live DB to `home.sqlite.prerepair-*`.
- Confirmed Matrim's domain events (7, keyed `squire='1'`) were untouched (user rows are projection
  state, not event-sourced) — so restoring her row reconnects all her quest history/balance.
- Restored her `users` row + original credential hash by direct SQL (server stopped, exact original
  values captured pre-incident).
- Re-added Knight "Dad" with the FIXED tool → correctly allocated `UserId(3)` (no collision).
- Final household: 1 Squire Matrim, 2 Knight Mom, 3 Knight Dad. Verified Dad login (`/login` 200,
  role Knight) on the restarted prod.

## Acceptance Criteria

- [x] `shared_local` seeds the id counter from the store high-water mark; empty store still bootstraps id 1.
- [x] `add_member` after a restart no longer reuses id 1 (verified: Dad → id 3).
- [x] `add_admin` escape-hatch tool builds clean and inserts a Knight via the identity layer.
- [x] Live data repaired (Matrim restored with history; Dad added) and verified via login.

## Dependencies

Identity component from [[SQUIRE-T-0022]]; surfaced during the OTA/redesign deploy work
[[SQUIRE-T-0085]].

## Status Updates

- Bug fixed, escape hatch added, live household repaired + verified, prod restarted. Done.
