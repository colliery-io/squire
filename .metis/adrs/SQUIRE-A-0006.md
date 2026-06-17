---
id: 001-simplify-reward-availability-to
level: adr
title: "Simplify reward availability to Once / Repeatable (no rate-limit math)"
number: 1
short_code: "SQUIRE-A-0006"
created_at: 2026-06-17T02:51:53.568337+00:00
updated_at: 2026-06-17T02:53:27.540342+00:00
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

# ADR-1: Simplify reward availability to Once / Repeatable (no rate-limit math)

**Status:** Decided (2026-06-16) · **Decision maker:** Dylan Storey
**Supersedes:** the availability sub-decision in SQUIRE-A-0005 and the rate-limit/stock language in PRD §7.6 (FR-R4). **Affects:** `shared_contract.rs` (applied), SQUIRE-S-0001 (Domain Core), SQUIRE-S-0003/0004/0005/0006.

## Context **[REQUIRED]**

SQUIRE-A-0005 modeled reward availability as `Unlimited` / `LimitedTotal{remaining}` / `PerDay{max}` / `PerWeek{max}`, with per-Squire vs household scoping rules and derived rate-limit counting. Two problems: (1) it's more machinery than the MVP wants — we don't want the system enforcing per-day/per-week math; and (2) `LimitedTotal { remaining }` is a **stored, decrement-on-use counter on a definition**, which contradicts AR-3 ("counters are derived, never stored"). The product intent is simpler: a reward has a cost and is redeemable either once or repeatedly; the parent eyeballs recent use and adjudicates.

## Decision **[REQUIRED]**

Replace the `Availability` enum with two cases:

```rust
enum Availability { Once, Repeatable }
```

- **`Once`** (e.g. a toy): a single household-wide redemption. After the first `ItemRedeemed` for the item it shows **out-of-stock** — **derived from the log** (does an `ItemRedeemed` exist for this item?), no stored counter, consistent with AR-3.
- **`Repeatable`**: no limit enforced. The view surfaces the **last redemption** (`RewardCard.last_redeemed`, the most recent `ItemRedeemed` for the item) so a parent can see recent use and decide. **No rate-limit math.**
- **Both deduct the cost** on redemption (unchanged `ItemRedeemed { cost }`).
- Drop `Blocked::RateLimited` and `LockReason::RateLimited`; keep `OutOfStock` (now only a redeemed `Once` item). `can_redeem` reduces to: item active, gate unlocked, balance ≥ cost, and — if `Once` — not already redeemed.

## Alternatives Analysis **[CONDITIONAL: Complex Decision]**

| Option | Pros | Cons | Risk | Cost |
|--------|------|------|------|------|
| **`Once` / `Repeatable`, no math (chosen)** | Matches product intent; trivial `can_redeem`; out-of-stock derived from log (fixes the AR-3 smell); parents adjudicate by sight | No system-enforced rate limits (by design) | Low | S |
| Keep rich availability (`PerDay`/`PerWeek`/`LimitedTotal`) + per-Squire scope | System-enforced limits; granular | Over-built for MVP; `LimitedTotal{remaining}` violates AR-3 unless reworked to a derived cap; more logic to test | Med | M |
| Fix only the smell (`LimitedTotal{remaining}`→`{total}` derived) but keep all four | Less enforcement removed | Still more machinery than wanted; keeps unused rate-limit paths | Low | M |

## Rationale **[REQUIRED]**

The household doesn't want the app policing per-day/per-week redemptions — the parents are in the loop and would rather *see* recent use than have it blocked by rules. Collapsing to `Once`/`Repeatable` delivers exactly that, makes `can_redeem` almost trivial, and — as a bonus — removes the one place we were storing a mutable counter on a definition (the `LimitedTotal{remaining}` smell), because `Once`'s out-of-stock is now derived from the event log like everything else. Richer limits can be reintroduced additively behind the same enum if a real need appears.

## Consequences **[REQUIRED]**

### Positive
- `can_redeem` is minimal: active + gate + balance + (Once ⇒ not already redeemed).
- All availability state stays **derived from the log** (AR-3) — no stored counters anywhere.
- Parents get visibility (`last_redeemed`) instead of opaque enforcement.

### Negative
- No automatic rate limiting; a child could repeatedly redeem a `Repeatable` reward until out of points (acceptable — cost still gates it, and the parent sees the activity).
- Supersedes part of A-0005 and PRD FR-R4; those docs note the change.

### Neutral
- `RewardCard` gains `last_redeemed: Option<Timestamp>` (informational, derived).
- Per-Squire vs household scoping is now moot for MVP; `Once` is household-wide. Richer/scoped limits remain a future, additive option.