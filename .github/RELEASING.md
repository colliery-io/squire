# Releasing Squire

Delivery follows **ADR SQUIRE-A-0018** (which supersedes the two-repo split of SQUIRE-A-0012): this
repo (`colliery-io/squire`) is **public**, and CI publishes the signed artifacts to **its own**
GitHub Releases; home servers + the install QR pull from there unauthenticated.

> This repo was `colliery-io/squire-core` (private source) until 2026-10-06. The old artifacts-only
> repo was renamed to `colliery-io/squire-releases` and frozen with releases up to v0.7.26; servers
> installed before the move already point at `colliery-io/squire`, so they follow this repo.

## One-time setup

Set the signing secrets on the **source** repo (`squire`, i.e. this repo). The
release keystore must reach CI to sign the APK; encode it and store it (and the passwords from
`clients/squire-android/keystore.properties`) as Actions secrets — nothing sensitive is committed.

```sh
# from the repo root (origin = colliery-io/squire)
gh secret set SIGNING_KEYSTORE_B64   < <(base64 -i clients/squire-android/keystore/squire-release.jks)
gh secret set SIGNING_STORE_PASSWORD --body '<storePassword from keystore.properties>'
gh secret set SIGNING_KEY_PASSWORD   --body '<keyPassword from keystore.properties>'
gh secret set SIGNING_KEY_ALIAS      --body 'squire'
```

Verify: `gh secret list` should show all four. Publishing uses the built-in `GITHUB_TOKEN`; no PAT.

## Cutting a release

1. Bump the version in `clients/squire-android/app/build.gradle.kts` (`versionCode` +1, `versionName`).
   Keep the signer identical to prior releases or installed phones can't update.
2. Commit, then tag and push:

   ```sh
   git tag v0.7.3 && git push origin v0.7.3
   ```

3. The **Release phone APK** workflow builds + signs `squire-<versionCode>.apk` and publishes it to a
   Release on this repo. **Release server binaries** attaches the four server builds to the same
   Release. Watch it with `gh run watch` (or the Actions tab).

That's the whole publish — no manual `cp`/manifest editing. Home servers pick up the new APK on their
next pull (see ADR SQUIRE-A-0012, server startup-pull); phones update over LAN via content-hash OTA.

## Publishing the docs

The public doc site (`docs/`, mdBook) is **decoupled from releases**. Any push to `main` that
touches `docs/**` triggers **Publish docs** (`.github/workflows/docs.yml`), which builds the book and
force-pushes the rendered site to this repo's `gh-pages` branch with the built-in `GITHUB_TOKEN`.
Edit docs and merge to `main`; that's the whole publish.

One-time on this repo: enable **Pages** with source = `gh-pages` branch (root). Refresh
the in-repo screenshots with `angreal docs shots` (re-runs the Keep Playwright gallery and copies the
Android Paparazzi goldens into `docs/src/images/`); build/preview locally with `angreal docs serve`.

