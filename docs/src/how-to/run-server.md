# Run the home server

This guide covers running `squire-serve` for real: from source, with the right environment, and as
an always-on background service. For first-time install use the
[home-server tutorial](../tutorials/self-host-setup.md); for every knob see
[Server configuration](../reference/configuration.md).

## Run it

**From the installed binary:**

```sh
squire-serve
```

**From source** (a checkout of `squire-core`):

```sh
angreal serve            # debug
angreal serve --release  # what production uses
```

On first run, set the initial admin (once):

```sh
SQUIRE_ADMIN_NAME="Mom" SQUIRE_ADMIN_SECRET="choose-a-secret" squire-serve
```

It listens on **`API_PORT`** (default `8080`) for the phones and **`KEEP_PORT`** (default `4920`)
for the Keep, and stores data under `SQUIRE_DATA_DIR` (an OS data dir by default). See the
[configuration reference](../reference/configuration.md) for the full list.

## Stop it

```sh
angreal stop                 # frees the default ports
angreal stop --ports 8080,4920
```

## Run it always-on

You want the server to start at boot and restart if it exits — otherwise the phones can't sync while
the computer is logged out or asleep.

- **macOS** — a `launchd` LaunchAgent runs it in the background without the app open. (Tracked in
  the project as the always-on background-server work.)
- **Linux** — a `systemd` user (or system) service achieves the same: set `Restart=on-failure` and
  your `SQUIRE_*` environment in the unit file.

Whichever you use, put your first-run admin and any overrides in the service's environment, not on an
interactive command line.

## Add an admin to a running household

If you ever get locked out, the escape hatch inserts a Knight admin directly (run with the server
stopped):

```sh
angreal add-admin --name "Dad" --secret "a-secret"
```

## Next

- [Expose it over the internet](cloudflare-tunnel.md) so phones work away from home.
- [Back up & restore](backup-restore.md).
