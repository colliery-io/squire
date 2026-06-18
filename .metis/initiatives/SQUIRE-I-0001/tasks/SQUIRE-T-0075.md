---
id: member-administration-on-the-phone
level: task
title: "Member administration on the phone (LAN endpoints + native Manage Members + pair-a-device)"
short_code: "SQUIRE-T-0075"
created_at: 2026-06-18T20:33:29.694238+00:00
updated_at: 2026-06-18T20:41:42.601926+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Member administration on the phone (LAN endpoints + native Manage Members + pair-a-device)

## Parent Initiative

[[SQUIRE-I-0001]]

## Objective

Bring household member administration to the native parent app, at parity with the Keep's Members + Pair tabs. A Knight can list members, add a Knight/Squire, de/reactivate, and **pair a member's device** (mint a one-time code and show it as text + a QR) — all from the phone over the LAN api. Closes the last major Keep-only admin surface.

## Current State (map)

- **Identity** is complete: `add_member`, `mint_pairing_code` (hashed, 30-min TTL, plaintext returned once), `consume_pairing_code` (unauth, one-use). `Change::PutUser` / `Change::SetUserActive` persist members (audited by caller).
- **api**: `POST /members` (RequireKnight, add) and `POST /pair/codes` (RequireKnight, mint code → `{code, expires_at}`) already exist as control-plane endpoints. **Missing**: a Knight-gated **list** and **de/reactivate**.
- **Keep**: full Members tab (`GET/POST /api/members`, `POST /api/members/{id}/active`) + Pair tab that renders a QR **SVG server-side** (`qrcode` crate).
- **Phone**: pairing *consumer* exists (ZXing scan + manual entry → `POST /pair` → `Session`). **No member UI. No QR generator** — but ZXing (already a `:pairing` dep) can encode a QR bitmap, and the phone already knows its own reachable host/port/household.

## Define-First: Expected Flows & Outcomes

### Flow M1 — List members
- The screen shows every member (Knights + Squires) with display name, role, and an active/inactive state, fetched from a Knight-gated `GET /admin/members`.

### Flow M2 — Add a member
- **Inputs**: display name (required), role (Squire default | Knight), initial secret (required).
- **Outcome**: `add_member` creates the user (audited to the acting Knight); it appears in the list immediately. A Squire added here auto-joins "all squires" quest assignments (existing `AllSquires` semantics).
- **Validation**: blank name/secret blocked at the form.

### Flow M3 — De/reactivate
- Toggling a member archives/restores them (`SetUserActive`, never deletes; audit preserved). An inactive member can't pair or act. **Guard**: the acting Knight cannot deactivate their own account (no self-lockout).

### Flow M4 — Pair a member's device
- For any active member, "Pair device" mints a one-time code (`POST /admin/members/{id}/pair-code` → `{code, expires_at}`).
- The phone builds the `squire://pair?host=&port=&household=&code=` payload from **its own session** (the LAN address it reached) and shows: the code as selectable text + host/port/household for manual entry, **and** a QR (generated locally via ZXing) the new device can scan.
- The new device pairs via the existing `POST /pair` consumer — no change there.

## Acceptance Criteria

- [x] LAN api adds `RequireKnight` `GET /admin/members` (list) + `POST /admin/members/{id}/active` (de/reactivate) with flat DTOs (`MemberSummaryDto`, `SetActiveReq`). **Add + mint reuse the existing control-plane `POST /members` and `POST /pair/codes`** — both already RequireKnight, so no new endpoints needed there (and the phone's bearer client just calls them via `ControlApi`).
- [x] Set-active rejects self-deactivation (400) and a missing member (404).
- [x] `openapi.json` regenerated; conformance passes; SDK regenerated.
- [x] api integration tests (`crates/api/tests/knight.rs`, +5): list, set-active toggle + missing-404, self-deactivate-400, and the Squire-403 boundary on list + set-active (20 pass total).
- [x] `KnightApiAdapter` gains `listMembers` / `addMember` / `setMemberActive` / `mintPairCode` (the last two add a `ControlApi` instance on the same bearer client).
- [x] A QR generator helper in `:pairing` (`PairQr`, ZXing `BarcodeEncoder`, no new dependency) producing a `Bitmap` from the pair payload (the reverse of `PairTarget.parse`).
- [x] Native `MemberAdminScreen` (Compose): member list with de/reactivate (self-guarded — no toggle on your own account), add-member form (name, role chips, secret), and a "Pair" action that mints a code and shows code text + host/port + QR in a dialog.
- [x] Reachable from `KnightHomeScreen` overflow ⋮ menu ("Manage members"); wired through `KnightHomeHost`.
- [x] Paparazzi snapshot (`knightManageMembers`) renders the list + add form for image validation (re-recorded the stale `knightReviewHome` golden — it predated the ⋮ menu).
- [x] App builds (`:app:assembleDebug` SUCCESSFUL); full `verifyPaparazziDebug` green.

## Implementation Notes

### Technical Approach
- **api** (`crates/api/src/authoring.rs` or a new `members.rs`): RequireKnight handlers. `list` reads `snapshot.users`; `add` delegates to `state.identity.add_member`; `active` applies `Change::SetUserActive` directly to the store (mirror the Keep's `members.rs`), 404 if the user is absent, and reject self-deactivation; `pair-code` delegates to `state.identity.mint_pairing_code`. Reuse the domain `Role` (already codegen-friendly + in the SDK).
- **SDK**: `./gradlew :sdk:openApiGenerate`.
- **Android**: QR helper in `:pairing`; `MemberAdminScreen.kt` in the app; adapter methods; overflow menu + `KnightHomeHost` `managingMembers` state. Payload built from `session.host/port/household` + minted code. Paparazzi seam `initialMembers` to skip the live fetch.

### Dependencies
- Builds on T-0072/0074 (authoring screens pattern) and the existing pairing stack (ADR A-0010, T-0044/0046).

## Status Updates

**2026-06-18 — Done.**
- **api** (`crates/api/src/authoring.rs`): added `GET /admin/members` (reads `snapshot.users` → `MemberSummaryDto`) and `POST /admin/members/{id}/active` (`SetActiveReq`; applies a raw `Change::SetUserActive` audited to the caller; 404 missing; **400 on self-deactivation**). Routes in `lib.rs`; paths+schemas in `openapi.rs`. Add + mint reuse the existing control-plane endpoints.
- `cargo build -p api` SUCCESS; `openapi.json` regenerated; conformance (2) PASS.
- **Decision**: didn't duplicate add/mint under `/admin` — `POST /members` and `POST /pair/codes` are already RequireKnight, so the phone calls them via a `ControlApi` riding the adapter's bearer client. Less surface, no behavior change.
- api integration tests (`tests/knight.rs`, +5): list returns the seeded household; set-active toggles + lists; missing → 404; self-deactivate → 400; Squire token → 403 on list + set-active. `cargo test -p api --test knight` → 20 pass.
- SDK regenerated (`MemberSummaryDto`, `SetActiveReq`; `KnightApi.{listMembers,setMemberActive}`; `ControlApi.{addMember,mintPairCode}` already present).
- **QR generator**: `:pairing/PairQr` — `payload(PairTarget)` builds the `squire://pair?…` URI, `bitmap(content)` encodes a QR via ZXing `BarcodeEncoder` (already on the classpath; no new dep). The phone builds the payload from **its own session** (the LAN address it reached) + the minted code, so a phone-minted code scans identically to a Keep-minted one.
- Android: `KnightApiAdapter` member methods (+`ControlApi`); `MemberAdminScreen.kt` (list + de/reactivate self-guarded, add form, Pair dialog with code + QR); overflow "Manage members"; `KnightHomeHost` `managingMembers` (passes `session.user/host/port/household`). `:app:assembleDebug` SUCCESSFUL.
- Paparazzi `knightManageMembers` recorded + image-validated (self-guard visible: "Arthur (you)" has no Deactivate; inactive Percival shows only Reactivate, no Pair). Re-recorded the stale `knightReviewHome` golden (predated the ⋮ menu). `verifyPaparazziDebug` green.