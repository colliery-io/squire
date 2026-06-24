import { test, expect, Page } from "@playwright/test";
import * as fs from "fs";

const SCREENS = "screens";
fs.mkdirSync(SCREENS, { recursive: true });

async function login(page: Page) {
  await page.goto("/");
  await page.fill("#login-form input[name=user]", "1");
  await page.fill("#login-form input[name=secret]", "demo");
  await page.click("#login-form button[type=submit]");
  await page.locator("#tabs .tab.active").waitFor();
  await page.locator("#tabs .tab[data-tab=achievements]").click();
}

test("create a Points-earned achievement", async ({ page }) => {
  await login(page);
  await page.fill("#achievement-form input[name=name]", "My Points Goal");
  await page.selectOption("#ach-criterion", "PointsEarned");
  await page.fill("#ach-points-fields input[name=total]", "100");
  await page.fill("#achievement-form input[name=bonus]", "25");
  await page.click("#achievement-form button[type=submit]");
  await expect(page.locator("#achievement-list")).toContainText("My Points Goal");
  await page.screenshot({ path: `${SCREENS}/ach-01-points.png`, fullPage: true });
});

test("create a Streak achievement scoped to a category (F3 + F4)", async ({ page }) => {
  await login(page);
  await page.fill("#achievement-form input[name=name]", "Tidy Streak");
  await page.selectOption("#ach-criterion", "Streak");
  await page.selectOption("#ach-scope", "Category");
  await page.fill("#ach-scope-category", "Bedroom");
  await page.fill("#ach-streak-fields input[name=length]", "7");
  await page.selectOption("select[name=basis]", "CalendarDays");
  await page.fill("#achievement-form input[name=bonus]", "25");
  await page.screenshot({ path: `${SCREENS}/ach-02-streak-form.png`, fullPage: true });
  await page.click("#achievement-form button[type=submit]");
  await expect(page.locator("#achievement-list")).toContainText("Tidy Streak");
  await page.screenshot({ path: `${SCREENS}/ach-03-streak-created.png`, fullPage: true });
});

test("edit an achievement in place — upsert + criterion round-trip (SQUIRE-T-0120)", async ({ page }) => {
  await login(page);
  // Create a Points-earned achievement.
  await page.fill("#achievement-form input[name=name]", "Saver");
  await page.selectOption("#ach-criterion", "PointsEarned");
  await page.fill("#ach-points-fields input[name=total]", "50");
  await page.fill("#achievement-form input[name=bonus]", "5");
  await page.click("#achievement-form button[type=submit]");
  await expect(page.locator("#achievement-list li", { hasText: "Saver" })).toHaveCount(1);

  // Edit: the form round-trips the criterion (PointsEarned + total) and relabels.
  const row = page.locator("#achievement-list li", { hasText: "Saver" });
  await row.getByRole("button", { name: "Edit" }).click();
  await expect(page.locator("#achievement-form button[type=submit]")).toHaveText("Save changes");
  await expect(page.locator("#ach-criterion")).toHaveValue("PointsEarned");
  await expect(page.locator("#ach-points-fields input[name=total]")).toHaveValue("50");
  await expect(page.locator("#achievement-form input[name=bonus]")).toHaveValue("5");

  await page.fill("#achievement-form input[name=name]", "Penny Pincher");
  await page.fill("#ach-points-fields input[name=total]", "200");
  await page.click("#achievement-form button[type=submit]");

  // Renamed + retuned in place: one row, old name gone (no duplicate), back to create mode.
  await expect(page.locator("#achievement-list li", { hasText: "Penny Pincher" })).toHaveCount(1);
  await expect(page.locator("#achievement-list li", { hasText: "Saver" })).toHaveCount(0);
  await expect(page.locator("#achievement-form button[type=submit]")).toHaveText("Add achievement");
});

test("an empty category scope is rejected with an inline error", async ({ page }) => {
  await login(page);
  await page.fill("#achievement-form input[name=name]", "Bad Achievement");
  await page.selectOption("#ach-criterion", "TotalCompletions");
  await page.selectOption("#ach-scope", "Category");
  // leave the category blank
  await page.fill("#ach-total-fields input[name=count]", "5");
  await page.click("#achievement-form button[type=submit]");
  await expect(page.locator("#achievement-error")).toContainText("category");
  await expect(page.locator("#achievement-list")).not.toContainText("Bad Achievement");
});

test("a newly added quest appears in the achievement scope dropdown (T-0071)", async ({ page }) => {
  await login(page); // lands on Achievements; go author a quest first
  await page.locator("#tabs .tab[data-tab=quests]").click();
  await page.fill("#quest-form input[name=title]", "Brand New Chore");
  await page.click("#quest-form button[type=submit]");
  await expect(page.locator("#quest-list")).toContainText("Brand New Chore");

  // The achievement composer's Quest-scope dropdown must now include it.
  await page.locator("#tabs .tab[data-tab=achievements]").click();
  await page.selectOption("#ach-criterion", "TotalCompletions");
  await page.selectOption("#ach-scope", "Quest");
  await expect(page.locator("#ach-scope-quest")).toContainText("Brand New Chore");
});

test("import from the achievement library and gate a reward (F5)", async ({ page }) => {
  await login(page);
  await page.locator("#ach-library summary").click();
  const row = page.locator("#ach-library-list li", { hasText: "Century Club" });
  await row.getByRole("button", { name: "Import" }).click();
  await expect(row.getByRole("button")).toHaveText("Imported ✓");
  await expect(page.locator("#achievement-list")).toContainText("Century Club");
  await page.screenshot({ path: `${SCREENS}/ach-04-library.png`, fullPage: true });

  // The imported achievement becomes selectable as a reward gate.
  await page.locator("#tabs .tab[data-tab=rewards]").click();
  await expect(page.locator("#item-gate")).toContainText("Century Club");
});
