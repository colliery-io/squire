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
  await page.locator("#tabs .tab[data-tab=review]").click();
}

test("reject a claim with an inline reason (no browser prompt) (V1)", async ({ page }) => {
  await login(page);
  // The demo seeds a pending claim (Gawain · Tidy your room).
  const row = page.locator("#claim-queue li", { hasText: "Tidy your room" });
  await expect(row).toBeVisible();
  await row.locator("button.reject").click();
  // An inline reason editor appears in-place — no prompt() dialog.
  await row.locator("input.reject-reason").fill("Bed wasn't made");
  await page.screenshot({ path: `${SCREENS}/review-01-reject-inline.png`, fullPage: true });
  await row.locator("button.confirm-reject").click();
  // The queue refreshes and the claim is gone.
  await expect(page.locator("#claim-queue li", { hasText: "Tidy your room" })).toHaveCount(0);
});

test("adjust a balance inline with a required reason (V2)", async ({ page }) => {
  await login(page);
  const row = page.locator("#squire-balances li", { hasText: "Gawain" });
  await expect(row).toBeVisible();
  await row.locator("button.adjust").click();
  // Apply is disabled until a reason is typed (the engine 400s a blank reason).
  await expect(row.locator("button.apply-adjust")).toBeDisabled();
  await row.locator("input.adjust-amount").fill("10");
  await row.locator("input.adjust-reason").fill("Helped with dishes");
  await expect(row.locator("button.apply-adjust")).toBeEnabled();
  await page.screenshot({ path: `${SCREENS}/review-02-adjust-inline.png`, fullPage: true });
  await row.locator("button.apply-adjust").click();
  // The balance reflects the adjustment after the queue refreshes.
  await expect(page.locator("#squire-balances li", { hasText: "Gawain" })).toContainText("10 pts");
});

test("approve a claim still works (V3 — no regression)", async ({ page }) => {
  await login(page);
  // A second seeded claim, independent of the one the reject test consumes.
  const row = page.locator("#claim-queue li", { hasText: "Walk the dog" });
  await expect(row).toBeVisible();
  await row.getByRole("button", { name: "Approve" }).click();
  await expect(page.locator("#claim-queue li", { hasText: "Walk the dog" })).toHaveCount(0);
});
