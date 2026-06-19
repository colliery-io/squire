# Squire — release artifacts

Public distribution for **Squire**: signed Android APKs and (soon) cross-platform server binaries.

- **Source** lives in a separate **private** repo; only built, signed artifacts are published here
  (see ADR SQUIRE-A-0012).
- **Phone app**: a Squire home server pulls the latest APK from this repo's Releases and serves it on
  your home LAN — install via the QR the Keep shows; in-place upgrades flow from the server.
- Releases are produced automatically by CI on each version tag.

Grab the latest under **[Releases](../../releases)**.