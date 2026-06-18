import { defineConfig } from "@playwright/test";

// Ports for the throwaway demo server Playwright boots for the run (high/free; Docker squats 8080).
const KEEP_PORT = 19920;
const API_PORT = 19080;

/**
 * E2E config for the Keep web UI (SQUIRE-T-0069).
 *
 * `webServer` builds + launches the `squire-home` **demo** (wiped + reseeded each start: admin
 * Knight 1/`demo`, Squire 2 "Gawain", sample quests) and waits for the Keep's `/health` before the
 * suite runs — then tears it down. Uses the system Chrome (`channel: "chrome"`) so no browser
 * download is needed. Screenshots/video/trace are captured to `test-results/` on failure; tests also
 * save explicit step screenshots under `screens/`.
 */
export default defineConfig({
  testDir: "./tests",
  outputDir: "./test-results",
  timeout: 30_000,
  expect: { timeout: 10_000 },
  fullyParallel: false,
  workers: 1,
  forbidOnly: !!process.env.CI,
  reporter: [["list"], ["html", { outputFolder: "playwright-report", open: "never" }]],
  use: {
    baseURL: `http://127.0.0.1:${KEEP_PORT}`,
    channel: "chrome",
    headless: true,
    viewport: { width: 1000, height: 1400 },
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
    video: "retain-on-failure",
  },
  webServer: {
    // Build then run the demo from the repo root (one dir up). Deterministic timezone for the
    // Settings test. The demo binds the Keep on KEEP_PORT (loopback) + the api on API_PORT.
    command:
      `bash -c 'cd .. && cargo build -p squire-home --bin squire-home && ` +
      `API_PORT=${API_PORT} KEEP_PORT=${KEEP_PORT} SQUIRE_MDNS=off SQUIRE_TZ=America/Detroit ` +
      `target/debug/squire-home'`,
    url: `http://127.0.0.1:${KEEP_PORT}/health`,
    reuseExistingServer: false,
    timeout: 180_000,
    stdout: "pipe",
    stderr: "pipe",
  },
});
