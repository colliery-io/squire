import { test, expect, Page } from "@playwright/test";
import * as fs from "fs";

const SCREENS = "screens";
fs.mkdirSync(SCREENS, { recursive: true });

/** Open a tab and, if it has an authoring form behind a "New …" toggle, open that too. */
async function openTab(page: Page, tab: string) {
  await page.locator(`#tabs .tab[data-tab=${tab}]`).click();
  const compose = page.locator(`section[id^="${{items: "items"}[tab] ?? tab}"] details.compose`).first();
  if (await compose.count()) await compose.evaluate((d) => { (d as HTMLDetailsElement).open = true; });
}

async function login(page: Page) {
  await page.goto("/");
  await page.fill("#login-form input[name=user]", "1");
  await page.fill("#login-form input[name=secret]", "demo");
  await page.click("#login-form button[type=submit]");
  await page.locator("#tabs .tab.active").waitFor();
  await page.locator("#tabs .tab[data-tab=rewards]").click();
  // The form sits behind a "New …" toggle; open it so tests can fill it directly.
  await page.locator("section[id^=items] details.compose").first().evaluate((d) => { (d as HTMLDetailsElement).open = true; });
}

test("create a reward (R1)", async ({ page }) => {
  await login(page);
  await page.fill("#item-form input[name=name]", "Ice cream trip");
  await page.fill("#item-form input[name=description]", "A scoop of your choice");
  await page.fill("#item-form input[name=cost]", "12");
  await page.selectOption("#item-form select[name=availability]", "Repeatable");
  await page.screenshot({ path: `${SCREENS}/reward-01-form.png`, fullPage: true });
  await page.click("#item-form button[type=submit]");
  await expect(page.locator("#item-list")).toContainText("Ice cream trip");
  await page.screenshot({ path: `${SCREENS}/reward-02-created.png`, fullPage: true });
});

test("a cost below 1 is rejected with an inline error (R1)", async ({ page }) => {
  await login(page);
  await page.fill("#item-form input[name=name]", "Free stuff");
  await page.fill("#item-form input[name=cost]", "0");
  await page.click("#item-form button[type=submit]");
  await expect(page.locator("#item-error")).toContainText("at least 1");
  await expect(page.locator("#item-list")).not.toContainText("Free stuff");
});

test("import from the reward library (R2)", async ({ page }) => {
  await login(page);
  await page.locator("#rewards-library summary").click();
  const row = page.locator("#rewards-library-list li", { hasText: "Movie night pick" });
  await row.getByRole("button", { name: "Import" }).click();
  await expect(row.getByRole("button")).toHaveText("Imported ✓");
  await expect(page.locator("#item-list")).toContainText("Movie night pick");
  await page.screenshot({ path: `${SCREENS}/reward-03-library.png`, fullPage: true });
});

test("archive a reward (R3)", async ({ page }) => {
  await login(page);
  await page.fill("#item-form input[name=name]", "Temporary treat");
  await page.fill("#item-form input[name=cost]", "5");
  await page.click("#item-form button[type=submit]");
  const row = page.locator("#item-list li", { hasText: "Temporary treat" });
  await row.getByRole("button", { name: "Archive" }).click();
  await expect(row).toContainText("archived");
});

test("edit a reward in place — upsert, not a duplicate (SQUIRE-T-0120)", async ({ page }) => {
  await login(page);
  await page.fill("#item-form input[name=name]", "Comic book");
  await page.fill("#item-form input[name=cost]", "8");
  await page.click("#item-form button[type=submit]");
  await expect(page.locator("#item-list li", { hasText: "Comic book" })).toHaveCount(1);

  // Edit: prefilled from the existing reward, submit relabelled.
  const row = page.locator("#item-list li", { hasText: "Comic book" });
  await row.getByRole("button", { name: "Edit" }).click();
  await expect(page.locator("#item-form button[type=submit]")).toHaveText("Save changes");
  await expect(page.locator("#item-form input[name=name]")).toHaveValue("Comic book");
  await expect(page.locator("#item-form input[name=cost]")).toHaveValue("8");

  await page.fill("#item-form input[name=name]", "Graphic novel");
  await page.fill("#item-form input[name=cost]", "15");
  await page.click("#item-form button[type=submit]");

  // Renamed in place: one row, old name gone, new cost shown; submit back to create mode.
  await expect(page.locator("#item-list li", { hasText: "Graphic novel" })).toHaveCount(1);
  await expect(page.locator("#item-list li", { hasText: "Comic book" })).toHaveCount(0);
  await expect(page.locator("#item-list li", { hasText: "Graphic novel" })).toContainText("15");
  await expect(page.locator("#item-form button[type=submit]")).toHaveText("Add reward");
});

test("a newly added achievement appears in the reward gate dropdown (R4)", async ({ page }) => {
  await login(page); // lands on Rewards; go author an achievement first
  await openTab(page, "achievements");
  await page.fill("#achievement-form input[name=name]", "Gatekeeper Goal");
  await page.selectOption("#ach-criterion", "PointsEarned");
  await page.fill("#ach-points-fields input[name=total]", "50");
  await page.fill("#achievement-form input[name=bonus]", "10");
  await page.click("#achievement-form button[type=submit]");
  await expect(page.locator("#achievement-list")).toContainText("Gatekeeper Goal");

  // Back on Rewards, the gate dropdown must include the just-created achievement (no reload).
  await openTab(page, "rewards");
  await expect(page.locator("#item-gate")).toContainText("Gatekeeper Goal");
  await page.screenshot({ path: `${SCREENS}/reward-04-gate.png`, fullPage: true });
});
