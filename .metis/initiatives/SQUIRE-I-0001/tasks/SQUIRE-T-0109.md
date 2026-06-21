---
id: bug-adjustreq-currency-rejects
level: task
title: "Bug: AdjustReq currency rejects explicit JSON null — phone coin grants 400"
short_code: "SQUIRE-T-0109"
created_at: 2026-06-21T22:56:50.609810+00:00
updated_at: 2026-06-21T22:56:50.609810+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Bug: AdjustReq currency rejects explicit JSON null — phone coin grants 400

## Symptom (operator, 2026-06-21)
Coin grants from the **phone app** silently failed — the operator's "gopher coins" for the Squire never
saved. `POST /admin/adjust` returned **400**; no `Adjusted` event written.

## Root cause
The Kotlin SDK's `Json` has **`encodeDefaults = true`**, and the generated `AdjustReq.currency` is
`Currency? = null`. So the app serializes **`"currency": null`** on every adjust. The server field was
`#[serde(default = "default_coins")] pub currency: Currency` (non-`Option`). `#[serde(default)]` only
supplies a value for an **absent** field — an **explicit `null`** is handed to `Currency`'s deserializer
and fails → 400 before the handler runs.

Confirmed live (minted Knight token, non-writing probe): `currency:null` → **400**, currency absent →
**404** (deserializes, then squire-not-found). The `null` is the discriminator.

This was introduced with the currency field (SQUIRE-A-0013 / T-0098): `#[serde(default)]` looked
back-compatible but doesn't cover the SDK's explicit-null encoding.

## Fix (committed `b94200a`, NOT yet deployed)
`currency: Option<Currency>` (`#[serde(default)]`) in **both** the api `AdjustReq` (`knight.rs`) and the
Keep `AdjustReq` (`review.rs`); resolve `None ⇒ Currency::Coins` at the use site. `Option` accepts
absent *and* null. Re-froze `crates/api/openapi.json` (currency now nullable). **Server-only fix** — the
SDK already sends `null`, so existing phones (squire-12) start working the moment the server ships this;
**no app update needed.**

## Status
- [x] Root-caused + confirmed live (400 on null).
- [x] Fixed in source (api + keep), openapi re-frozen, adjust + openapi tests green.
- [ ] **Deployed** — deliberately deferred. Deploying = a `squire-serve` rebuild = re-churns the macOS
  Local Network permission ([[SQUIRE-T-0108]]). **Bundle this deploy with T-0108 (stable signing) +
  version-alignment + `allowBackup=false`** as one "make it solid" release, so the permission resets at
  most once more.
- **Interim workaround (given to operator):** grant coins from the **desktop Keep** — its adjust omits
  the `currency` field, so it defaults to Coins and is unaffected.

## Latent same-trap (not yet firing)
`CreateQuestReq.cash` is `Long? = null` in the SDK (same `encodeDefaults` interaction), but the app and
Keep always send a numeric `cash`, so it never sends null today. Apply the same `Option<…>` treatment
when touching authoring next, to be safe.

## Acceptance
- [x] `"currency": null` is accepted (⇒ Coins); absent + `"Coins"`/`"Cash"` still work.
- [ ] Phone coin grant lands an `Adjusted{Coins}` event on prod (verify after the bundled deploy).
- [ ] `cash` null-trap closed (or explicitly deferred with a note).
