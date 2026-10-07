# Back up & restore

All of a household's data lives in the server's data directory (`SQUIRE_DATA_DIR`). Squire's store
supports a **single-file per-tenant export/import**, which makes backups and restores
straightforward: snapshot the export somewhere safe, and restore by importing it into a fresh server.

## What to back up

- The **per-tenant export** (the household's data as one file) — the durable thing you care about.
- For a quick belt-and-suspenders copy you can also archive the whole `SQUIRE_DATA_DIR` while the
  server is stopped.

## Manual backup

1. Stop the server (`angreal stop`) so the data is at rest.
2. Export / copy the household data file out of `SQUIRE_DATA_DIR`.
3. Store it off the machine — another disk, or object storage.
4. Start the server again.

## Restore

1. Stand up a server pointed at an empty `SQUIRE_DATA_DIR`.
2. Import the exported household file.
3. Start the server; the household opens with its full history intact (balances and streaks are
   *derived* from the event log, so re-importing the log reconstructs everything).

## Why restores are trustworthy

Squire never stores balances as mutable counters — every number is derived by replaying an
append-only event log ([How Squire works](../explanation/how-squire-works.md)). A restored log
therefore reproduces exactly the same balances, streaks, and history.
