# Squire — real-device runbook (SQUIRE-T-0050)

How to run Squire for a real household: the computer runs the server; the phones run the apps and
pair to it over your home Wi-Fi. Everything is LAN-only (no cloud).

## 0. One-time setup

**On the computer** (the always-on machine — a laptop/mini-PC on the home Wi-Fi):

```sh
# from the repo root
cargo build --release -p squire-home          # builds squire-serve (and the demo)
```

**Build the signed app** (needs the JDK/Android SDK — see `clients/android-env.sh`). It's **one app**
that shows the child or parent UI based on the role of whoever it's paired as:

```sh
cd clients/squire-android
source ../android-env.sh
./gradlew :app:assembleRelease
# Signed APK: app/build/outputs/apk/release/app-release.apk
```

Sideload the same APK onto every phone (`adb install <apk>`, or copy + tap to install with "unknown
sources" enabled). Pair a child's phone as a Squire → it shows the child home; pair a parent's phone
as a Knight → it shows the parent review home (and can assume any Squire).

## 1. Start the server

```sh
SQUIRE_ADMIN_NAME="<your name>" SQUIRE_ADMIN_SECRET="<a password>" \
  cargo run --release -p squire-home --bin squire-serve
```

- First run **bootstraps** your household + admin (that Knight account). Later runs just **load** it
  (data persists in `~/.local/share/squire` on Linux, `~/Library/Application Support/squire` on
  macOS; override with `SQUIRE_DATA_DIR`). The signing key is persisted, so paired phones keep
  working across restarts.
- The banner prints the address phones pair to — your computer's **LAN IP**, auto-detected:

  ```
  LAN api (phones):        http://10.0.0.232:8080   ← phones pair to this (QR + mDNS)
  ```

  Note that IP. If detection is wrong (multiple interfaces / VPN), set it explicitly:
  `SQUIRE_PAIR_HOST=<computer's LAN IP>`. Ports default to `API_PORT=8080` / `KEEP_PORT=4920`.

> The **Keep** (parent admin UI) is loopback-only: open `http://127.0.0.1:<KEEP_PORT>` in a browser
> **on the computer**. Sign in with the admin name + secret from step 1.

## 2. Add members + pair the phones

In the Keep (on the computer):

1. **Members** → add a Squire (the child) and any other Knights. Each gets an initial secret (not
   used by the phone — pairing replaces it).
2. **Pair** → pick the member → it shows a **QR** (and a text code, valid 30 min, single-use).
3. On that member's phone, open the app → **Scan QR** (point the camera at the screen). It pairs and
   drops into the home screen. If the camera/QR is fussy, tap **Discover** (mDNS) to fill host/port,
   or type host (`10.0.0.232`), port (`8080`), household, and the code by hand, then **Pair**.

Repeat for each phone. "Forget" on a phone clears its pairing to re-pair.

## Pushing an app update (optional)

The server can offer app updates over the LAN so you don't have to re-sideload each phone by hand:

1. Build a new **signed** release APK (bump `versionCode` in `app/build.gradle.kts`, same keystore —
   see `SQUIRE-T-0049`).
2. Put the APK + a `manifest.json` in a directory, and point the server at it with `SQUIRE_APK_DIR`:

   ```
   <SQUIRE_APK_DIR>/
     squire.apk
     manifest.json
   ```
   ```json
   { "squire": { "versionCode": 2, "versionName": "0.2.0", "file": "squire.apk" } }
   ```

   ```sh
   SQUIRE_APK_DIR=/path/to/apks SQUIRE_ADMIN_SECRET=... \
     cargo run --release -p squire-home --bin squire-serve
   ```
3. Each phone shows an **"Update available — vX → Get update"** banner on next launch; tapping it
   downloads the APK in the browser and the system installer takes over (the user confirms — Android
   won't silently install a sideloaded app). The update installs over the old app, **keeping the
   pairing + data** (same signing key, higher `versionCode`). No `SQUIRE_APK_DIR` set ⇒ no banner.

## 3. Use it (the loop)

- **Squire (child):** sees today's quests → **Mark done**; sees rewards → **Redeem**.
- **Knight (parent):** sees the review queue → **Approve/Reject** claims and redemption requests;
  **Add funds**, **Redeem** for a child, or **Mark done** on their behalf.
- Both update on their own within ~5 s (auto-refresh); offline (no Wi-Fi) still shows the last view
  and queues actions, which flush when the phone is back on the network.

## What to check on real hardware (the T-0050 gate)

The emulator can't exercise these — verify on actual phones:

- [ ] **Camera QR scan** pairs both phones (the path the emulator couldn't drive).
- [ ] **Discover (mDNS)** finds the server, or note the failure (some routers block mDNS / isolate
      clients — the QR's host/port is the fallback and should always work).
- [ ] **Reachability:** phones reach the computer's LAN IP:port; the chore→review→reward loop works
      across two real phones.
- [ ] **Offline-first:** turn Wi-Fi off on a phone → it still renders + queues an action → turn
      Wi-Fi back on → the action flushes.

Report anything that breaks; file follow-ups for fixes.

## Troubleshooting

- **Phone can't reach the computer:** confirm both are on the same Wi-Fi (not a guest network), the
  computer's firewall allows the api port, and the QR host is the LAN IP (not `127.0.0.1` /
  `10.0.2.2`, which are loopback/emulator-only). `SQUIRE_PAIR_HOST` overrides it.
- **Discover finds nothing:** many home routers filter mDNS or enable AP/client isolation — use the
  QR or manual host/port. (`SQUIRE_MDNS=off` disables the advert entirely.)
- **Pairing code rejected:** codes are single-use and expire after 30 min — mint a fresh one.
- **Tokens stop working after a server move:** the signing key lives in the data dir; keep
  `SQUIRE_DATA_DIR` stable (or copy `signing.key`) so paired phones keep verifying.

> `cargo run -p squire-home` (no `--bin`) is the **throwaway demo** (wipes + re-seeds a "demo"
> household each run, emulator-oriented). Use `squire-serve` for real data.
