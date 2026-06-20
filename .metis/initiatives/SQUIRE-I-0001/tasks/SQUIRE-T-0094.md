---
id: day-1-usage-feedback-squire-knight
level: task
title: "Day-1 usage feedback: Squire/Knight UI fixes (balance, achievements, cards, coins/stars, menu)"
short_code: "SQUIRE-T-0094"
created_at: 2026-06-20T01:14:19.997144+00:00
updated_at: 2026-06-20T01:14:19.997144+00:00
parent: SQUIRE-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: SQUIRE-I-0001
---

# Day-1 usage feedback: Squire/Knight UI fixes

Operator feedback after a day of real use. Grounded against the current code.

## Items

1. **Squire: total coins not visible on top.** The balance `GoldPill` sits in the app-bar title Row
   next to the avatar + title + two text actions (Refresh/Forget) — on a narrow phone it's squeezed
   off. → free the actions area (overflow menu, see #6) so the balance shows.
2. **Unclear if the upgrade is happening when you hit install.** The `UpdateBanner` does show
   "Downloading… %", but the feedback isn't obvious. → clearer states (downloading → opening installer)
   + more prominent affordance.
3. **Achievements: can't see all activated ones** ("best friends" off-screen). Badges render in the
   scrollable list — likely a layout/scroll/height issue. NEEDS a repro detail (child home Badges vs
   Knight admin list).
4. **Can't set achievements/streaks to specific quests.** The DOMAIN already supports `Scope::Quest`
   (`quest_in_scope`); the phone authoring UI only exposes Any/Category. → add a "specific quest"
   scope option in the phone (and confirm the LAN api/SDK exposes it). UI gap, not a domain change.
5. **"Do it" → "Done"** on the Available quest button. (trivial)
6. **Forget button too prominent → overflow (⋮) menu**, and add a **version marker** + **download
   latest APK** action in that menu.
7. **Stars and coins are visually mixed.** One currency (points) is rendered as ★-in-a-gold-coin
   everywhere (balance, quest reward, reward cost, achievement bonus, "N more ★"). Reads ambiguously.
   → DECISION: pick one consistent metaphor (coin vs star) and apply it.
8. **Card/detail modals** for quests / rewards / streaks — tap a card to open a dialog with full
   details. (new component)

## Plan / grouping

- **Now (clear):** #6 app-bar overflow menu (Refresh/Forget/version/download-latest) which also fixes
  #1 (frees space for the balance); #5 label.
- **Polish:** #2 update-banner clarity.
- **Decisions needed:** #7 (coin vs star metaphor); #3 (which screen / repro).
- **Bigger:** #8 detail modals; #4 quest-scoped achievements in the phone authoring UI.

## Status Updates

- Captured + grounded. Starting with the app-bar overflow menu + the "Done" label.
- **Done (5/8), committed + goldens re-recorded:** #1 balance visible (app-bar actions → ⋮ menu);
  #2 update-banner clarity ("Update now" → "Downloading… N%" → "tap Install in the system prompt");
  #5 "Do it!" → "Done"; #6 Forget into the ⋮ menu + "Download latest update" + a version marker;
  #7 one consistent **coins** currency (star-less minted coin + every "★" → "coins").
- **ALL 8 DONE.** Remaining three completed:
  - #8 tap-to-open detail dialogs for quests/rewards/streaks/badges (+ goals).
  - #4 quest-scoped achievements/streaks in the phone authoring (Keep already had it → now at
    parity; config is not phone-only).
  - #3 "🎯 Goals to unlock" on the child home — new `GoalView` + `goals()` projection (active,
    not-yet-earned, non-streak achievements with a kid-friendly description + bonus), regenerated
    `openapi.json` + SDK, child section + detail dialog. Golden updated.
- Rust tests green; Paparazzi goldens re-recorded and verified (the child-home golden shows coins,
  the ⋮ menu, "Done", and the Goals section).
- Next: build the signed APK + publish OTA so all 8 reach the phones.
