---
id: 001-keep-admin-ui-embedded-local-web
level: adr
title: "Keep admin UI: embedded local web app, engine-direct, loopback-only"
number: 1
short_code: "SQUIRE-A-0008"
created_at: 2026-06-17T11:08:15.823655+00:00
updated_at: 2026-06-17T11:09:11.412802+00:00
decision_date: 
decision_maker: 
parent: 
archived: false

tags:
  - "#adr"
  - "#phase/decided"


exit_criteria_met: false
initiative_id: NULL
---

# ADR-1: Keep admin UI: embedded local web app, engine-direct, loopback-only

Resolves the open "Admin UI form factor" decision in [[SQUIRE-S-0004]] (Architecture Framing — ADR was TBD).

## Context

The Keep (SQUIRE-S-0004) is the parent/admin app and the household's **only writer**. It must drive the Domain Core **in-process** (`Engine::handle` + `Repository::apply` over the single-writer `Repository`, AR-1) and **never** route its operations through the network API (`api` crate, the LAN trust boundary; FR-ADM4, AR-8). Authoring (define/archive) must not be network-reachable in any role. The spec left the UI form factor open: CLI, TUI, or a local web page served by the same process — to be decided before the Keep is built.

## Decision

The Keep is an **embedded local web application served in-process** by the Keep binary:

- **Single self-contained binary.** The web UI (HTML/CSS/JS) is **embedded into the Keep binary** (e.g. `rust-embed`), so there is no separate asset-server, no node/JS build step in the critical path, and `cargo run -p keep` is the whole app.
- **Loopback-only.** The Keep's admin HTTP server binds to **localhost (127.0.0.1) only** — it is a same-machine admin surface, NOT the LAN. This is what keeps authoring "local-only, never exposed over the network" (NFR-1.1.3 / AR-8) even though the UI is delivered over HTTP.
- **Engine-direct handlers.** The Keep's own HTTP handlers construct a `Command`, call `Engine::handle(&snapshot, cmd, &clock)`, and commit the `Vec<Change>` via `Repository::apply(by, …)` — **in-process, over the shared single-writer store**. They do **not** call the `api` crate's router or make any network request back into the household API. The Keep depends on `domain-core` / `store` / `identity`, **not** on `api` for its command path.
- **Operator identity for audit.** The parent authenticates to the Keep as a specific Knight (reusing the `identity` component); that Knight is stamped as `by`/`actor` on every committed change (A-0005, A-0007).
- **Frontend default.** Server-rendered HTML with light vanilla JS (progressive enhancement; htmx-style form posts are acceptable). No SPA framework / node toolchain unless a later task justifies it. Keyboard-friendly forms and an in-place-actionable review queue are first-class.
- **Co-hosting (deployment note, not mandated here).** Because the Keep is the single writer and holds the `SharedStore`, the same process MAY also host the LAN network `api` (for the phones) on a separate, network-bound listener sharing that one store. The two listeners are distinct (loopback admin vs LAN api); only the admin listener exposes authoring. Exact process topology is a wiring detail for the Keep tasks.

## Alternatives Analysis

| Option | Pros | Cons | Risk Level | Implementation Cost |
|--------|------|------|------------|-------------------|
| Embedded local web app (chosen) | Rich, familiar UI; single binary with embedded assets; reuses axum; engine-direct in-process | Delivered over HTTP, so loopback-binding discipline is load-bearing; must consciously avoid looping through the network API | Medium | M |
| TUI (ratatui) | Pure in-process, no HTTP at all; keyboard-native; trivially local-only | Less familiar UI; richer views (tables, forms, log inspector) are more work to build well | Low | M |
| CLI (subcommands) | Simplest; scriptable | Multi-field authoring forms + a batched review queue are clunky as one-shot commands; poor fit for "fast authoring" | Low | S |

## Rationale

The operator chose the embedded web app for the richest, most familiar admin UX while keeping a single self-contained Rust binary. The engine-direct + loopback-only constraints preserve every invariant the form factor could threaten: the network API is not in the command path (AR-8/FR-ADM4), and binding to localhost keeps authoring off the network. axum is already a dependency, so the local server reuses known infrastructure.

## Consequences

### Positive
- Familiar, fast, keyboard-friendly admin UI; richer views (review queue, event-log inspector, audit surfacing) than a TUI/CLI affords with comparable effort.
- One binary, embedded assets — no separate static host, no node build in the critical path.
- Engine-direct command path keeps the single-writer invariant and the no-network-authoring rule intact.

### Negative
- The UI rides on HTTP, so **loopback-only binding is a load-bearing security control** — it must be enforced and tested (a bind-address assertion), not assumed.
- Two conceptual listeners (loopback admin vs LAN api) if co-hosted — care needed that authoring routes live only on the loopback listener.

### Neutral
- The Keep gains a thin web layer (axum + embedded assets + minimal JS), but no SPA framework or node toolchain by default.
- A future richer frontend (framework/build step) remains possible without revisiting this decision.