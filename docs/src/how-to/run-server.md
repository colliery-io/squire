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

**From source** (a checkout of `squire`):

```sh
angreal serve            # debug
angreal serve --release  # what production uses
```

> **On macOS the installer already runs the server as a background service** (see the
> [tutorial](../tutorials/self-host-setup.md)). Running `squire-serve` by hand as well will collide
> on the ports — stop the service first (`launchctl bootout gui/$(id -u) ~/Library/LaunchAgents/io.colliery.squire.plist`)
> if you want to run it in the foreground.

It listens on **`API_PORT`** (default `8080`) for the phones and **`KEEP_PORT`** (default `4920`)
for the Keep, and stores data under `SQUIRE_DATA_DIR` (an OS data dir by default). See the
[configuration reference](../reference/configuration.md) for the full list.

A fresh household has no admin until you create one in the Keep's first-run form (you don't set it on
the command line). If you ever need to inject one, see
[Add an admin to a running household](#add-an-admin-to-a-running-household) below.

## Stop it

```sh
angreal stop                 # frees the default ports
angreal stop --ports 8080,4920
```

## Run it always-on

You want the server to start at boot and restart if it exits — otherwise the phones can't sync while
the computer is logged out or asleep.

- **macOS** — **already done for you.** The installer registers a `launchd` LaunchAgent
  (`io.colliery.squire`) that runs the server in the background on login, auto-restarts it within
  ~10s if it exits, and survives reboots. Manage it with:

  ```sh
  launchctl kickstart -k gui/$(id -u)/io.colliery.squire   # restart
  launchctl bootout    gui/$(id -u) ~/Library/LaunchAgents/io.colliery.squire.plist   # stop/remove
  ```

  Logs go to `~/Library/Logs/squire-serve.log`.

- **Linux** — set this up yourself with a `systemd` user (or system) service: point `ExecStart` at
  `~/.local/bin/squire-serve`, set `Restart=on-failure`, and put any `SQUIRE_*` overrides in the
  unit's environment.

## Add an admin to a running household

If you ever get locked out, the escape hatch inserts a Knight admin directly (run with the server
stopped):

```sh
angreal add-admin --name "Dad" --secret "a-secret"
```

## Next

- [Run it in Docker, reachable over Tailscale](docker-tailscale.md) so phones work away from home.
- [Back up & restore](backup-restore.md).
