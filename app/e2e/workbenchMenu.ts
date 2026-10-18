import type { Page } from "@playwright/test";
export async function workbenchMenu(page: Page) {
  const inspector = page.locator(".inspector-host");
  if (await inspector.evaluate(node => node.matches(":modal"))) await page.keyboard.press("Escape");
  const menu = page.locator(".file-menu");
  if (!(await menu.getAttribute("open") !== null)) await menu.locator("summary").click();
}
