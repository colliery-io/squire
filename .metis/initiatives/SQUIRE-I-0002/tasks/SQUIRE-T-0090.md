---
id: background-update-checks-via
level: task
title: "Background update checks via WorkManager + update-available notification"
short_code: "SQUIRE-T-0090"
created_at: 2026-06-19T19:44:35.979453+00:00
updated_at: 2026-06-19T19:44:35.979453+00:00
parent: SQUIRE-I-0002
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: true
initiative_id: SQUIRE-I-0002
---

# Background update checks via WorkManager + update-available notification

## Parent Initiative

[[SQUIRE-I-0002]] — "the app runs in the background + fetches upgrades" from [[SQUIRE-A-0012]].

## Objective

Keep the app pulling upgrades while backgrounded: a periodic WorkManager job asks the paired home
server whether a newer APK is published (content-hash detection, [[SQUIRE-T-0085]]) and posts a
notification the user taps to open the app and install.

## Approach & rationale

- **WorkManager** (periodic, 6h, network-constrained) rather than an always-on **foreground service**:
  app updates are rare, so a battery-friendly OS-scheduled check with no permanent notification fits
  far better; near-real-time would need FCM push (a separate, larger effort).
- `UpdateCheckWorker` (CoroutineWorker): loads the `Session`, runs `UpdateChecker.check`, and on a hit
  posts a notification (`squire_updates` channel) whose tap opens `MainActivity` (→ the existing
  in-app install flow). Scheduled when a session exists / on pair; cancelled on forget. Best-effort.
- `POST_NOTIFICATIONS` added to the manifest + runtime-requested on Android 13+.

## Acceptance Criteria

- [x] A periodic WorkManager job checks for a newer APK while backgrounded (network-constrained).
- [x] On an available update it notifies; tapping opens the app to install.
- [x] Scheduled on pair / startup-with-session, cancelled on forget; no session / offline = no-op.
- [x] App compiles with the worker + permission wiring.

## Status / follow-ups

- Done (compile-verified). Full runtime validation (periodic firing + notification) needs a device.
- Follow-up: background **state** sync (keep the StateView cache warm for the paired role) — left out
  to avoid replicating the Player/Knight sync stacks untested; the app already syncs on open.

## Status Updates

- Implemented `UpdateCheckWorker` + scheduling in `MainActivity` + `POST_NOTIFICATIONS`; added the
  `androidx.work` dependency. `:app:compileDebugKotlin` green. Device validation deferred.
