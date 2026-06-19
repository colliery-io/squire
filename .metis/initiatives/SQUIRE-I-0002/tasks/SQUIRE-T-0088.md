---
id: qr-click-to-install-the-keep
level: task
title: "QR click-to-install: the Keep renders a QR for the LAN APK download URL"
short_code: "SQUIRE-T-0088"
created_at: 2026-06-19T18:50:43.316344+00:00
updated_at: 2026-06-19T18:55:17.938422+00:00
parent: SQUIRE-I-0002
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0002
---

# QR click-to-install: the Keep renders a QR for the LAN APK download URL

## Parent Initiative

[[SQUIRE-I-0002]] — the new-phone onramp from [[SQUIRE-A-0012]].

## Objective

In the Keep's **Pair** tab, show a QR a new phone scans to **download + install** the current APK over
the home LAN — `http://<pair-host>:<api-port>/app/squire-<N>.apk`. The APK is served by the LAN api
(`GET /app/<file>`, unauthenticated, SQUIRE-T-0051); this just points a QR at it.

## Approach

- Reuse the Keep's server-side QR (the `qrcode` crate already used by `pair.rs`) and `advertised_addr()`
  (the LAN host/port). New Keep route `GET /api/app/install` (Operator-gated) reads
  `SQUIRE_APK_DIR/manifest.json`, confirms the file exists, and returns `{available, version_name,
  version_code, file, url, qr_svg}`.
- Front-end: an "Install on a new phone" card at the top of the Pair panel; `keep.js` loads it when the
  Pair tab opens. `available:false` → a "no build published yet" note.

## Acceptance Criteria

- [x] `GET /api/app/install` returns the LAN URL + QR for the published APK (or `available:false`).
- [x] The Pair tab shows the install QR + URL + version; degrades gracefully with no APK.
- [x] Scanning the QR downloads the APK from the LAN api (`/app/<file>`, verified serving the bytes).

## Status Updates

- Done. Keep `app_install` module + `GET /api/app/install` (Operator-gated) reads the manifest and
  renders a QR of `http://<pair-host>:<api-port>/app/<file>` via the existing `qrcode` crate (shared
  `advertised_addr`). Pair tab shows an "Install on a new phone" card that loads on open and degrades
  to a note when nothing's published. Verified end-to-end: server pulled `squire-6.apk` → endpoint
  returned `available:true` with URL + QR SVG; unauthenticated → 401; all 23 Keep E2E green.