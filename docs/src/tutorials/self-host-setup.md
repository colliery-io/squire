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

This downloads the latest signed `squire-serve` for your machine into `~/.local/bin`. What happens
next depends on your OS:

- **macOS** — the installer also sets the server up as a **background service** (a `launchd`
  LaunchAgent, `io.colliery.squire`) that starts on login, auto-restarts if it ever exits, and
  survives reboots. **The home server starts running immediately** — you don't launch it by hand. It
  also creates a double-clickable **`Squire.app`** in `~/Applications`, which is simply a shortcut
  that opens the Keep (the background service is what keeps the server running). Skip the service
  with `SQUIRE_NO_SERVICE=1`.
- **Linux** — you get the binary only. Start it with `squire-serve`, and set up your own service to
  keep it running — see [Run the home server](../how-to/run-server.md).

(Prefer to run from source instead? See [Run the home server](../how-to/run-server.md).)

## 2. Open the Keep & create your admin

On macOS the server is already up. Open **`Squire.app`** (or just visit `http://localhost:4920`).
On Linux, run `squire-serve` first, then open that URL.

A fresh install has **no admin yet**. The Keep greets you with a *"First run? Create the admin
Knight"* form — fill it in to create your parent login. (That's the normal path; you don't pass admin
credentials on the command line.)

## 3. You're ready

Logged into the Keep, you can now [add members and create chores](family-setup.md). The server keeps
running in the background (macOS) or under whatever service you set up (Linux), so phones can sync
whenever they're on the network.

## Next steps

- **Use it from outside the house** (a parent's phone on cellular): [Expose it over the internet](../how-to/cloudflare-tunnel.md).
- **Protect the data**: [Back up & restore](../how-to/backup-restore.md).
- **Tune it**: [Server configuration](../reference/configuration.md).
