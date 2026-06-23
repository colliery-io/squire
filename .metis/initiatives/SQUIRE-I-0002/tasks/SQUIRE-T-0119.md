---
id: public-doc-site-on-github-pages
level: task
title: "Public doc site on GitHub Pages (mdBook, Diátaxis, cross-repo push to colliery-io/squire)"
short_code: "SQUIRE-T-0119"
created_at: 2026-06-23T17:26:11.210858+00:00
updated_at: 2026-06-23T22:32:27.187494+00:00
parent: SQUIRE-I-0002
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: SQUIRE-I-0002
---

# Public doc site on GitHub Pages (mdBook, Diátaxis, cross-repo push to colliery-io/squire)

## Parent Initiative

[[SQUIRE-I-0002]] — same source-private / artifacts-public split as [[SQUIRE-A-0012]]. The doc
site is one more artifact published to the public dist repo `colliery-io/squire`.

## Objective

Stand up a simple public documentation site for Squire on GitHub Pages, authored in this private
repo (`colliery-io/squire-core`) and cross-pushed to `colliery-io/squire` — reusing the existing
`DIST_REPO_TOKEN` cross-repo path (no new secret). Built with **mdBook**, organized by the
**Diátaxis** framework, covering **both** audiences (families and self-hosters), with **real
screenshots** sourced from the existing Playwright Keep gallery and the Android Paparazzi goldens.

## Design decisions (approved)

- **Generator**: mdBook (Rust-native; no non-Rust toolchain in CI beyond `mdbook` itself).
- **Audience**: both — a landing page that splits into *For families* and *For self-hosters*.
- **Information architecture**: Diátaxis (Tutorials / How-to / Reference / Explanation).
- **Screenshots**: committed into `docs/src/images/` (keeps CI docs build fast). Refreshed locally
  via `angreal docs shots`, which re-runs the Keep Playwright gallery and copies the relevant
  Paparazzi snapshot goldens. Sources:
  - Keep web UI → `e2e/screens/tab-*.png` (Playwright `gallery.spec.ts`).
  - Phone app → `clients/squire-android/app/src/test/snapshots/images/*.png` (Paparazzi; Compose
    can't be Playwright-driven).
- **Publish mechanism**: `.github/workflows/docs.yml` on push to `main` touching `docs/**` builds
  the book and pushes the rendered output to a `gh-pages` branch of `colliery-io/squire` via
  `DIST_REPO_TOKEN`. Decoupled from release tags so docs ship independently of APKs.
- **Pages**: enabled on `colliery-io/squire`, serving `gh-pages` at root.

## Acceptance Criteria

- [x] `docs/` mdBook builds locally (`angreal docs build`) and serves (`angreal docs serve`).
- [x] Diátaxis IA in `src/SUMMARY.md`, both audiences, real screenshots rendered.
- [x] `angreal docs shots` regenerates screenshots into `docs/src/images/`.
- [x] `.github/workflows/docs.yml` builds + cross-pushes to `gh-pages` of `colliery-io/squire`.
- [x] GitHub Pages enabled on `colliery-io/squire`; site reachable.
- [x] `.github/RELEASING.md` notes the docs publish path.

## Implementation Notes

### Reference accuracy
Config/reference page is sourced from `crates/squire-home/src/bin/squire-serve.rs` (env: `API_PORT`
8080, `KEEP_PORT` 4920, `SQUIRE_DATA_DIR`, `SQUIRE_HOUSEHOLD`, `SQUIRE_ADMIN_NAME/SECRET`,
`SQUIRE_APK_DIR`, `SQUIRE_MDNS`, `SQUIRE_SELF_UPDATE`, `SQUIRE_APK_SYNC`, `SQUIRE_DIST_REPO`) and
the install/tunnel/backup tickets ([[SQUIRE-T-0091]], [[SQUIRE-T-0100]], [[SQUIRE-T-0106]]).

### Dependencies
- `DIST_REPO_TOKEN` already exists (contents:write on `colliery-io/squire`) — used by
  `release-apk.yml`. A branch push to `gh-pages` is within that scope; no new secret.

## Status Updates

### 2026-06-23 — built locally, awaiting first publish

**Done (in this private repo):**
- `docs/` mdBook scaffold: `book.toml` (rust/ayu theme, search, fold) + Diátaxis `SUMMARY.md`.
- All 14 content pages written for both audiences (real, accurate prose — server config sourced
  from `squire-serve.rs`; tunnel/backup pages marked *planned* since [[SQUIRE-T-0100]]/[[SQUIRE-T-0106]]
  aren't shipped).
- 11 real screenshots committed to `docs/src/images/` (6 phone Paparazzi goldens + 5 Keep Playwright
  gallery tabs). No orphans; all references resolve.
- `angreal docs {build,serve,shots}` — `shots` re-runs the Keep gallery + copies Paparazzi goldens.
- `.github/workflows/docs.yml` — on push to `main` touching `docs/**`, builds the book and
  force-pushes to `gh-pages` of `colliery-io/squire` via `DIST_REPO_TOKEN` (hand-rolled git push,
  matching `release-apk.yml`; no new secret).
- `.gitignore` excludes `docs/book/`; `RELEASING.md` documents the publish path.
- Verified: `mdbook build docs` clean; rendered site screenshotted and looks correct.

**Shipped:**
1. Committed + pushed to `main` (`95491e0`). The **Publish docs** workflow ran green (13s) and
   created `gh-pages` on `colliery-io/squire`.
2. **Pages enabled** on `colliery-io/squire` (source = `gh-pages`, root). Live and verified:
   **https://colliery-io.github.io/squire/** returns 200 (homepage, subpages, and images).
3. Custom domain — **not doing** (user decision).

All acceptance criteria met. The publish path is now self-sustaining: any future push to `main`
touching `docs/**` rebuilds and redeploys automatically.