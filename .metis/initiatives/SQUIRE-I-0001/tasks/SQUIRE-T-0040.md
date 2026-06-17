---
id: knight-app-compose-review-quick
level: task
title: "Knight app: Compose review/quick-action UI + KnightApi transport (lazy-login) + Room durability + live demo"
short_code: "SQUIRE-T-0040"
created_at: 2026-06-17T16:58:52.860702+00:00
updated_at: 2026-06-17T17:22:14.429771+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Knight app: Compose review/quick-action UI + KnightApi transport (lazy-login) + Room durability + live demo

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0006]] (REQ-K1–K10, NFR-1/2/5/6) · blocked_by [[SQUIRE-T-0039]] · siblings [[SQUIRE-T-0034]]/[[SQUIRE-T-0037]]

## Objective

Ship the parent's pocket review app: a new `:knight-app` Android module (Compose) over `:knight-core` + `:sdk`. It renders the cross-Squire `HouseholdReview` (per-Squire balances + the household-wide pending claim/redemption queues) and exposes the quick-actions — approve/reject a claim, approve/reject a redemption, redeem directly, add funds (reason required), mark a quest done for a child. Transport adapts `:knight-core`'s `KnightSubmit`/fetch ports onto the generated `KnightApi`/`ControlApi` with **lazy login** (the T-0037 fix) and status→`SubmitResult` mapping. Cache + privileged outbox persist in Room (durable across restart). Seed a Knight login in `squire-home` and prove the full review loop live on the emulator: child claims on the Squire app → parent approves on the Knight app → child sees the credit.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] `:knight-app` Android module (applicationId `com.squire.knight`, own `MainActivity`/manifest with INTERNET + cleartext) added to `settings.gradle.kts`; installs alongside the Squire app on the emulator.
- [x] Transport adapter binds `:knight-core` ports to the generated `KnightApi` (`householdReview`, `reviewClaim`, `reviewRedemption`, `redeem`, `adjust`, `markDone`) + `ControlApi.login`, with the lazy `ensureLoggedIn()` token guard and a status-code→`SubmitResult` mapping (2xx→Ack, 409/403→AlreadyDone, other 4xx→Poison, 5xx/IO→Offline).
- [x] Compose review home: per-Squire balance + Add-funds; a pending-claims section (Approve / Reject) and a pending-requests section (Approve / Reject), each labeled with its Squire; an add-funds dialog with a **required** reason (Add disabled until amount>0 AND non-blank reason, REQ-K6); offline banner when rendering from cache. Quick-actions route through the ViewModel → `:knight-core`.
- [x] Room durability: `ReviewCache` + privileged outbox persist (own Room db, `knight.db`, composite-key idempotent outbox); a queued Approve survived an app force-stop and flushed on reconnect (verified live).
- [x] `squire-home` already seeds the Knight login (`UserId(1)`/`demo`); `cargo build` green. Live emulator demo passed: Squire's pending "Tidy your room" claim appeared in the Knight queue → parent tapped Approve → balance 0→10; then offline Approve of a redemption request survived a kill and flushed on reconnect (balance 10→7). `./gradlew :knight-app:assembleDebug` builds; `:knight-core:test` + `:core:test` green. Screenshots captured.

## Implementation Notes

### Technical Approach
Mirror the Squire `:app` (MainActivity wiring, RoomOutbox/RoomStateCache pattern, SquireApiAdapter→here `KnightApiAdapter`). The privileged outbox stores `{id, kind, payloadJson}` like the Squire one; payload is the SDK request DTO serialized with kotlinx. The adapter maps `com.squire.sdk.infrastructure.ClientException.statusCode` onto `SubmitResult`. UI: a `KnightHomeScreen(state, on…)` stateless composable + `KnightViewModel` mirroring `SquireViewModel.refresh()=syncNow`. Reuse `clients/android-env.sh` + the AVD "squire". Keep demo creds memorable (`demo`/`demo`).

### Dependencies
[[SQUIRE-T-0039]] (`:knight-core`), [[SQUIRE-T-0038]] (decodable `DecisionDto`), `squire-home` (co-hosted Keep+api over one store), the running AVD + toolchain from [[SQUIRE-T-0032]].

### Risk Considerations
Two apps sharing one emulator/host — keep `com.squire.knight` distinct from `com.squire.app` and a separate `knight.db`. The Knight is gated by `RequireKnight`; logging in as the Squire (user 2) would 403 every action — wire user 1 (`demo`). Live-refresh friction (manual Refresh only) is inherited and out of scope here. Pairing/secure token storage remain deferred (NFR-5) — demo creds are baked, local-only.

## Status Updates

**2026-06-17 — Done.** New `:knight-app` module (`com.squire.knight`) added to `settings.gradle.kts`. Files: `data/db/KnightDb.kt` (`CachedReviewEntity` single-row + `PrivilegedOutboxEntity{seq,key,kind,payloadJson}` with a unique `key` index for idempotent INSERT OR IGNORE, ordered by `seq`), `data/RoomReviewCache.kt`, `data/RoomPrivilegedOutbox.kt` (serializes the 5 SDK request DTOs by kind), `data/KnightApiAdapter.kt` (`ReviewFetcher`+`KnightSubmit` over `KnightApi`/`ControlApi`, lazy `ensureLoggedIn()` as Knight user 1, status→`SubmitResult` mapping via `ClientException.statusCode`), `KnightViewModel.kt`, `ui/KnightHomeScreen.kt` (Squires + Add-funds dialog with required reason, Pending claims, Pending requests, offline banner), `MainActivity.kt`.

Two bugs fixed during bring-up: (1) the manifest `android:name=".MainActivity"` resolved to `com.squire.knight.MainActivity` but the class is in `com.squire.knight.app` → ClassNotFound crash; fully-qualified the activity name. (2) A KDoc line `**IO**/unknown` contained the substring `*/` which closed the comment early → parse errors; reworded.

Also caught a live-demo gotcha (not a code bug): the running `squire-home` was the **pre-T-0038 binary**, so `{"verdict":"approve"}` came back 422 → mapped to Poison → silently dropped. Rebuilt + restarted `squire-home` (new flat-`DecisionDto` contract) and the loop worked.

**Live emulator demo (against fresh `squire-home`):** Squire's pending "Tidy your room" claim rendered in the Knight queue (Gawain 0 pts) → tapped **Approve** → server `balance 0→10`, claim cleared, UI refreshed. **Durability:** Squire requested an Ice-cream redemption → Knight (online) cached the queue → went **offline** → tapped Approve (queued `REVIEW_REDEMPTION:700700700` in `knight.db`) → **force-stopped** (row survived) → back **online** + relaunch → startup `syncNow()` lazy-logged-in and flushed → `balance 10→7`, request cleared, outbox drained to 0. `:knight-app:assembleDebug` builds; `:knight-core:test` (12) + `:core:test` green.