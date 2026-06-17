# Chore-Quest — Product Requirements Document

**Status:** v1 design, ready for implementation
**Companion artifact:** `shared_contract.rs` (authoritative type definitions — the engine, API, and phone client all build against it)

---

## 1. Overview

Chore-Quest is a local, unpublished household app that gamifies chores for a single child. The parent authors chores, rewards, and achievements and reviews submissions on a computer; the child views quests and progress and submits completions on an Android phone. Points are earned only when the parent (or an auto-approve rule) confirms a submission, and are spent only on parent-approved redemptions.

The system is two cooperating apps with a strict trust boundary: the **computer is the single source of truth and the only writer**; the **phone is a client** with a local cache and an offline outbox.

## 2. Goals

- Let the parent define and maintain chores, point values, streak-based achievements, and a reward store quickly, with a keyboard.
- Let the child see what to do, track progress (points + streaks), see rewards, and submit "I did X" and "I want to redeem Y".
- Guarantee the child cannot grant themselves points or redemptions.
- Work smoothly when the two devices are not online at the same time.

## 3. Non-Goals (v1)

- Play Store publishing.
- More than one child / multi-household.
- Submitting or syncing from outside the home network (no cloud relay).
- Photo/video evidence on submissions.
- Streak "freeze"/grace days.

## 4. Actors

- **Parent (Admin)** — operates on the computer. Authors definitions, reviews claims and redemption requests, redeems directly, makes manual point adjustments.
- **Child (Player)** — operates on the phone. Reads state, submits completion claims, requests redemptions.

## 5. Architecture (constraints, not suggestions)

- **AR-1** The computer holds the canonical store and is the only process that writes it (single-writer).
- **AR-2** Storage is split into two shapes: **definitions** (quests, items, achievements) are mutable records; **activity** (claims, approvals, redemptions, rejections, achievement unlocks, adjustments) is an append-only event log.
- **AR-3** Balance, streaks, "quests due", and achievement-unlocked status are **derived** from the event log, never stored as mutable counters.
- **AR-4** Point values are **snapshotted onto the approval event**, so editing a quest's reward never rewrites past history.
- **AR-5** Every mutation (define, archive, review, redeem, adjust, claim) passes through one validated entry point (`handle()` → list of `Change`s); all rules live there.
- **AR-6 Tech stack:** computer side = Rust (engine + store + local API + admin UI), SQLite for persistence. Phone side = native Kotlin + Jetpack Compose, Room (SQLite) for the local cache/outbox.
- **AR-7** The domain core (types, `handle`, projections) has no I/O dependencies and is shared/portable.
- **AR-8 Trust boundary:** the network API exposes only read-state and child-submission operations (§9). Approve, reject, redeem, define, and archive are local admin operations and are never reachable over the network.

## 6. Domain model (summary; see `shared_contract.rs` for exact types)

- **Quest** — `title`, optional `description`, optional free-form `category`, `reward: Points`, `cadence` (one-off with optional due date, or recurring `Daily` / `Weekly{days}` / `EveryNDays{n, anchor}`), `auto_approve: bool`, `repeatable_within_day: bool`, `active: bool`, optional `icon`.
- **RedeemableItem** — `name`, `cost: Points`, optional `gate: AchievementId`, `availability` (`Unlimited` / `LimitedTotal` / `PerDay` / `PerWeek`), `active`, optional `icon`.
- **Achievement** — `name`, `criterion` (`Streak{scope, length, basis}` / `TotalCompletions{scope, count}` / `PointsEarned{total}`), `bonus_points`, `active`. `scope` is a specific quest, a category, or any.
- **Events** — `CompletionClaimed`, `CompletionApproved (+points)`, `CompletionRejected`, `ItemRedeemed (−cost)`, `AchievementUnlocked (+bonus)`, `PointsAdjusted (±)`, `RedemptionRequested`, `RedemptionRejected`.

---

## 7. Functional Requirements

### 7.1 Authoring (Parent)

- **FR-A1** The parent can create, edit, and archive a Quest with all fields in §6. Archiving never deletes; archived quests remain referenced by historical events.
- **FR-A2** The parent can create, edit, and archive a RedeemableItem, including its cost, optional achievement gate, and availability rule.
- **FR-A3** The parent can create, edit, and archive an Achievement, choosing its criterion, scope, and bonus points.
- **FR-A4** Categories are free-form text entered at quest-definition time; no separate management screen is required. Any achievement scoped to a category applies to all quests sharing that label.
- **FR-A5** Editing a quest's reward affects only future approvals; previously approved completions keep their snapshotted point value.

### 7.2 Quest scheduling & "due" logic

- **FR-Q1** "Quests due on date D" is derived: active quests whose cadence matches D, minus quests already satisfied for D.
- **FR-Q2** A one-off quest is due until it has an approved completion; a recurring quest is due on each matching scheduled day.
- **FR-Q3** For `EveryNDays`, due days are computed from the `anchor` date and interval `n`.
- **FR-Q4** If `repeatable_within_day` is false, only one open/approved completion is allowed per quest per day; a second claim is rejected (`AlreadyClaimedToday`). If true, each claim is independent and pays out again.

### 7.3 Completion claim & review

- **FR-C1** The child can submit a completion claim for a due quest, for a given date `on` (defaulting to today).
- **FR-C2** A claim is appended as `CompletionClaimed` and is worth zero points until reviewed.
- **FR-C3** If the quest's `auto_approve` is true, the claim is approved immediately on receipt (no parent action), emitting `CompletionApproved`.
- **FR-C4** The parent can approve or reject any pending claim. Approve emits `CompletionApproved` with the quest's current reward snapshotted; reject emits `CompletionRejected` with an optional reason.
- **FR-C5** A claim can be reviewed only once; reviewing an already-resolved claim is rejected (`AlreadyReviewed`).
- **FR-C6** Streaks and achievements use the claim's `on` date (the day the chore was done), not the approval timestamp.

### 7.4 Points & ledger

- **FR-P1** Balance = sum over the event log of approvals (+points), redemptions (−cost), achievement bonuses (+), and adjustments (±). Pending claims/requests contribute nothing.
- **FR-P2** The parent can make a manual point adjustment with a required reason (`PointsAdjusted`).
- **FR-P3** Balance is always recomputable from the log alone; it is never persisted as an authoritative mutable value.

### 7.5 Streaks & achievements

- **FR-S1** A streak for a quest-scoped achievement counts consecutive **scheduled occurrences** completed (respecting that quest's schedule); a gap on a non-scheduled day does not break it.
- **FR-S2** A streak for a category- or any-scoped achievement counts consecutive **calendar days** with at least one in-scope completion.
- **FR-S3** A repeatable quest counts as a single occurrence per day toward any streak, regardless of how many times it was completed that day.
- **FR-S4** When a criterion is first satisfied, the engine emits `AchievementUnlocked` once; the unlock is sticky and survives a later streak break.
- **FR-S5** Achievement unlock awards `bonus_points` (if any) and unlocks any RedeemableItem gated on it.
- **FR-S6** Streak counters are derivable and displayable for any scope without defining an achievement.

### 7.6 Redemption

- **FR-R1** The child can request a redemption of a RedeemableItem (`RedemptionRequested`); this never spends points on its own.
- **FR-R2** The parent can approve or reject a redemption request. Approve emits `ItemRedeemed` (with the originating `request_id`); reject emits `RedemptionRejected` with an optional reason.
- **FR-R3** The parent can also redeem an item directly without a prior request (`ItemRedeemed` with no `request_id`).
- **FR-R4** A redemption is permitted only if, **at the moment of commit**: the item is active, its gating achievement (if any) is unlocked, the balance covers the cost, and availability is not exceeded. Otherwise it is blocked with a specific reason (insufficient points / locked / out of stock / rate-limited).
- **FR-R5** No points are reserved while a request is pending; affordability is validated only at approval time.

### 7.7 Player app (phone)

- **FR-PL1** The phone renders entirely from a single `StateView` payload: today's quests with status, current balance, streaks, available rewards, and the status of the child's recent claims and requests.
- **FR-PL2** Each reward shows whether it is affordable and, if locked, why (needs achievement / out of stock / rate-limited).
- **FR-PL3** Each quest shows its status (available / pending review / completed today).
- **FR-PL4** The child can submit a claim and a redemption request from the phone.
- **FR-PL5** The UI is age-appropriate and emphasizes progression feedback (points, streaks, milestone proximity).

### 7.8 Sync & offline

- **FR-SY1** The phone caches the last `StateView` and renders fully from cache when the computer is unreachable.
- **FR-SY2** Claims and redemption requests created offline are stored in a local outbox and submitted when the computer becomes reachable.
- **FR-SY3** Every child submission carries a phone-minted id (`claim_id` / `request_id`); resubmitting the same id is idempotent and must not create a duplicate.
- **FR-SY4** "Sync" = flush the outbox, then re-fetch `StateView`. Pending items resolve when their status changes in the refreshed state.

### 7.9 API (the only network surface)

- **FR-API1** `GET /state` → `StateView`.
- **FR-API2** `POST /claims` (`SubmitClaimReq` → `SubmitClaimResp`); idempotent on `claim_id`.
- **FR-API3** `POST /redemption-requests` (`RequestRedemptionReq` → `RequestRedemptionResp`); idempotent on `request_id`.
- **FR-API4** No other operations are exposed. Admin operations are not routable.

### 7.10 Admin app (computer)

- **FR-ADM1** Provides a review queue of pending claims and redemption requests, each approvable/rejectable.
- **FR-ADM2** Provides authoring for quests, items, and achievements (FR-A*).
- **FR-ADM3** Provides direct redemption and manual point adjustment.
- **FR-ADM4** Form factor (CLI, TUI, or local web page served by the same process) is an implementation choice, as long as it talks to the engine directly and never through the network API.

---

## 8. Non-Functional Requirements

- **NFR-1 Offline-first (phone).** Viewing and queuing submissions must never require the computer to be reachable. Neither device needs to be online simultaneously for the child to act.
- **NFR-2 Integrity.** The phone must be incapable of minting points, approving its own submissions, or redeeming. The authoritative balance exists only on the computer and is derived from the log.
- **NFR-3 Idempotency.** Duplicate submissions of the same client-minted id are no-ops; the outbox may retry freely.
- **NFR-4 Durability.** The event log is the system of record, persisted to disk, and survives process/host restarts. Definitions are persisted. Provide a data export (full store to a single file) for backup.
- **NFR-5 Lightweight access control.** The API requires a pairing secret established once between phone and computer, so another device on the LAN cannot read state or submit as the child. No account system.
- **NFR-6 Network scope.** Communication is over the home LAN (local HTTP). The computer's reachability/address may change; the phone must handle "computer not found" gracefully via the cache + outbox.
- **NFR-7 Time handling.** All `on` dates and day boundaries are computed in a single configured household timezone. Streak and "due" logic must be timezone-stable and not shift with travel or daylight-saving changes.
- **NFR-8 Performance.** Targets single-child scale: dozens of active quests, thousands of events accrued over years. All projections (balance, streaks, due list, state view) compute in well under 100 ms on the computer.
- **NFR-9 Testability.** The domain core (`handle` + projections) is pure and unit-testable with an in-memory repository. Include property tests for invariants (e.g., balance never goes negative except via explicit adjustment; an approved claim is counted exactly once).
- **NFR-10 Portability of the core.** The engine and types have no dependency on the storage, transport, or UI layers; swapping SQLite or the API transport must not touch the core.
- **NFR-11 Observability.** The admin side can list the raw event log for a quest/item to explain how a balance or streak was reached.
- **NFR-12 Low review friction.** Auto-approve and a single batched review queue keep routine parent effort minimal.

---

## 9. Acceptance criteria (end-to-end)

- **AC-1** Child completes a daily quest offline → claim queues → on next sync it appears in the parent's review queue → parent approves → child's balance increases by the quest's reward on next refresh.
- **AC-2** A quest with `auto_approve = true` is claimed → points are credited without any parent action.
- **AC-3** Child submits the same claim twice (e.g., outbox retry) → exactly one `CompletionClaimed` exists and exactly one approval is possible.
- **AC-4** A Mon/Wed/Fri quest completed three consecutive scheduled days shows a streak of 3 even though weekend days were skipped.
- **AC-5** A reward gated on a not-yet-unlocked achievement shows as locked on the phone with the achievement name, and cannot be redeemed even with sufficient points.
- **AC-6** Child requests a redemption they can afford, but the parent makes another redemption first that drains the balance → the request fails at approval with "insufficient points".
- **AC-7** Editing a quest's reward from 10 to 15 does not change the points of completions approved while it was 10.
- **AC-8** With the computer offline, the phone still renders quests, balance, streaks, and rewards from cache.

## 10. Open items (implementer's discretion)

- Admin UI form factor (CLI / TUI / local web).
- Exact LAN discovery mechanism (manual host:port entry vs mDNS).
- Storage library on the Rust side (e.g., `rusqlite` vs `sqlx`).

## 11. Future considerations (post-v1)

- Multiple children.
- Cloud relay for submit-from-anywhere.
- Photo evidence on claims.
- Streak freezes / grace days.
- Per-period repeat caps (a max-count instead of a boolean) on repeatable quests.
