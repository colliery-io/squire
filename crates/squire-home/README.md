# squire-home / squire-serve

One crate, two binaries that host the same topology (ADR SQUIRE-A-0008): one shared
single-writer store + identity, with the loopback **Keep** (parent admin UI) and the LAN
**api** (phone clients) over it, plus a best-effort mDNS advert (`_squire._tcp`, SQUIRE-T-0047).

## `squire-home` — the demo (throwaway)

```
cargo run -p squire-home
```

Wipes a fixed `/tmp/squire-home` dir and **re-seeds a deterministic "demo" household every run**
(admin Knight `1`/`demo`, Squire `2`/`demo`, sample quests + a streak achievement + a gated
reward). For the emulator one-tap flow and manual demos. Not durable — do not use for real data.

Env: `API_PORT` (8080), `KEEP_PORT` (4920), `SQUIRE_MDNS=off`.

## `squire-serve` — the persistent server (real use, SQUIRE-T-0048)

```
SQUIRE_ADMIN_SECRET=<secret> cargo run -p squire-home --bin squire-serve
```

Keeps data across restarts in a **durable OS data dir** and does **register-or-load**: on first
run it bootstraps the first admin Knight from env; afterwards it opens the existing household
**untouched** (no wipe, no demo seed). The HMAC **signing key is generated once and persisted**
(`<data dir>/signing.key`, mode `0600`), so previously paired devices keep verifying after a
restart.

Env:

| Var | Default | Notes |
|-----|---------|-------|
| `SQUIRE_DATA_DIR` | `<OS data dir>/squire` (e.g. `~/.local/share/squire`, `~/Library/Application Support/squire`) | durable; never wiped |
| `SQUIRE_HOUSEHOLD` | `home` | tenant handle |
| `SQUIRE_ADMIN_NAME` | `Admin` | first run only |
| `SQUIRE_ADMIN_SECRET` | — | **required on first run** (don't commit/log it) |
| `API_PORT` / `KEEP_PORT` | `8080` / `4920` | listeners |
| `SQUIRE_SIGNING_KEY` | — | override the persisted key (raw bytes) |
| `SQUIRE_MDNS` | on | `off` disables the mDNS advert |

First run prints the bootstrapped admin id; then open the Keep (`http://127.0.0.1:<KEEP_PORT>`),
sign in as the admin, add members, and pair devices (Keep → Pair a device).
