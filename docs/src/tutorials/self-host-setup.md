# Stand up a home server

This tutorial gets the Squire **home server** running on a computer in your house, so the phones on
your LAN can reach it. Allow about 15 minutes. When you're done, hand off to
[Set up Squire for your family](family-setup.md) to add members and chores.

## What you're installing

The home server (`squire-serve`) is a single Rust binary. It:

- serves the **Keep** (parent admin web UI) and the **Local API** the phones talk to,
- keeps all household data in a durable folder on this machine — nothing leaves your network,
- advertises itself on the LAN (mDNS) so phones can discover it,
- pulls the latest phone app and updates itself in the background (see
  [Delivery & updates](../explanation/delivery-and-updates.md)).

## 1. Install

On the home computer (macOS or Linux), run the one-line installer:

```sh
curl -fsSL https://raw.githubusercontent.com/colliery-io/squire/main/install.sh | sh
```

This fetches the latest signed server build from the public release repo and drops a clickable
launcher in place. (Prefer to run from source? See [Run the home server](../how-to/run-server.md).)

## 2. First run

Start the server. On **first run only**, set the initial admin so you can log into the Keep:

```sh
SQUIRE_ADMIN_NAME="Mom" SQUIRE_ADMIN_SECRET="choose-a-secret" squire-serve
```

On every later start, just run `squire-serve` — it opens the existing household untouched. (You can
also skip the env vars and create the first admin from the Keep's first-run form.)

You should see it print the API and Keep ports and the data directory it's using.

## 3. Open the Keep

Visit `http://localhost:4920` on the same machine and log in with the admin you just set. You're now
ready to [add members and create chores](family-setup.md).

## 4. Keep it running

For day-to-day use you'll want the server to start on its own and stay up. See
[Run the home server](../how-to/run-server.md) for running it as a background service.

## Next steps

- **Use it from outside the house** (a parent's phone on cellular): [Expose it over the internet](../how-to/cloudflare-tunnel.md).
- **Protect the data**: [Back up & restore](../how-to/backup-restore.md).
- **Tune it**: [Server configuration](../reference/configuration.md).
