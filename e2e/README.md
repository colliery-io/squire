# Squire Keep — end-to-end tests (Playwright)

Browser-driven E2E for the **Keep** web UI (SQUIRE-T-0069). Replaces the manual curl / headless-screenshot loop: real clicks, real assertions, screenshots/video/trace on failure.

## What it does
`playwright.config.ts` `webServer` builds + launches the **`squire-home` demo** (wiped + reseeded every start: admin Knight `1`/`demo`, Squire `2` "Gawain", sample quests) on loopback ports (Keep `19920`, api `19080`), waits for `/health`, runs the suite, then tears it down. Uses your **system Chrome** (`channel: "chrome"`) — no browser download.

## Run
```bash
cd e2e
npm install          # first time (downloads @playwright/test only)
npm test             # headless; boots the demo server itself
npm run test:headed  # watch it drive the browser
npm run report       # open the HTML report (incl. failure traces)
```

Step screenshots land in `e2e/screens/`; failure artifacts (screenshot/video/trace) in `e2e/test-results/`.

## Covers
- Login → tabbed shell lands on **Quests**
- **Create** a weekly quest assigned to a specific squire → appears in the list with the right labels
- **Import** from the starter library → quest created
- **Settings**: read the seeded timezone, change it (persists across reload), reject an invalid zone

## Not covered
The **native Android app** can't be driven by Playwright (it's Compose, not web). That needs a Compose UI / screenshot test (Espresso) — a separate harness.
