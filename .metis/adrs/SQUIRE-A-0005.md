---
id: 001-domain-core-becomes-per-squire
level: adr
title: "Domain core becomes per-Squire: every event carries a subject"
number: 1
short_code: "SQUIRE-A-0005"
created_at: 2026-06-17T02:24:16.263821+00:00
updated_at: 2026-06-17T02:25:10.416959+00:00
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

# ADR-1: Domain core becomes per-Squire: every event carries a subject

**Status:** Decided (2026-06-16) · **Decision maker:** Dylan Storey
**Forced by:** SQUIRE-A-0004 (per-user accounts, N Squires). **Scope:** the per-Squire subject, **quest assignment + completion mode**, and **Knight-actor audit** — all applied to `shared_contract.rs`. **Affects:** SQUIRE-S-0001 (Domain Core), SQUIRE-S-0003/0005/0006 (StateView/QuestStatus + parent read), SQUIRE-I-0001.

## Context **[REQUIRED]**

SQUIRE-A-0004 establishes that a household has **one-or-more Squires** (no assumed single child). The original PRD/contract modeled a **single implicit player**: events carried no subject, and balance/streaks/`StateView` were household-global. With multiple Squires that is wrong — a claim, a balance, a streak, an unlock, and an adjustment each belong to a *specific* Squire. The domain core must attribute activity per-Squire, or the multi-child model is not actually expressible.

## Decision **[REQUIRED]**

Make the domain core **per-Squire (subject-carrying)**, applied additively to `shared_contract.rs`:

- **Identity in the contract:** add `UserId`, `Role { Knight, Squire }`, and a credential-free domain `User { id, role, display_name, active }`. `Snapshot` gains `users: Vec<User>`.
- **Every `Event` carries `squire: UserId`.** The household log is the union of its Squires' sub-logs; per-Squire balance/streaks/state are derived by filtering on `squire` — fully consistent with AR-3 (derive from the log).
- **Subject-bearing commands:** `SubmitClaim` and `RequestRedemption` carry the acting `squire` (the API fills it from the caller's token, so the *wire* request DTOs still omit it); `RedeemItem` and `AdjustPoints` carry the target `squire`. `ReviewClaim`/`ReviewRedemption` need no subject — it is derived from the referenced claim/request.
- **Projections take a `squire`:** `balance/quests_due/current_streak/is_unlocked/can_redeem(snap, squire, …)`.
- **`StateView` is per-Squire** (gains `squire`); a new **`HouseholdReview`** DTO gives the Knight pending work across all Squires + per-Squire balances (provisional shape — refined in SQUIRE-S-0003/S-0006).
- **New `DomainError`s:** `UserNotFound`, `NotASquire` (a command's `squire` must be an active Squire).
- **Items & achievements are a shared catalog; quests are *assigned*.** Every quest is assigned to one-or-more Squires (`Assignment::AllSquires`, or an explicit `Squires(set)`) with a completion mode:
  - **`EachAssignee`** — every assignee completes their own occurrence/claim/payout ("pick up your room").
  - **`Race`** — any one assignee; the occurrence stays open until the **first *approved*** completion, which closes it and is the sole payout ("take out the trash, he who dares wins"). Others may still claim while it's open (a false/early claim doesn't lock out the real doer); reviewing a claim against an already-closed occurrence yields `OccurrenceTaken`.
  New `DomainError`s `NotAssigned` / `OccurrenceTaken`; `quests_due` and claim validation are gated by assignment and (for `Race`) occurrence-open status; `QuestStatus` gains `TakenByOther`. `AllSquires` auto-includes Squires added later.
- **Knight-actor audit (decided).** The five Knight-committed events — `CompletionApproved`, `CompletionRejected`, `ItemRedeemed`, `PointsAdjusted`, `RedemptionRejected` — carry `actor: Option<UserId>` (the acting Knight; `None` = auto-approve / system). The commands `ReviewClaim`/`ReviewRedemption`/`RedeemItem`/`AdjustPoints` carry the acting Knight `actor` (the API fills it from the Knight's token). This gives per-user audit of *who* committed each fact, alongside the Squire *subject*. Squire-originated events (`CompletionClaimed`, `RedemptionRequested`) need no `actor` — the `squire` is the actor.
- **Item availability** — **superseded by ADR SQUIRE-A-0006.** Availability is now simply `Once` (single household-wide redemption; out-of-stock derived from the log) or `Repeatable` (no limit; UI shows `last_redeemed`). No rate-limit math; `can_redeem(snap, squire, item, on)` = active + gate + balance + (Once ⇒ not already redeemed).
- **User lifecycle via the single writer:** `Change::PutUser`/`SetUserActive`, produced by the Identity component (SQUIRE-S-0007), **not** by `Engine::handle` (which never manages users).

Both items once deferred here are now resolved: reward availability was simplified (**ADR SQUIRE-A-0006**, supersedes the availability bullet) and authoring-table audit was decided (**ADR SQUIRE-A-0007** — last-editor columns via `Repository::apply(by, …)`).

## Alternatives Analysis **[CONDITIONAL: Complex Decision]**

| Option | Pros | Cons | Risk | Cost |
|--------|------|------|------|------|
| **`squire` on every event + projections take a squire (chosen)** | Per-Squire balance/streak is a simple log filter; uniform "every fact names its Squire"; stays pure & derive-from-log (AR-3, AR-7) | Touches every event/projection signature; slight denormalization (approval also carries squire) | Low | M |
| Subject only on "root" events (claim/request); derive elsewhere via id joins | Fewer fields | Adjust/unlock/direct-redeem have no root → still need a subject; projections need joins; messier and slower | Medium | M |
| Keep single-player domain; multiplex by running one logical store per Squire | No domain change | Breaks the shared household catalog and cross-Squire parent review; fights schema-per-tenant (tenant = household, not child) | High | L |

## Rationale **[REQUIRED]**

Putting `squire` on every event makes per-Squire balance/streaks a trivial, fast filter over the one log — the same derive-from-the-log discipline that already governs the system (AR-3) — and keeps the core pure (AR-7). The "root-only" variant fails because adjustments, unlocks, and direct redeems have no claim/request to inherit a subject from, and it forces joins into the hot projection path (NFR-8). Running a store-per-Squire was rejected because the tenant boundary is the *household* (A-0002), and the parent's cross-Squire review and the shared quest catalog both need one household store.

## Consequences **[REQUIRED]**

### Positive
- The multi-Squire model is actually expressible; each child has their own balance, streaks, unlocks, and `StateView`.
- Per-Squire derivations remain pure log-filters — no new mutable state, consistent with AR-3/AR-7.
- The parent (Knight) gets a coherent cross-Squire review surface (`HouseholdReview`).

### Negative
- Broad (but additive) contract change: every `Event` (subject + actor), the `Quest` definition (assignment + completion), the privileged + submit commands, all five projections, `StateView`/`QuestStatus`, `Snapshot`, and `DomainError` change shape — the Domain Core spec (SQUIRE-S-0001) and the consumers (S-0003/0005/0006) must be updated.
- The `Race` occurrence-resolution rule (first *approved* completion wins; false claims don't lock out) is non-trivial engine logic the Domain Core spec must pin down with tests.

### Neutral
- Availability was subsequently simplified to `Once`/`Repeatable` (no rate-limit math) — ADR SQUIRE-A-0006 supersedes the availability bullet above.
- Knight-actor audit is on the five privileged events; authoring-table (definition) audit is now decided too — ADR SQUIRE-A-0007 (last-editor columns).
- `Assignment::AllSquires` is dynamic (auto-includes future Squires); an explicit `Squires(set)` is static.