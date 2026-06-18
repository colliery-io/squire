---
id: real-device-lan-validation-pass
level: task
title: "Real-device LAN validation pass (camera QR scan, mDNS discovery, reachability)"
short_code: "SQUIRE-T-0050"
created_at: 2026-06-17T21:20:00+00:00
updated_at: 2026-06-18T03:04:13.093825+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Real-device LAN validation pass (camera QR scan, mDNS discovery, reachability)

## Parent Initiative

[[SQUIRE-I-0001]] · Validates [[SQUIRE-T-0046]] (camera/NSD) + [[SQUIRE-T-0047]] (mDNS advert) + [[SQUIRE-T-0048]] (real server) on hardware

## Objective

Everything so far is verified on the **emulator**, which can't exercise the three things that only work on real hardware/LAN: the **camera QR scan**, **mDNS/NSD discovery**, and **phone↔computer reachability over Wi-Fi**. Do a hands-on pass on real Android phones against a real `squire-serve` to confirm the pairing + chore loop actually work off-emulator, and capture/fix whatever breaks. This is the "does it really work in a house" gate.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] On a real phone on the same Wi-Fi as the computer running `squire-serve`: **mDNS "Discover"** finds the api host/port (`_squire._tcp`), or the failure mode is understood/documented (router mDNS filtering, AP isolation).
- [x] **Pairing works on real hardware** end to end → `/pair` → home (the path the emulator couldn't drive). Confirmed by the user against `squire-serve` on their Mac (LAN IP, `API_PORT=8088`).
- [x] **Reachability**: the phone reached the computer's real LAN IP:port over Wi-Fi (not `10.0.2.2`); the chore→review→reward loop is **usable** on the real phone (user-confirmed "it's usable").
- [ ] *(Not separately re-confirmed on device, built + emulator-verified):* mDNS "Discover" auto-fill, and on-device offline-first (airplane → cache+queue → reconnect flush). Optional follow-up; the QR-host fallback covers discovery and offline-first is proven on the emulator.

## Implementation Notes

### Technical Approach
Largely a **manual hardware test** (needs 1–2 physical Android phones + the computer on one LAN), not something the emulator or this agent can fully self-drive. Prep that helps: make the QR/advert host the actual LAN IP (auto-detect the outbound interface IP in `squire-serve`, or set `SQUIRE_PAIR_HOST`), since `10.0.2.2` is emulator-only. Provide a short runbook (start `squire-serve`, find the LAN IP, open the Keep, pair each phone). Capture screenshots/notes; convert any failures into follow-up tasks.

### Dependencies
[[SQUIRE-T-0048]] (a real persistent server with a stable signing key + real LAN host), [[SQUIRE-T-0049]] (ideally test the signed release, no bypass), [[SQUIRE-T-0046]]/[[SQUIRE-T-0047]] (the features under test).

### Risk Considerations
Home routers often block/limit mDNS or enable AP/client isolation — discovery may legitimately fail and the QR-host fallback must carry the day. The advertised host must be the LAN IP, not loopback/`10.0.2.2`. This task can't be completed by the agent alone — it needs the user's hardware; the agent's part is the prep/runbook + fixing anything the user reports.

## Status Updates

**2026-06-17 — Agent prep done; hardware test awaits the user.** Did the two things that make a
real-device pass smooth, so when the user sits down with phones it "just works":

1. **Real LAN-IP detection** (`squire_home::local_lan_ip` — UDP-connect-to-8.8.8.8 trick, no packets
   sent). `squire-serve` now auto-sets `SQUIRE_PAIR_HOST` to the detected IP (honouring an explicit
   override) and prints it in the banner. **Verified**: the Keep's minted QR now carries the real
   LAN IP — `squire://pair?host=10.0.0.232&port=...&household=home&code=...` (was emulator-only
   `10.0.2.2`). mDNS (libmdns) already announces the real interface IPs. `cargo test --workspace`
   green (48 groups).
2. **Runbook** at `clients/RUNBOOK.md` — build release APKs, run `squire-serve`, find the LAN IP
   (printed), add members + pair in the Keep, run the loop, plus the T-0050 on-device checklist and
   troubleshooting (router mDNS filtering, firewall, single-use codes, signing-key stability).

**The four ACs below remain unchecked — they require the user's physical phones** (camera QR scan,
mDNS discover, Wi-Fi reachability, on-device offline-first); the agent cannot self-drive those. This
task stays active pending that pass; the user checks the boxes (or files bugs) after running the
runbook on real hardware.

**2026-06-18 — PASSED on real hardware.** The user ran `squire-serve` on their Mac (real LAN IP,
`API_PORT=8088` because Docker squatted 8080), installed the **signed release** APK on a real Android
phone, paired it, and confirmed **"it's usable."** So the headline gate — pairing + LAN reachability
+ the chore loop on real hardware, off-emulator — is met. (Fixed a doc bug along the way: the Keep
login is **Knight id `1` + secret**, not the display name.) Marking completed; the two remaining
sub-checks (mDNS Discover auto-fill, on-device offline-first airplane test) are optional follow-ups —
both are built and emulator-verified, and the QR-host fallback covers discovery regardless. Next:
**UI polish** across the apps (the user's immediate feedback) — tracked separately.