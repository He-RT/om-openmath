import { test, expect } from "@playwright/test";
import { workbenchMenu } from "./workbenchMenu";

test("workbench breakpoints retain the actual editor, selection and assistant draft", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/");
  await page.getByRole("button", { name: "＋ Math", exact: true }).click();
  const editor = page.getByRole("textbox", { name: "Math input 1", exact: true });
  await editor.fill("let a=2\na+1");
  await editor.press("ControlOrMeta+Enter");
  await expect(page.locator(".cell-output")).toContainText("3");
  const draft = page.getByRole("textbox", { name: "Message to assistant" });
  await draft.fill("unsent draft");
  await page.getByRole("tab", { name: "Variables", exact: true }).click();
  expect(await page.locator('.inspector-host').evaluate(node=>node.getBoundingClientRect().height)).toBe(await page.locator('.workspace-layout').evaluate(node=>node.getBoundingClientRect().height));
  await page.getByRole("tab", { name: "Assistant", exact: true }).click();
  await expect(draft).toHaveValue("unsent draft");
  const editorNode = await editor.elementHandle(), draftNode = await draft.elementHandle();
  await editor.click(); await editor.press("Home"); await editor.press("Shift+ArrowRight");
  const selection = await page.evaluate(() => window.getSelection()?.toString());
  for (const width of [1000, 390, 1440]) {
    await page.setViewportSize({ width, height: 900 });
    await expect.poll(() => editorNode?.evaluate(node => node.isConnected)).toBe(true);
    expect(await editor.evaluate((node, original) => node === original, editorNode)).toBe(true);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    if (width === 390) {
      await expect(page.locator(".inspector-host")).toHaveJSProperty("open", true);
      expect(await page.locator(".inspector-host").evaluate(node => node.matches(":modal"))).toBe(true);
      const height = await page.locator(".inspector-host").evaluate(node => node.getBoundingClientRect().height);
      expect(height).toBeCloseTo(810, 0);
      await page.keyboard.press("Escape");
    }
    // Native modal focus intentionally moves into the phone sheet. After it closes,
    // the notebook's own selection must still be there when the editor regains focus.
    await editor.press("Shift");
    expect(await page.evaluate(() => window.getSelection()?.toString())).toBe(selection);
  }
  await page.getByRole("button", { name: "Inspector", exact: true }).click();
  await expect(draft).toHaveValue("unsent draft");
  expect(await draft.evaluate((node, original) => node === original, draftNode)).toBe(true);
  await page.getByRole("button", { name: "Outline", exact: true }).click();
  await page.getByRole("button", { name: "Outline", exact: true }).click();
  await page.locator(".outline-row").click();
  await expect(editor).toContainText("let a=2");
});
test("phone launches with one column and opens a real modal inspector or outline", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/");
  await expect(page.getByRole("button", { name: "Assistant", exact: true })).toBeVisible();
  await expect(page.locator(".inspector-host")).not.toBeVisible();
  await page.getByRole("button", { name: "Assistant", exact: true }).click();
  await expect(page.getByRole("dialog", { name: "Inspector", exact: true })).toBeVisible();
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "Outline", exact: true }).click();
  await expect(page.getByRole("dialog", { name: "Outline", exact: true })).toBeVisible();
  await page.keyboard.press("Escape");
  await workbenchMenu(page);
  await page.getByRole("button", { name: "Function catalog", exact: true }).click();
  await expect(page.getByRole("dialog", { name: "Function catalog", exact: true })).toBeVisible();
  await expect(page.locator(".signature")).toContainText("Solve");
});
test("toolbar menus support arrows, skip hidden phone-only entries and restore focus on Escape", async ({ page }) => {
  await page.setViewportSize({width:1440,height:900});await page.goto("/");
  const more=page.getByRole("button",{name:"More",exact:true});
  await more.press("ArrowDown");
  await expect(page.getByRole("button",{name:"New notebook",exact:true})).toBeFocused();
  await page.keyboard.press("ArrowUp");
  await expect(page.getByRole("button",{name:"AI settings",exact:true})).toBeFocused();
  await page.keyboard.press("Home");
  await expect(page.getByRole("button",{name:"New notebook",exact:true})).toBeFocused();
  for(let index=0;index<5;index++) await page.keyboard.press("ArrowDown");
  await expect(page.getByRole("button",{name:"Function catalog",exact:true})).toBeFocused();
  await page.keyboard.press("Escape");await expect(more).toBeFocused();
  await expect(page.locator(".file-menu")).not.toHaveAttribute("open");
});
