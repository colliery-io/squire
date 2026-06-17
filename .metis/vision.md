---
id: squire
level: vision
title: "squire"
short_code: "SQUIRE-V-0001"
created_at: 2026-06-16T22:14:29.880792+00:00
updated_at: 2026-06-16T22:55:31.823226+00:00
archived: false

tags:
  - "#vision"
  - "#phase/published"


exit_criteria_met: false
initiative_id: NULL
---

# Squire Vision

## Purpose **[REQUIRED]**

Squire turns a single child's household chores into a game they *want* to play, while keeping every point and reward firmly under parental control. It exists to make routine responsibility feel like progression — quests, streaks, achievements, and a reward store — without ever letting the child mint their own points or grant their own rewards.

The deeper aim is a system that is **trustworthy by construction**: the child can act freely (and offline), but only the parent's computer can ever move the balance. Correctness, auditability, and a frictionless parent review loop matter as much as the gameplay.

## Product/Solution Overview **[CONDITIONAL: Product/Solution Vision]**

Squire is a local, unpublished, single-household app built as **cooperating clients around a single-writer core, separated by an authenticated trust boundary**:

- **The Keep (computer, parent/admin)** — Rust engine + store + local admin UI + local API. The single source of truth and the *only* writer. The parent authors quests, items, and achievements; reviews claims and redemption requests; redeems directly; and makes manual adjustments.
- **The Squire (Android phone, child/player)** — native Kotlin + Jetpack Compose, with a local cache and an offline outbox. Renders quests, balance, streaks, and rewards; submits completion claims and redemption requests. It can read and propose, but never commit.
- **The Knight (Android phone, parent/quick-admin)** — a parent's pocket remote for the review loop: approve/reject a claim, mark a quest done for the child, approve/reject or directly redeem a reward, and add funds (a point adjustment). Same native stack and offline outbox as the Squire, but authenticated with a **parent credential** the child app never holds. Authoring stays keyboard-first on the Keep.

All phone apps are *clients*: the Keep remains the single writer. The Knight never writes the store directly — it submits privileged `Command`s that the Keep validates through the one engine door and commits, so the integrity model rests on per-user tenant-scoped tokens plus single-writer commit, not on hiding operations from the network. Identity is per-user: a household registers (creating its fully isolated tenant), then adds N Knights (adults) and N Squires (children), each with their own account and role (A-0002/A-0004). Persistence is **Diesel dual-backend** so the same code runs single-tenant on local SQLite or multi-tenant on hosted Postgres (A-0003). Target audience for the MVP: a single household running locally on a home network, with no cloud — but the foundation is multi-tenant-capable for a future hosted deployment.

## Current State **[REQUIRED]**

Greenfield. We have a finalized v1 PRD (`squire_prd.md`) and an authoritative shared type contract (`shared_contract.rs`) defining the domain model, events, commands, ports, and API DTOs. No engine, store, API, admin UI, or phone client exists yet. The contract is design-first (types and signatures only); no behavior is implemented.

## Future State **[REQUIRED]**

A working two-app system where:

- The parent maintains chores, point values, streak achievements, and a reward store quickly, with a keyboard.
- The child sees what to do, tracks points and streaks, browses rewards, and submits "I did X" / "I want Y" from their phone — even while the computer is off.
- Points are earned only on parent (or auto-approve) confirmation and spent only on parent-approved redemptions.
- All balances, streaks, due-lists, and unlocks are *derived* from an append-only event log — never stored as mutable counters — so history is always explainable and never silently rewritten.
- The two devices never need to be online simultaneously for the child to act.

## Major Features **[CONDITIONAL: Product Vision]**

- **Parent authoring (the Keep):** create/edit/archive quests (with cadence, reward, auto-approve, repeatability), redeemable items (cost, achievement gate, availability rules), and streak/total/points achievements. Archiving never deletes — history stays valid.
- **Quest scheduling & due-logic:** one-off and recurring cadences (Daily / Weekly{days} / EveryNDays{n,anchor}); a derived "what's due today" that subtracts already-satisfied quests.
- **Claim & review loop:** child submits completion claims; parent approves/rejects from a single batched queue; auto-approve quests skip review entirely. Point values are snapshotted at approval so editing a reward never rewrites the past.
- **Points ledger:** balance derived purely from the event log (approvals +, redemptions −, bonuses +, adjustments ±). Manual adjustments require a reason.
- **Streaks & achievements:** schedule-aware streaks for quest scopes, calendar-day streaks for category/any scopes; sticky unlocks that award bonus points and unlock gated store items.
- **Reward store & redemption:** child requests redemptions; parent approves/rejects or redeems directly; affordability and availability validated only at commit time. No points reserved while pending.
- **Player app (the Squire):** renders entirely from a single per-Squire `StateView` payload — today's quests with status, balance, streaks, affordable/locked rewards, and the status of recent claims/requests — scoped to the signed-in child (A-0005). The Knight sees a per-household review across all Squires.
- **Parent quick-admin app (the Knight):** a phone remote for the review loop — approve/reject claims, mark a quest done for the child, approve/reject or directly redeem rewards, and add funds. Authenticated with a parent credential; quick actions only (no authoring). Acts even when the Keep is asleep by queuing privileged commands in an idempotent outbox that flushes on reconnect.
- **Sync & offline:** each phone caches the last `StateView` and renders from cache when the computer is unreachable; claims, requests, and parent quick-actions queue in a local outbox and flush idempotently (client-minted ids) on reconnect.

## Success Criteria **[REQUIRED]**

- The child can complete a daily quest **offline**, have it appear in the parent's queue on next sync, and see their balance rise after approval — without either device being online at the same time.
- The **child** phone (the Squire) is **provably incapable** of minting points, approving submissions, or redeeming: the child role's network surface offers only read + propose. Privileged operations are reachable only under a separate parent credential (the Knight), and even then the Keep — never the phone — performs the write.
- Auto-approve plus a single batched review queue keep routine parent effort to seconds per day.
- Any balance or streak is **explainable** by replaying the raw event log for the relevant quest/item.
- Editing a quest's reward never changes the points of completions approved under the old value.
- All projections (balance, streaks, due list, state view) compute in well under 100 ms at single-child scale (dozens of quests, thousands of events over years).
- All eight PRD acceptance criteria (AC-1 … AC-8) pass end-to-end.

## Principles **[REQUIRED]**

- **Single writer.** The computer holds the canonical store and is the only process that writes it.
- **Events are truth; counters are derived.** Activity is an append-only log; balance, streaks, due-status, and unlocks are computed, never stored as mutable values.
- **Snapshot at commit.** Point values are captured onto the approval event so history is immutable under later edits.
- **One validated door.** Every mutation passes through a single `handle()` entry point where all rules live and are testable.
- **Pure, portable core.** The domain core (types, `handle`, projections) has no I/O, storage, transport, or UI dependencies.
- **Single-writer is the invariant; the boundary is authenticated, not absent.** The network exposes per-user, tenant-scoped, role-bearing tokens — Squires (read + propose) and Knights (privileged quick-actions). What guarantees integrity is not "admin ops never touch the network" but that the Keep is the only writer and every command passes the one validated door. Authoring stays local to the Keep.
- **Per-user identity, tenant isolation.** Every household is a fully isolated tenant (schema-level); every actor is a per-user account with a role, and every event carries the acting subject. The domain stays tenant-agnostic — tenancy is provisioned around it, not baked into it (A-0002/A-0004/A-0005).
- **Offline-first for both phones.** Viewing and queuing actions never require the computer to be reachable; outboxes flush idempotently on reconnect.
- **Idempotency everywhere.** Duplicate submissions of a client-minted id are no-ops; the outbox may retry freely.

## Constraints **[REQUIRED]**

- **Scope (v1):** multi-tenant-capable but **dual-mode** — the MVP runs as a single local tenant (one household, the Keep) on SQLite; the architecture is fully isolated per household at the schema level so a future hosted deployment can run many households on the same code (A-0002). A household has **N Knights (adults) + N Squires (children)**, each a per-user account (A-0004). No Play Store publishing, no cloud relay / off-network sync, no photo evidence, no streak freezes.
- **Tech stack:** computer = Rust (engine + store + local API + admin UI); phone = native Kotlin + Jetpack Compose with Room. Core is shared/portable. Persistence is **Diesel dual-backend (Postgres + SQLite interchangeable)** — local SQLite for the MVP, hosted Postgres for the future multi-tenant deployment, same code (A-0003).
- **Accounts & tenancy:** per-user accounts with a **registration path** — registration creates a household (its fully isolated tenant) and adds Knights & Squires (A-0004). Each tenant is schema-isolated (A-0002). Access control is **per-user tenant-scoped tokens** carrying a role (Knight = privileged; Squire = read + propose), superseding shared pairing secrets. Authoring remains on the Keep; the Knight phone does quick-actions only.
- **Network:** home LAN only (local HTTP) for the MVP, for both phones; computer address may change, so each phone must degrade gracefully to cache + outbox.
- **Time:** all `on` dates and day boundaries computed in one configured household timezone; streak/due logic must be timezone-stable across travel and DST.
- **Durability:** the event log is the system of record, persisted and surviving restarts; a full-store single-file export must exist for backup.

## Open Items (implementer's discretion)

- Admin UI form factor: CLI / TUI / local web page served by the same process.
- LAN discovery: manual host:port entry vs mDNS.
- Rust storage library: **resolved → Diesel dual-backend (Postgres + SQLite)** (A-0003).