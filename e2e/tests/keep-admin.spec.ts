import { test, expect, Page } from "@playwright/test";
import * as fs from "fs";

const SCREENS = "screens";
fs.mkdirSync(SCREENS, { recursive: true });

async function login(page: Page, tab: string) {
  await page.goto("/");
  await page.fill("#login-form input[name=user]", "1");
  await page.fill("#login-form input[name=secret]", "demo");
  await page.click("#login-form button[type=submit]");
  await page.locator("#tabs .tab.active").waitFor();
  await page.locator(`#tabs .tab[data-tab=${tab}]`).click();
  // Authoring forms sit behind a "New …" toggle; open it so tests can fill the form directly.
  const compose = page.locator(`section[id^="${tab}"] details.compose`).first();
  if (await compose.count()) await compose.evaluate((d) => { (d as HTMLDetailsElement).open = true; });
}

test("Members tab shows a roster with role badges (A1)", async ({ page }) => {
  await login(page, "members");
  // The demo seeds Arthur (Knight) + Gawain (Squire).
  const arthur = page.locator("#member-list li", { hasText: "Arthur" });
  await expect(arthur.locator(".badge-knight")).toHaveText("Knight");
  const gawain = page.locator("#member-list li", { hasText: "Gawain" });
  await expect(gawain.locator(".badge-squire")).toHaveText("Squire");
  await page.screenshot({ path: `${SCREENS}/admin-01-members.png`, fullPage: true });
});

test("adding a member shows the token as a success notice (A1)", async ({ page }) => {
  await login(page, "members");
  await page.fill("#member-form input[name=display_name]", "Lancelot");
  await page.selectOption("#member-form select[name=role]", "Squire");
  await page.fill("#member-form input[name=initial_secret]", "secret");
  await page.click("#member-form button[type=submit]");
  const notice = page.locator("#member-token.notice-success");
  await expect(notice).toBeVisible();
  await expect(notice.locator("code.tok")).not.toBeEmpty();
  await expect(page.locator("#member-list li", { hasText: "Lancelot" })).toBeVisible();
  await page.screenshot({ path: `${SCREENS}/admin-02-member-token.png`, fullPage: true });
});

test("rename a member in place — same row, role + id kept (SQUIRE-T-0120)", async ({ page }) => {
  await login(page, "members");
  const gawain = page.locator("#member-list li", { hasText: "Gawain" });
  await expect(gawain).toBeVisible();
  // The Rename button prompts for the new name; accept it.
  page.once("dialog", (d) => d.accept("Galahad"));
  await gawain.getByRole("button", { name: "Rename" }).click();

  // Renamed in place: new name present, old gone, still a Squire (role preserved — it's an upsert).
  const galahad = page.locator("#member-list li", { hasText: "Galahad" });
  await expect(galahad).toBeVisible();
  await expect(galahad.locator(".badge-squire")).toHaveText("Squire");
  await expect(page.locator("#member-list li", { hasText: "Gawain" })).toHaveCount(0);

  // Rename back: every spec file shares the one demo server, and later files look for "Gawain".
  page.once("dialog", (d) => d.accept("Gawain"));
  await galahad.getByRole("button", { name: "Rename" }).click();
  await expect(page.locator("#member-list li", { hasText: "Gawain" })).toBeVisible();
});

test("Pair tab renders a framed hand-off card with a QR (A2)", async ({ page }) => {
  await login(page, "pair");
  await page.click("#pair-form button[type=submit]");
  const card = page.locator("#pair-result.pair-card");
  await expect(card).toBeVisible();
  await expect(card.locator("#pair-qr svg")).toBeVisible();
  await expect(card.locator("code.pair-code")).not.toBeEmpty();
  await page.screenshot({ path: `${SCREENS}/admin-03-pair-card.png`, fullPage: true });
});

test("Log tab shows a hint then the event log (A3)", async ({ page }) => {
  await login(page, "log");
  await expect(page.locator("#log-hint")).toBeVisible();
  await expect(page.locator("#log-output")).toBeHidden();
  await page.selectOption("#log-form select[name=scope]", "quest");
  await page.fill("#log-form input[name=id]", "101");
  await page.click("#log-form button[type=submit]");
  await expect(page.locator("#log-output")).toBeVisible();
  await expect(page.locator("#log-hint")).toBeHidden();
  await page.screenshot({ path: `${SCREENS}/admin-04-log.png`, fullPage: true });
});
