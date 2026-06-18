import { test, Page } from "@playwright/test";
import * as fs from "fs";

const SCREENS = "screens";
fs.mkdirSync(SCREENS, { recursive: true });

const TABS = ["quests", "review", "rewards", "achievements", "members", "pair", "log", "settings"];

async function login(page: Page) {
  await page.goto("/");
  await page.fill("#login-form input[name=user]", "1");
  await page.fill("#login-form input[name=secret]", "demo");
  await page.click("#login-form button[type=submit]");
  await page.locator("#tabs .tab.active").waitFor();
}

/** Capture every Keep tab to `screens/tab-<name>.png` — a one-shot "show me current state" gallery. */
test("capture every Keep tab", async ({ page }) => {
  await login(page);
  // The Pair tab is more interesting with a code minted; do that one specially.
  for (const t of TABS) {
    await page.locator(`#tabs .tab[data-tab=${t}]`).click();
    await page.waitForTimeout(350); // let the tab's loader settle
    if (t === "pair") {
      await page.click("#pair-form button[type=submit]").catch(() => {});
      await page.waitForTimeout(400);
    }
    await page.screenshot({ path: `${SCREENS}/tab-${t}.png`, fullPage: true });
  }
});
