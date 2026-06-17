---
id: knight-app-compose-review-quick
level: task
title: "Knight app: Compose review/quick-action UI + KnightApi transport (lazy-login) + Room durability + live demo"
short_code: "SQUIRE-T-0040"
created_at: 2026-06-17T16:58:52.860702+00:00
updated_at: 2026-06-17T16:58:52.860702+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Knight app: Compose review/quick-action UI + KnightApi transport (lazy-login) + Room durability + live demo

## Parent Initiative

[[SQUIRE-I-0001]] · Spec: [[SQUIRE-S-0006]] (REQ-K1–K10, NFR-1/2/5/6) · blocked_by [[SQUIRE-T-0039]] · siblings [[SQUIRE-T-0034]]/[[SQUIRE-T-0037]]

## Objective

Ship the parent's pocket review app: a new `:knight-app` Android module (Compose) over `:knight-core` + `:sdk`. It renders the cross-Squire `HouseholdReview` (per-Squire balances + the household-wide pending claim/redemption queues) and exposes the quick-actions — approve/reject a claim, approve/reject a redemption, redeem directly, add funds (reason required), mark a quest done for a child. Transport adapts `:knight-core`'s `KnightSubmit`/fetch ports onto the generated `KnightApi`/`ControlApi` with **lazy login** (the T-0037 fix) and status→`SubmitResult` mapping. Cache + privileged outbox persist in Room (durable across restart). Seed a Knight login in `squire-home` and prove the full review loop live on the emulator: child claims on the Squire app → parent approves on the Knight app → child sees the credit.

## Acceptance Criteria

- [ ] `:knight-app` Android module (applicationId `com.squire.knight`, own `MainActivity`/manifest with INTERNET + cleartext) added to `settings.gradle.kts`; installs alongside the Squire app on the emulator.
- [ ] Transport adapter binds `:knight-core` ports to the generated `KnightApi` (`householdReview`, `reviewClaim`, `reviewRedemption`, `redeem`, `adjust`, `markDone`) + `ControlApi.login`, with the lazy `ensureLoggedIn()` token guard and a status-code→`SubmitResult` mapping (2xx→Ack, 409→AlreadyDone, 403 occurrence-taken→AlreadyDone, other 4xx→Poison, IO→Offline).
- [ ] Compose review home: per-Squire balance chips; a pending-claims section (Approve / Reject) and a pending-requests section (Approve / Reject), each labeled with its Squire; an add-funds action with a **required** reason (blocked until non-empty, REQ-K6); offline banner when rendering from cache. Quick-actions route through the ViewModel → `:knight-core`.
- [ ] Room durability: `ReviewCache` + privileged outbox persist (own Room db, `knight.db`); a queued action survives an app kill and flushes on reconnect (the T-0037 pattern).
- [ ] `squire-home` seeds a Knight login usable by the app (Knight `UserId(1)`/`demo`); `cargo build` green. Live emulator demo: Squire claims "Tidy your room" (review-required) → it appears in the Knight queue → parent taps Approve → child's balance reflects it. `./gradlew :knight-app:assembleDebug` builds; `:knight-core:test` + `:core:test` green. Screenshot captured.

## Implementation Notes

### Technical Approach
Mirror the Squire `:app` (MainActivity wiring, RoomOutbox/RoomStateCache pattern, SquireApiAdapter→here `KnightApiAdapter`). The privileged outbox stores `{id, kind, payloadJson}` like the Squire one; payload is the SDK request DTO serialized with kotlinx. The adapter maps `com.squire.sdk.infrastructure.ClientException.statusCode` onto `SubmitResult`. UI: a `KnightHomeScreen(state, on…)` stateless composable + `KnightViewModel` mirroring `SquireViewModel.refresh()=syncNow`. Reuse `clients/android-env.sh` + the AVD "squire". Keep demo creds memorable (`demo`/`demo`).

### Dependencies
[[SQUIRE-T-0039]] (`:knight-core`), [[SQUIRE-T-0038]] (decodable `DecisionDto`), `squire-home` (co-hosted Keep+api over one store), the running AVD + toolchain from [[SQUIRE-T-0032]].

### Risk Considerations
Two apps sharing one emulator/host — keep `com.squire.knight` distinct from `com.squire.app` and a separate `knight.db`. The Knight is gated by `RequireKnight`; logging in as the Squire (user 2) would 403 every action — wire user 1 (`demo`). Live-refresh friction (manual Refresh only) is inherited and out of scope here. Pairing/secure token storage remain deferred (NFR-5) — demo creds are baked, local-only.

## Status Updates

*To be added during implementation*
