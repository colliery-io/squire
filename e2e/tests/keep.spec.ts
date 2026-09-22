import { test, expect, Page } from "@playwright/test";
import * as fs from "fs";

const SCREENS = "screens";
fs.mkdirSync(SCREENS, { recursive: true });

/** Sign in as the demo admin Knight (UserId 1 / "demo") and wait for the tabbed shell. */
async function login(page: Page) {
  await page.goto("/");
  await page.fill("#login-form input[name=user]", "1");
  await page.fill("#login-form input[name=secret]", "demo");
  await page.click("#login-form button[type=submit]");
  await expect(page.locator("#tabs .tab.active")).toBeVisible();
}

test("login lands on the Quests tab", async ({ page }) => {
  await login(page);
  await expect(page.locator("#tabs .tab.active")).toHaveText("Quests");
  await expect(page.locator("#quests-panel")).toBeVisible();
  await page.screenshot({ path: `${SCREENS}/01-quests-tab.png`, fullPage: true });
});

test("create a weekly quest assigned to a specific squire", async ({ page }) => {
  await login(page);
  await page.fill("#quest-form input[name=title]", "Walk the dog");
  await page.fill("#quest-form input[name=reward]", "8");
  await page.selectOption("#quest-cadence", "Weekly");
  for (const d of ["Mon", "Wed", "Fri"]) {
    await page.check(`#quest-weekly-fields input[value=${d}]`);
  }
  await page.check('input[name=assign][value=some]');
  await page.check('#quest-squires input[value="2"]'); // Gawain
  await page.screenshot({ path: `${SCREENS}/02-quest-form.png`, fullPage: true });

  await page.click("#quest-form button[type=submit]");

  const list = page.locator("#quest-list");
  await expect(list).toContainText("Walk the dog");
  await expect(list).toContainText("Mon/Wed/Fri");
  await expect(list).toContainText("Gawain");
  await page.screenshot({ path: `${SCREENS}/03-quest-created.png`, fullPage: true });
});

test("edit a quest in place — upsert, not a duplicate (SQUIRE-T-0120)", async ({ page }) => {
  await login(page);
  // Create a daily quest.
  await page.fill("#quest-form input[name=title]", "Sweep the porch");
  await page.fill("#quest-form input[name=reward]", "4");
  await page.click("#quest-form button[type=submit]");
  await expect(page.locator("#quest-list li", { hasText: "Sweep the porch" })).toHaveCount(1);

  // Edit it: the form prefills from the existing quest and the submit relabels.
  const row = page.locator("#quest-list li", { hasText: "Sweep the porch" });
  await row.getByRole("button", { name: "Edit" }).click();
  await expect(page.locator("#quest-form button[type=submit]")).toHaveText("Save changes");
  await expect(page.locator("#quest-form input[name=title]")).toHaveValue("Sweep the porch");
  await expect(page.locator("#quest-form input[name=reward]")).toHaveValue("4");

  await page.fill("#quest-form input[name=title]", "Sweep the deck");
  await page.fill("#quest-form input[name=reward]", "9");
  await page.click("#quest-form button[type=submit]");

  // Edited in place: one renamed row, the old title gone, the new reward shown; back to create mode.
  await expect(page.locator("#quest-list li", { hasText: "Sweep the deck" })).toHaveCount(1);
  await expect(page.locator("#quest-list li", { hasText: "Sweep the porch" })).toHaveCount(0);
  await expect(page.locator("#quest-list li", { hasText: "Sweep the deck" })).toContainText("9");
  await expect(page.locator("#quest-form button[type=submit]")).toHaveText("Post to the board");
});

test("an archived quest disappears from the quests page", async ({ page }) => {
  await login(page);
  await page.fill("#quest-form input[name=title]", "Polish the armour");
  await page.fill("#quest-form input[name=reward]", "3");
  await page.click("#quest-form button[type=submit]");
  const row = page.locator("#quest-list li", { hasText: "Polish the armour" });
  await expect(row).toHaveCount(1);

  // The quests page is only what a squire can work towards right now.
  await row.getByRole("button", { name: "Archive" }).click();
  await expect(row).toHaveCount(0);
});

test("import a quest from the starter library", async ({ page }) => {
  await login(page);
  await page.locator("#library summary").click(); // expand the collapsible library
  const row = page.locator("#library-list li", { hasText: "Take out the trash" });
  await row.getByRole("button", { name: "Import" }).click();
  await expect(row.getByRole("button")).toHaveText("Imported ✓");
  await expect(page.locator("#quest-list")).toContainText("Take out the trash");
  await page.screenshot({ path: `${SCREENS}/04-library-import.png`, fullPage: true });
});

test("change the household timezone in Settings (applied live)", async ({ page }) => {
  await login(page);
  await page.locator("#tabs .tab[data-tab=settings]").click();
  await expect(page.locator("#settings-tz")).toBeVisible();
  // Seeded from SQUIRE_TZ in the webServer command.
  await expect(page.locator("#settings-tz")).toHaveValue("America/Detroit");

  await page.fill("#settings-tz-custom", "America/Chicago");
  await page.click("#settings-form button[type=submit]");
  await expect(page.locator("#settings-status")).toContainText("America/Chicago");
  await page.screenshot({ path: `${SCREENS}/05-settings-timezone.png`, fullPage: true });

  // Reloading + revisiting Settings shows the persisted choice (proves the write stuck). The Keep
  // restores the session from its cookie on load (SQUIRE-T-0130), so no second sign-in.
  await page.reload();
  await expect(page.locator("#shell")).toBeVisible();
  await page.locator("#tabs .tab[data-tab=settings]").click();
  await expect(page.locator("#settings-tz")).toHaveValue("America/Chicago");
});

test("invalid timezone is rejected", async ({ page }) => {
  await login(page);
  await page.locator("#tabs .tab[data-tab=settings]").click();
  await page.fill("#settings-tz-custom", "Not/AZone");
  await page.click("#settings-form button[type=submit]");
  await expect(page.locator("#settings-error")).toContainText("valid IANA timezone");
});
