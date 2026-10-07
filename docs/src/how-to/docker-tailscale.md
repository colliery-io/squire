# Run it in Docker, reachable over Tailscale

Run the home server as a Docker Compose stack and, optionally, join it to your
[Tailscale](https://tailscale.com) tailnet so paired phones keep syncing away from home Wi‑Fi — on
cellular, at school, travelling. Traffic is WireGuard-encrypted and only devices signed in to *your*
tailnet can reach the server; nothing is exposed to the public internet and no router port is opened.

This is a **from-source** path (a checkout of `squire` with Docker + Compose v2). It is private
remote access for your own household.

## What ends up reachable from where

| Surface | This computer | Home LAN | Your tailnet |
|---------|:---:|:---:|:---:|
| Phone API (`SQUIRE_API_PORT`) | ✓ | ✓ (unless `SQUIRE_BIND=127.0.0.1`) | ✓ (with the `tailscale` profile) |
| The Keep (parent admin UI) | ✓ `http://127.0.0.1:4920` | ✗ | ✗ (unless you opt in with `KEEP_BIND=0.0.0.0`) |

The Keep stays operator-only by construction: inside the container it listens on the Docker bridge
interface only, and that port is published on the host's loopback only. The Tailscale node shares the
server's network namespace, but the Keep has no listener on the tailnet interface.

**How the pieces fit.** Three containers: a do-nothing `net` holder that owns the network namespace,
the published ports and the hostname; the `squire` server; and (optionally) `tailscale`. The server
and Tailscale both *join* the holder rather than each other, so either one can restart, crash or be
redeployed without cutting the other off. The one thing not to do is recreate `net` by itself
(`docker compose up -d --force-recreate net`) — that strands the other two until the next plain
`angreal deploy up`, which heals it.

## Set it up

```sh
cp deploy/.env.example deploy/.env
$EDITOR deploy/.env            # SQUIRE_PAIR_HOST is required — see below
angreal deploy up --build
```

Then open the Keep at <http://127.0.0.1:4920>. On a fresh data volume, create the admin there
("First run? Create the admin Knight").

**`SQUIRE_PAIR_HOST`** is the address baked into every pairing QR. A container cannot detect it, so
you must set it: your computer's LAN IP for home-only use, or the node's tailnet name
(`squire.<your-tailnet>.ts.net`) or `100.x.y.z` address once Tailscale is on. A phone keeps the
address it was paired with — re-pair a phone to move it to the tailnet address.

## Turn on Tailscale

1. In the Tailscale admin console generate an auth key (Settings → Keys). A one-off key is enough —
   the node's state persists in the `tailscale-state` volume.
2. In `deploy/.env` set `TAILSCALE_AUTHKEY=…` and `COMPOSE_PROFILES=tailscale`, then
   `angreal deploy up`.
3. In the admin console, **disable key expiry** for the new `squire` machine.
4. Install Tailscale on each phone, sign in to the same tailnet, set `SQUIRE_PAIR_HOST` to the tailnet
   address, `angreal deploy redeploy`, and pair (or re-pair) the phones.

Check the node with `angreal deploy status` (or `angreal deploy logs -s tailscale`, looking for
"Success").

> **`invalid key … not valid` in the tailscale logs** means the auth key is spent or expired — a
> one-off key works exactly once, so a key that already joined another machine cannot be reused.
> The sidecar restart-loops until it is fixed (the server itself is unaffected): generate a fresh
> key, put it in `deploy/.env`, and `angreal deploy up`. Once a node has joined, its identity lives
> in the `tailscale-state` volume and the key is never needed again.

## Moving an existing install into the container

Your household is two things: `<household>.sqlite` and `signing.key` (every paired phone's token was
minted with it — keep it and the phones stay signed in).

```sh
# 1. Stop the bare-metal server. launchd restarts a merely-killed one, so unload the service —
#    and DISABLE it: `bootout` only lasts until your next login, when `RunAtLoad` would start the
#    old server again, on the OLD data dir, racing the container for the port.
launchctl list | grep -i squire                       # find your label
launchctl bootout gui/$(id -u)/io.colliery.squire     # unload it now
launchctl disable gui/$(id -u)/io.colliery.squire     # …and keep it from coming back
# 2. Copy the data dir into the stack's volume (the source is never modified):
angreal deploy import-data                            # or: --from /path/to/squire-data
# 3. Keep the API port the phones already know (SQUIRE_API_PORT in deploy/.env), then:
angreal deploy up --build
```

Never run the bare-metal server and the container at once — two servers must not share a household.

To go back: `angreal deploy down`, then `launchctl enable gui/$(id -u)/<label>` and
`launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/<label>.plist`. The import copied your data,
so the original directory is exactly as you left it — minus anything done in the container since.

## Day to day

| Task | Command |
|------|---------|
| Ship a server code change | `angreal deploy redeploy` (the image has self-update **off** — this *is* the update path) |
| Forgot a password | `angreal deploy reset-secret` lists members; `--user <id or name>` resets one (same account, new secret; ~10 s restart) |
| Logs | `angreal deploy logs` / `angreal deploy logs -s tailscale` |
| Status + health + tailnet node | `angreal deploy status` |
| Stop (keeps your data) | `angreal deploy down` |

> **Never `docker compose down -v`** on this stack — the `squire-data` volume *is* the household.

mDNS discovery cannot leave a container, so the image ships with it off. If phones are paired by LAN
IP and you want discovery back, run `deploy/advertise-mdns.sh` on the host (it advertises
`_squire._tcp` on the container's behalf).

## See also

- [Server configuration](../reference/configuration.md) — every variable, including `KEEP_BIND`.
- [Back up & restore](backup-restore.md)
