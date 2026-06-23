# Glossary

### The Keep
The parent admin web app that runs on the home computer. It holds the Rust engine, the store, and
the Local API, and is the **single source of truth and the only writer**. Parents author quests,
rewards, and achievements here and review the queue. Reachable at `http://localhost:4920` on the
host.

### The Squire
The phone app in **player** mode, used by a child. Shows today's quests, balance, streaks, and
rewards; submits completion claims and redemption requests. It can read and propose but never
commit, and it works offline.

### The Knight
The same phone app in **parent quick-admin** mode. Approve/reject claims, mark a quest done, redeem
a reward, add funds — from anywhere. Authenticated with a parent credential the Squire app never
holds. Authoring stays keyboard-first in the Keep.

### Home server (`squire-serve`)
The persistent binary that serves the Keep and the Local API and stores all household data on your
machine. See [Run the home server](../how-to/run-server.md).

### Quest
A chore. Has an assignee, a cadence (one-off / Daily / Weekly / Every N days), a reward, and an
optional auto-approve. Completing one creates a **claim**.

### Claim
A child's "I did it" submission against a quest. Enters the review queue (unless the quest
auto-approves); on approval the reward is paid and the value is snapshotted.

### Reward
Something a child spends a balance on. Has a cost, an availability (*Once* / *Repeatable*), and an
optional achievement gate. Spending one creates a **redemption request**.

### Redemption request
A child's "I want that" against a reward (including [cash-out](../how-to/cash-out.md)). Affordability
is checked only at approval; nothing is reserved while it's pending.

### Achievement
A sticky reward for consistency — streaks, totals, or point milestones. Can award bonus points and
unlock gated rewards.

### Hazard
A negative behavior that deducts coins or progress — consequences alongside rewards.

### Currency
Squire tracks balances in more than one currency: **coins** (an in-app score) and **dollars** (real
money). Quests can pay either or both; dollars are settled via [cash-out](../how-to/cash-out.md).

### Adjustment
A direct, parent-only change to a balance. Requires a reason, which is recorded in the log.

### Pairing
The one-time flow that links a phone to a household member and gives it that member's role, via a
QR-encoded one-time code. See [Pair a phone](../how-to/pair-phone.md).

### Event log
The append-only record of everything that happened. Balances, streaks, and due-lists are **derived**
from it, never stored as mutable counters — so every number is explainable by replay. See
[How Squire works](../explanation/how-squire-works.md).
