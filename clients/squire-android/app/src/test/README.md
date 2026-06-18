# Android screenshot tests (Paparazzi) — SQUIRE-T-0070

JVM-rendered screenshots of the app's Compose screens — **no emulator, no adb, no server**. Renders
each screen to a PNG so a change can be validated visually in a plain `./gradlew test`.

## Run
```bash
cd clients/squire-android
./gradlew :app:recordPaparazziDebug   # (re)write the golden PNGs under app/src/test/snapshots/images/
./gradlew :app:verifyPaparazziDebug   # fail the build on a pixel diff vs the goldens (CI)
```
The goldens are committed; review them in a PR like any other artifact.

## How it stays decoupled
The screens are **stateless** (they take a UI-state object + callbacks), so the test passes sample
state — no network. The one stateful screen, `QuestAdminScreen`, takes `initialQuests` /
`libraryOverride` seams (null in production → live fetch) so it renders populated.

## The loop
Plan a UI change → implement → `recordPaparazziDebug` → review the PNG. (It already caught a real
`LazyColumn` key collision in `PlayerHomeScreen`.)

## Scope
Covers the **native** screens (Knight review, Squire player home, Manage Quests). The **web Keep**
is covered separately by the Playwright suite in `/e2e` (SQUIRE-T-0069).
