# Delivery & updates

Squire isn't on an app store and has no update server you log into. Instead it uses a simple,
auditable delivery model: **signed artifacts are published as GitHub Releases on the public source
repository, and your home server pulls from there.** This page explains how a new version reaches
your phones.

## One repository

`colliery-io/squire` is public: it holds the source code (engine, apps, this documentation),
and its GitHub Releases hold the signed artifacts: the phone APK and the server binaries. Home
servers and the install QR fetch them **unauthenticated**. This site is served from the same
repository's GitHub Pages.

## How an update flows

```
                build + sign (CI)
squire  ───────────────────────▶  GitHub Release on  colliery-io/squire
 (public)                                        │
                                                 │  pull (no auth)
                                                 ▼
                                         your home server  (squire-serve)
                                                 │  serve over LAN
                                                 ▼
                                           phones update in place
```

1. A release is cut by tagging `squire`; CI builds and **signs** the phone APK and server
   binaries and publishes them to a Release on the same repository.
2. Your **home server** checks that repo's Releases on a schedule and on startup:
   - it **self-updates** its own binary, and
   - it **pulls the latest phone APK** into its updates directory.
3. **Phones update over your LAN** from the home server — no Play Store, no reinstall. Detection is
   by content hash, so a phone only downloads when the bytes actually change.

## What this means for you

- **You don't sideload updates repeatedly.** Install the app once; the server hands out new versions
  over Wi‑Fi and the app updates itself.
- **Signing is stable.** Updates are signed with a consistent key, so installed phones keep
  accepting them (and macOS keeps its Local Network permission across server updates).
- **You can pin or disable it.** `SQUIRE_SELF_UPDATE=off` and `SQUIRE_APK_SYNC=off` turn the
  automatic behavior off; `SQUIRE_DIST_REPO` points at a different release repo. See
  [configuration](../reference/configuration.md).

## Why it's built this way

This is the delivery model recorded in the project's architecture decisions (public Releases,
server-pull + LAN distribution + self-update). It keeps the trust boundary intact
— the same single-writer home server that owns your data is the only thing that distributes code to
your phones.
