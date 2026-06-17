---
id: persistent-squire-serve-binary
level: task
title: "Persistent squire-serve binary: durable data dir, register-or-load (no wipe/reseed)"
short_code: "SQUIRE-T-0048"
created_at: 2026-06-17T21:20:00+00:00
updated_at: 2026-06-17T21:26:25.008968+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Persistent squire-serve binary: durable data dir, register-or-load (no wipe/reseed)

## Parent Initiative

[[SQUIRE-I-0001]] · Spec [[SQUIRE-S-0003]] (Local API) · A-0002 (tenancy) · the production sibling of the `squire-home` demo harness

## Objective

`squire-home` is a **demo harness**: it `remove_dir_all("/tmp/squire-home")` and re-seeds a fixed "demo" household on **every launch**, so nothing survives a restart. That's the single biggest blocker to real use. Ship a **persistent server binary** (`squire-serve`) that keeps data across restarts and does **register-or-load** instead of wipe-and-seed: on first run it bootstraps an **empty** household (or registers one from config/flags) and persists it; on subsequent runs it opens the existing store untouched. Same two surfaces (Keep loopback + LAN api over one shared store + mDNS), but durable and demo-seed-free.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] New binary `squire-serve` (`crates/squire-home/src/bin/squire-serve.rs`) on a **durable, configurable data dir** (`SQUIRE_DATA_DIR`, default `dirs::data_dir()/squire`, NOT `/tmp`) that **never wipes it**.
- [x] **Register-or-load**: `has_admin()` check → if no Knight, bootstrap the first admin from `SQUIRE_ADMIN_NAME`/`SQUIRE_ADMIN_SECRET` (secret required on first run) with **no demo seed**; otherwise open as-is. Verified idempotent: restart shows "loaded", store still has exactly 1 user.
- [x] Hosts the **same two surfaces** (Keep loopback + api `0.0.0.0:API_PORT`) over one shared store/identity + mDNS. **Signing key stable across restarts** (`SQUIRE_SIGNING_KEY` or a generated, persisted `signing.key` at `0600`) — **proven**: a token minted before a restart returned HTTP 200 after it.
- [x] `squire-home` stays the throwaway demo (now clearly labelled); `squire-serve` is the real one. Added `crates/squire-home/README.md` (demo vs serve, env table, first-run admin) + module docs on both bins + `lib.rs`.
- [x] `cargo build`/`cargo test --workspace` green (48 groups, 0 failures). **Manual check passed**: bootstrap admin → author quest "Walk the dog" in the Keep → **restart → quest + admin + pre-restart token all survive**.

## Implementation Notes

### Technical Approach
Factor the shared wiring out of `squire-home/main.rs` (provision/open store, `ProdIdentity::shared_local`, `AppState::new`, `KeepState::from_parts`, `tokio::try_join!` of the two `serve`s + mDNS) into a reusable `run(config)` and give it two entrypoints: the demo (`squire-home`, fixed dir + seed) and `squire-serve` (durable dir, register-or-load, no seed). Persist the HMAC signing key (generate on first run, write `0600` to the data dir) so tokens survive restarts — **critical**, else every restart invalidates paired devices. Use the existing `Provisioner`/`Store` (already durable SQLite) — only the *binary's lifecycle* changes (don't delete the dir; don't re-seed).

### Dependencies
The store/provisioner ([[SQUIRE-T-0011]]/[[SQUIRE-T-0012]]), identity register/login ([[SQUIRE-T-0022]]), mDNS ([[SQUIRE-T-0047]]), and the `squire-home` composition it generalizes.

### Risk Considerations
**Signing-key stability is load-bearing**: a per-run random key (today's demo behavior is a fixed literal) would silently 401 every paired phone after a restart — persist it. Don't auto-seed demo content into a real store. Secret handling for the first-run admin (don't log it; read from env/prompt). Keep the demo binary working so the emulator one-tap flow (and CI/manual demos) don't regress.

## Status Updates

**2026-06-17 — Done.** Factored the shared wiring out of `squire-home/main.rs` into `crates/squire-home/src/lib.rs`: `open_household` (provision-if-absent + open + `ProdIdentity::shared_local` — provisioning is idempotent, so an existing dir opens untouched), `has_admin`, `serve` (both surfaces + mDNS), `signing_key` (env override → persisted `signing.key` generated via `getrandom`, written `0600`), and moved `env_u16`/`start_mdns`. `main.rs` (demo) now wipes `/tmp` + seeds + calls the lib; new `src/bin/squire-serve.rs` is the persistent server (durable `dirs::data_dir()/squire`, register-or-load, no seed). Added `dirs` + `getrandom` deps and `crates/squire-home/README.md`.

**Verified live** (had to dodge Docker squatting :8080/:8081/:8090 — used free high ports): first run "newly bootstrapped" with admin Merlin; authored quest "Walk the dog"; **restart → "loaded", quest survives, the admin's pre-restart token still verifies (HTTP 200) — stable signing key**; store has exactly 1 user (idempotent). `signing.key` is `0600`, `home.sqlite` durable. `cargo test --workspace` green (48 groups). Unblocks [[SQUIRE-T-0049]] (signed release builds) and [[SQUIRE-T-0050]] (real-device pass) — both want a real server to pair against.