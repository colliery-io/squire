---
id: always-on-background-server
level: task
title: "Always-on background server: launchd LaunchAgent so the home server runs without the app open"
short_code: "SQUIRE-T-0093"
created_at: 2026-06-20T00:38:05.775079+00:00
updated_at: 2026-06-20T00:38:05.775079+00:00
parent: SQUIRE-I-0002
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: SQUIRE-I-0002
---

# Always-on background server: run the home server without the app open

## Parent Initiative

[[SQUIRE-I-0002]] — make "the home server is always reachable by the phones" actually true.

## Problem

The native desktop app ([[SQUIRE-T-0092]]) embeds the server, so the server only runs **while the app
is open** — quit it and phones can't sync. We want the server backgrounded: on login, auto-restarting,
no window required.

## Finding: launchd + unsigned binary is blocked on modern macOS

A macOS **launchd LaunchAgent** running the headless `squire-serve` is the textbook approach, and it
was built + tested — but **modern macOS (Darwin 25.x) will not run the unsigned binary under launchd**:
the process starts but **hangs in `dyld` before `main` and never binds** (verified: stable pid, no
LISTEN sockets, still dead after 40s; `sample` shows the main thread stuck in `dyld4::prepare`).
Direct/Terminal runs work because that launch path treats the binary differently. The plist itself is
correct (`plutil -lint` OK; `launchctl bootstrap` loads it; env vars flow). The blocker is the
**unsigned binary under launchd**, not the config.

Nothing was shipped — the launchd changes were reverted (local only).

## Options (the decision)

1. **Code-sign + notarize** (Developer ID) → the launchd daemon works as designed. Proper, but needs a
   paid Apple Developer account + a signing/notarization step in CI. Also fixes Gatekeeper for the
   browser-download path.
2. **Login item that opens `Squire.app`** (no signing): add Squire.app to macOS Login Items, so on
   login the app launches and its embedded server runs. Works without signing (the app is ad-hoc
   signed + unquarantined, like a double-click). Needs window-lifecycle handling so closing the window
   keeps the server running in the background (intercept close → hide; reopen from the dock), or
   `LSUIElement` to run as a windowless agent. Caveat: stops at logout, and quitting stops the server.
3. **Accept manual**: the operator opens the app (or runs the binary) when they want the server up.
   Phones backgrounded-update already works via [[SQUIRE-T-0090]].

## Recommendation

If distribution-to-others is the goal, **(1) signing** is the only true always-on daemon and also
unlocks frictionless installs. As a no-cost interim, **(2) login-item + close-to-background** gets
"starts on login, stays up while logged in" without signing.

## Status

Blocked on a direction decision (signing vs login-item vs manual). launchd-unsigned path proven a
dead end and reverted.
