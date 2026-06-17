# Squire phone clients

Both phone apps — **the Squire** (child player, [[SQUIRE-S-0005]]) and **the Knight** (parent
quick-admin, [[SQUIRE-S-0006]]) — are native Android (Kotlin / Jetpack Compose / Room, fixed by
AR-6) and talk to the Local API over the LAN. Per **ADR SQUIRE-A-0009** they consume a **generated
Kotlin SDK** rather than hand-written DTOs, so the wire types cannot drift from the Rust server.

## The contract pipeline

```
crates/domain-core (Rust wire DTOs, #[derive(ToSchema)])
        │  utoipa
        ▼
crates/api  ApiDoc ──gen──▶ crates/api/openapi.json   ← frozen, conformance-tested (the contract)
        │  openapi-generator (clients/generate-sdk.sh)
        ▼
clients/squire-sdk/  (Kotlin: SquireApi, KnightApi, ControlApi + models)   ← generated, gitignored
```

- **Source of truth:** `crates/api/openapi.json`, emitted from the Rust types. Regenerate it with
  `cargo run -p api --example gen_openapi` (or `UPDATE_OPENAPI=1 cargo test -p api --test openapi`).
  `cargo test -p api --test openapi` fails if the spec drifts from the server.
- **SDK generation:** `./clients/generate-sdk.sh` — fetches the openapi-generator jar into `tools/`
  (gitignored) and writes the Kotlin SDK to `clients/squire-sdk/` (gitignored; regenerate, don't
  hand-edit). Needs only a JVM (Java 17+).

## Surfaces

| Generated API | Used by | Endpoints |
|---------------|---------|-----------|
| `SquireApi`   | the Squire (S-0005) | `GET /state`, `POST /claims`, `POST /redemption-requests` |
| `KnightApi`   | the Knight (S-0006) | `GET /household-review`, `POST /admin/*` |
| `ControlApi`  | both / pairing      | `POST /register`, `POST /login`, `POST /members` |

## Toolchain

| Phase | Needs | Notes |
|-------|-------|-------|
| Contract (`openapi.json`) | Rust toolchain only | already in the repo; tested by `cargo test` |
| SDK generation | JVM (Java 17+) | `clients/generate-sdk.sh`; jar auto-fetched |
| Android app build | Android SDK + Gradle | not in MacPorts — install Google's command-line tools (see below) |

### Installing the Android toolchain (for the app build, not the SDK)

MacPorts does not package the Android SDK. Use Google's command-line tools:

```sh
# 1. command-line tools (unzip into ~/Library/Android/sdk/cmdline-tools/latest)
#    from https://developer.android.com/studio#command-line-tools-only
# 2. platforms + build-tools + platform-tools, then accept licenses:
sdkmanager "platform-tools" "platforms;android-34" "build-tools;34.0.0"
sdkmanager --licenses
```

With the SDK + JDK 17 (already present) and the Gradle wrapper, the app's JVM unit tests
(`./gradlew testDebugUnitTest`) and a debug compile (`./gradlew assembleDebug`) run without an
emulator; instrumented UI tests need an emulator/device.
