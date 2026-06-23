# Releasing Squire

Delivery follows **ADR SQUIRE-A-0012**: the source repo (`colliery-io/squire-core`) stays **private**;
signed artifacts are published to the **public** repo (`colliery-io/squire`); home servers + the
install QR pull from there unauthenticated.

> Repo layout: **`colliery-io/squire-core`** = private source (this repo). **`colliery-io/squire`** =
> public, artifacts only. Both already exist.

## One-time setup

Set the signing + cross-repo secrets on the **source** repo (`squire-core`, i.e. this repo). The
release keystore must reach CI to sign the APK; encode it and store it (and the passwords from
`clients/squire-android/keystore.properties`) as Actions secrets — nothing sensitive is committed.

```sh
# from the repo root (origin = colliery-io/squire-core)
gh secret set SIGNING_KEYSTORE_B64   < <(base64 -i clients/squire-android/keystore/squire-release.jks)
gh secret set SIGNING_STORE_PASSWORD --body '<storePassword from keystore.properties>'
gh secret set SIGNING_KEY_PASSWORD   --body '<keyPassword from keystore.properties>'
gh secret set SIGNING_KEY_ALIAS      --body 'squire'

# A fine-grained PAT with Contents: read+write on colliery-io/squire ONLY, so CI can publish the
# release across repos (the default GITHUB_TOKEN can't write to another repo).
gh secret set DIST_REPO_TOKEN        --body '<PAT with contents:write on colliery-io/squire>'
```

Verify: `gh secret list` should show all five.

## Cutting a release

1. Bump the version in `clients/squire-android/app/build.gradle.kts` (`versionCode` +1, `versionName`).
   Keep the signer identical to prior releases or installed phones can't update.
2. Commit, then tag and push:

   ```sh
   git tag v0.7.3 && git push origin v0.7.3
   ```

3. The **Release phone APK** workflow builds + signs `squire-<versionCode>.apk` and publishes it to a
   Release in `colliery-io/squire` (public). Watch it with `gh run watch` (or the Actions tab).

That's the whole publish — no manual `cp`/manifest editing. Home servers pick up the new APK on their
next pull (see ADR SQUIRE-A-0012, server startup-pull); phones update over LAN via content-hash OTA.

## Publishing the docs

The public doc site (`docs/`, mdBook) is **decoupled from releases**. Any push to `main` that
touches `docs/**` triggers **Publish docs** (`.github/workflows/docs.yml`), which builds the book and
force-pushes the rendered site to the `gh-pages` branch of `colliery-io/squire` using the **same
`DIST_REPO_TOKEN`** — no extra secret. Edit docs and merge to `main`; that's the whole publish.

One-time on `colliery-io/squire`: enable **Pages** with source = `gh-pages` branch (root). Refresh
the in-repo screenshots with `angreal docs shots` (re-runs the Keep Playwright gallery and copies the
Android Paparazzi goldens into `docs/src/images/`); build/preview locally with `angreal docs serve`.

## What's NOT here yet (tracked under SQUIRE-I-0002)

- Server startup-pull from `colliery-io/squire` (auto-populate the OTA updates dir + manifest).
- QR "click to install" in the Keep (encodes the LAN APK URL).
- Cross-platform server binaries + server self-update.
- Android background (foreground-service) sync + update polling.
