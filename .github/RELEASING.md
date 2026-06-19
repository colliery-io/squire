# Releasing Squire

Delivery follows **ADR SQUIRE-A-0012**: the source repo (`colliery-io/squire`) stays **private**;
signed artifacts are published to a separate **public** repo (`colliery-io/squire-dist`); home
servers + the install QR pull from there unauthenticated.

## One-time setup

### 1. Create the public distribution repo (artifacts only)

```sh
gh repo create colliery-io/squire-dist --public \
  --description "Squire release artifacts (signed APKs + server binaries). Built from the private source repo."
```

### 2. Set the signing + cross-repo secrets on the SOURCE repo

The release keystore must reach CI to sign the APK. Encode it and store it (and the passwords from
`clients/squire-android/keystore.properties`) as Actions secrets — nothing sensitive is committed.

```sh
# from the repo root
gh secret set SIGNING_KEYSTORE_B64   < <(base64 -i clients/squire-android/keystore/squire-release.jks)
gh secret set SIGNING_STORE_PASSWORD --body '<storePassword from keystore.properties>'
gh secret set SIGNING_KEY_PASSWORD   --body '<keyPassword from keystore.properties>'
gh secret set SIGNING_KEY_ALIAS      --body 'squire'

# A fine-grained PAT with Contents: read+write on colliery-io/squire-dist only, so CI can publish
# the release across repos (the default GITHUB_TOKEN can't write to another repo).
gh secret set DIST_REPO_TOKEN        --body '<PAT with contents:write on squire-dist>'
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
   Release in `squire-dist`. Watch it with `gh run watch` (or the Actions tab).

That's the whole publish — no manual `cp`/manifest editing. Home servers pick up the new APK on their
next pull (see ADR SQUIRE-A-0012, server startup-pull); phones update over LAN via content-hash OTA.

## What's NOT here yet (tracked under the delivery initiative)

- Server startup-pull from `squire-dist` (auto-populate the OTA updates dir + manifest).
- QR "click to install" in the Keep (encodes the LAN APK URL).
- Cross-platform server binaries + server self-update.
- Android background (foreground-service) sync + update polling.
