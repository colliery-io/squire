---
id: 001-authoring-audit-via-last-editor
level: adr
title: "Authoring audit via last-editor metadata at the store layer"
number: 1
short_code: "SQUIRE-A-0007"
created_at: 2026-06-17T02:51:54.452301+00:00
updated_at: 2026-06-17T02:53:28.486629+00:00
decision_date: 
decision_maker: 
parent: 
archived: false

tags:
  - "#adr"
  - "#phase/decided"


exit_criteria_met: false
initiative_id: NULL
---

# ADR-1: Authoring audit via last-editor metadata at the store layer

**Status:** Decided (2026-06-16) · **Decision maker:** Dylan Storey
**Relates to:** SQUIRE-A-0004 (identity), SQUIRE-A-0005 (event `actor`). **Affects:** `shared_contract.rs` (`Repository::apply` — applied), SQUIRE-S-0002 (Persistence), SQUIRE-S-0007 (Identity), SQUIRE-S-0004 (the Keep).

## Context **[REQUIRED]**

Activity in the event log now records both the Squire *subject* and the Knight *actor* (A-0005). But **definitions** — quests, items, achievements, users — live in mutable tables, and nothing records who created/edited/archived them. With multiple co-managing Knights, "who set the trash reward to 15?" or "who archived this quest?" has no answer. We decided that knowing the **last editor** is sufficient — full change-history is not needed for MVP.

## Decision **[REQUIRED]**

- **Last-editor metadata at the store layer:** each definition table (quests, items, achievements, users) gets `created_by: Option<UserId>`, `created_at`, `updated_by: Option<UserId>`, `updated_at` columns. This answers "who set X (and when)" without storing a history of prior values.
- **Domain types stay pure.** `Quest` / `RedeemableItem` / `Achievement` / `User` do **not** carry audit fields — audit is a persistence concern, not a domain one (AR-7).
- **Thread the actor alongside the write port, not on each `Change`:** `Repository::apply(by: Option<UserId>, changes)`. The store stamps `created_by`/`updated_by` (+ timestamps via `Clock`) on `PutQuest`/`PutItem`/`PutAchievement`/`PutUser`/`SetXActive` rows from `by`; `Append(Event)` ignores `by` (an event already carries its own `actor`/`squire`). `by = None` for system/seed writes.
- The API / Keep supplies `by` = the authenticated acting user when it calls `apply`.
- **Full change-history (an audit stream of every definition edit) is deferred** — add later if "how did this evolve" becomes a real need.

## Alternatives Analysis **[CONDITIONAL: Complex Decision]**

| Option | Pros | Cons | Risk | Cost |
|--------|------|------|------|------|
| **Last-editor columns, actor via `apply(by, …)` (chosen)** | Answers "who set X" cheaply; domain stays pure; one clean seam for the actor | No history of prior values; `apply` signature gains a param | Low | S |
| Audit fields on the domain structs | Self-contained | Pollutes pure domain types with persistence/auth metadata (breaks AR-7) | Low | S |
| Full definition-change audit stream | Complete history (who changed what, when, from→to) | More to build/store; not needed for the MVP question | Low | M |

## Rationale **[REQUIRED]**

"Who set X" is the actual question, and last-editor metadata answers it with four columns and no new subsystem. Keeping the audit off the domain types preserves the pure core (AR-7); putting the actor *alongside* `apply` (rather than on every `Change`) is right because the pure `Engine::handle` produces authoring `Change`s without knowing — and shouldn't know — the caller's identity. The acting user is known at the call site (the authenticated API/Keep), so that is where it enters.

## Consequences **[REQUIRED]**

### Positive
- Cheap, sufficient answer to "who created/edited/archived this definition," for any number of Knights.
- Domain core stays pure; the actor enters through one well-defined seam (`apply`).

### Negative
- Only the *last* editor is known — no history of prior values (deferred).
- `Repository::apply` gains a `by: Option<UserId>` parameter; all call sites must pass it.

### Neutral
- Timestamps come from the injected `Clock` (testable).
- This is symmetric with A-0005's event `actor`: activity audit lives on events; definition audit lives on table columns — each in its own storage shape.