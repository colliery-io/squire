---
id: 001-privileged-command-idempotency-via
level: adr
title: "Privileged-command idempotency via log-carried CommandId"
number: 1
short_code: "SQUIRE-A-0001"
created_at: 2026-06-17T01:47:49.235536+00:00
updated_at: 2026-06-17T01:49:24.610185+00:00
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

# ADR-1: Privileged-command idempotency via log-carried CommandId

**Status:** Decided (2026-06-16) · **Decision maker:** Dylan Storey
**Affects:** `shared_contract.rs`, SQUIRE-S-0003 (Local API), SQUIRE-S-0006 (Knight), SQUIRE-I-0001

## Context **[REQUIRED]**

Adding the parent quick-admin phone (the Knight, SQUIRE-S-0006) put privileged commands on the network, flushed from an **offline outbox that retries**. Retry-safety requires every privileged command to be idempotent.

Most are already safe via *natural keys carried in the append-only event log*:
- `SubmitClaim` / `RequestRedemption` emit `CompletionClaimed{claim_id}` / `RedemptionRequested{request_id}`; a replay is recognized because the id is already in the log.
- `ReviewClaim` / `ReviewRedemption` key on `claim_id` / `request_id`; the engine rejects a second review (`AlreadyReviewed`), so a replay is a no-op.
- "Mark-done" mints a `claim_id` and reuses the claim path — covered by the same mechanism.

The gap: **direct `RedeemItem` and `AdjustPoints` carried no client id.** A blind outbox retry would emit a second `ItemRedeemed` / `PointsAdjusted` — a double-spend or double-credit. We need a retry key for exactly these two, without compromising the architecture's invariant that *all derived state (incl. dedup) comes from the append-only log* (AR-3).

## Decision **[REQUIRED]**

Introduce a client-minted `CommandId(u128)` and **carry it onto the emitted event**, for the two commands lacking a natural key:

- `Command::RedeemItem { command_id: CommandId, item_id }`
- `Command::AdjustPoints { command_id: CommandId, amount, reason }`
- `Event::ItemRedeemed { request_id: Option<RequestId>, command_id: Option<CommandId>, .. }` — `command_id` is `Some` for a direct client redeem; request-originated redeems keep deduping on `request_id` (so `command_id` is `None` there).
- `Event::PointsAdjusted { command_id: CommandId, .. }` — always present.

Dedup stays **derived from the log**: when `handle` processes a `RedeemItem`/`AdjustPoints`, it scans the event log for an emitted event already carrying that `command_id`; if found, it returns `Ok(vec![])` (idempotent no-op) and the API re-returns current state. Identical mechanism to `claim_id`/`request_id`. No separate "processed-commands" table.

`ReviewClaim`, `ReviewRedemption`, `SubmitClaim`, `RequestRedemption`, and all authoring commands are **unchanged** — they already have natural keys or are local-only and non-retried.

The change is **additive and design-only** (no behavior is implemented yet). Applied to `shared_contract.rs` as part of this ADR.

## Alternatives Analysis **[CONDITIONAL: Complex Decision]**

| Option | Pros | Cons | Risk | Cost |
|--------|------|------|------|------|
| **A. Log-carried `CommandId` on the two commands (chosen)** | Dedup derived from the append-only log, same as claims/requests; no new mutable state; minimal surface (two commands); honors AR-3/AR-4 | Two event variants gain a field; the engine must scan for the id (cheap at single-child scale) | Low | XS |
| B. Command envelope `{command_id, command}` + a `processed_commands` table the store dedupes on | Uniform across *all* commands | Reintroduces mutable dedup state **outside** the log — directly contradicts AR-3 ("derived, never stored"); store grows a second source of truth; bigger change | Medium | S |
| C. No client id — make the outbox "at-most-once" (don't retry redeem/adjust) | No contract change | Defeats offline-first for the parent; a dropped ack loses an action or risks a double on manual resend; fragile | High | XS |

## Rationale **[REQUIRED]**

Option A keeps the system's defining property intact: **the append-only event log is the single source of truth, and everything — including idempotency — is derived from it.** It reuses the exact pattern that already makes child submissions safe, so there's one idempotency story across the whole system, not two. Option B's envelope is superficially "cleaner" (uniform) but smuggles a mutable processed-id set alongside the log, which is precisely the counter-derived state AR-3 exists to forbid; it would also make balance/audit reasoning depend on something outside the log. Option C trades away the offline-first parent experience we just decided to build.

Scoping the id to only `RedeemItem`/`AdjustPoints` is deliberate: the other privileged commands already dedupe naturally, so adding ids there would be noise.

## Consequences **[REQUIRED]**

### Positive
- Parent quick-actions are retry-safe from an offline outbox: no double-spend, no double-credit.
- One idempotency mechanism system-wide (log-carried client ids); nothing to reconcile between a log and a side table.
- Audit/observability unaffected — the id lives on the very event that explains the balance change.

### Negative
- `ItemRedeemed` and `PointsAdjusted` each gain a field; any future event (de)serialization must include it.
- `handle` does an id-existence check against the log for these two commands (negligible at target scale; bounded by NFR-8).

### Neutral
- A local Keep-originated `RedeemItem`/`AdjustPoints` also mints a `CommandId` (harmless, and gives the desktop admin the same double-submit protection).
- Does **not** define the parent wire DTOs or the dedup *storage representation* — those remain open decision areas in SQUIRE-S-0003 (parent state-read shape, transport) and SQUIRE-S-0002 (how processed ids are indexed for the scan).