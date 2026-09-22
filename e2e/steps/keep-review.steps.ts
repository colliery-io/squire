// Step definitions for the Keep desktop review flows (SQUIRE-T-0114), driving the Keep web UI via
// Playwright. Converted from tests/keep-review.spec.ts; reuses the same selectors and the demo
// server booted by playwright.config.ts (admin Knight 1/`demo`, Squire 2 "Gawain", seeded claims).
import { createBdd } from "playwright-bdd";
import { expect } from "@playwright/test";

const { Given, When, Then } = createBdd();

Given("the operator is signed in to the Keep", async ({ page }) => {
  await page.goto("/");
  await page.fill("#login-form input[name=user]", "1");
  await page.fill("#login-form input[name=secret]", "demo");
  await page.click("#login-form button[type=submit]");
  await page.locator("#tabs .tab.active").waitFor();
});

Given("the Review tab is open", async ({ page }) => {
  await page.locator("#tabs .tab[data-tab=review]").click();
});

When("the operator approves the claim for {string}", async ({ page }, quest: string) => {
  const row = page.locator("#claim-queue li", { hasText: quest });
  await expect(row).toBeVisible();
  await row.getByRole("button", { name: "Seal it" }).click();
});

When(
  "the operator rejects the claim for {string} with reason {string}",
  async ({ page }, quest: string, reason: string) => {
    const row = page.locator("#claim-queue li", { hasText: quest });
    await expect(row).toBeVisible();
    await row.locator("button.reject").click();
    // An inline reason editor appears in-place — no prompt() dialog.
    await row.locator("input.reject-reason").fill(reason);
    await row.locator("button.confirm-reject").click();
  },
);

When(
  "the operator adjusts {string} by {int} with reason {string}",
  async ({ page }, name: string, amount: number, reason: string) => {
    const row = page.locator("#squire-balances li", { hasText: name });
    await expect(row).toBeVisible();
    await row.getByRole("button", { name: "Adjust" }).click();
    // Apply is disabled until a reason is typed (the engine 400s a blank reason).
    await expect(row.locator("button.apply-adjust")).toBeDisabled();
    await row.locator("input.adjust-amount").fill(String(amount));
    await row.locator("input.adjust-reason").fill(reason);
    await expect(row.locator("button.apply-adjust")).toBeEnabled();
    await row.locator("button.apply-adjust").click();
  },
);

Then("the claim for {string} is no longer in the queue", async ({ page }, quest: string) => {
  await expect(page.locator("#claim-queue li", { hasText: quest })).toHaveCount(0);
});

Then("{string}'s coin balance shows {int}", async ({ page }, name: string, amount: number) => {
  await expect(
    page.locator("#squire-balances li", { hasText: name }).locator(".coin .amt"),
  ).toHaveText(String(amount));
});
