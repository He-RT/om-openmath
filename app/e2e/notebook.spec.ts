import { test, expect } from "@playwright/test";
test("real notebook UI edits/runs Unicode, applies parser fixes and source-only downloads", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByRole("button", { name: /Quadratic equation/ }).click();
  await expect(page.locator(".solution-list")).toContainText("-3");
  const editor = page.getByRole("textbox", {
    name: "Math input 1",
    exact: true,
  });
  await editor.fill("# 🙂\nsolve(α^2==4, α");
  await page.getByRole("button", { name: "Quick fix", exact: true }).click();
  await expect(editor).toContainText("solve(α^2==4, α)");
  await editor.press("ControlOrMeta+Enter");
  await expect(page.locator(".solution-list")).toContainText("α");
  await page
    .getByRole("textbox", { name: "Notebook title" })
    .fill("Unicode equations");
  const downloaded = page.waitForEvent("download");
  await page.getByRole("button", { name: "Save", exact: true }).first().click();
  const file = await downloaded;
  expect(file.suggestedFilename()).toBe("Unicode equations.omnb");
  const path = await file.path();
  if (!path) throw new Error("No download");
  const { readFile } = await import("node:fs/promises");
  const content = JSON.parse(await readFile(path, "utf8")) as {
    title: string;
    cells: { source: string }[];
    config?: unknown;
  };
  expect(content.title).toBe("Unicode equations");
  expect(content.cells[0]?.source).toContain("α");
  expect(content.config).toBeUndefined();
  await editor.press("Shift+Enter");
  await expect(
    page.getByRole("textbox", { name: "Math input 2", exact: true }),
  ).toBeVisible();
});
test("real definition edits cascade, errors remain local and command palette runs the active cell", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByRole("button", { name: "＋ Math", exact: true }).click();
  await page
    .getByRole("textbox", { name: "Math input 1", exact: true })
    .fill("let a=2");
  await page
    .getByRole("textbox", { name: "Math input 1", exact: true })
    .press("Shift+Enter");
  const second = page.getByRole("textbox", {
    name: "Math input 2",
    exact: true,
  });
  await second.fill("a+1");
  await second.press("ControlOrMeta+Enter");
  await expect(
    page.locator(".notebook-cell").nth(1).locator(".cell-output"),
  ).toContainText("3");
  const first = page.getByRole("textbox", {
    name: "Math input 1",
    exact: true,
  });
  await first.fill("let a=5");
  await first.press("ControlOrMeta+Enter");
  await expect(
    page.locator(".notebook-cell").nth(1).locator(".cell-output"),
  ).toContainText("6");
  await second.fill("solve(");
  await second.press("ControlOrMeta+Enter");
  await expect(page.locator(".notebook-cell").nth(1)).toHaveAttribute(
    "data-status",
    "Error",
  );
  await second.fill("2+2");
  await second.press("ControlOrMeta+k");
  await page.getByRole("option", { name: /^Run ⌘/ }).click();
  await expect(
    page.locator(".notebook-cell").nth(1).locator(".cell-output"),
  ).toContainText("4");
});
for (const width of [375, 720, 1280])
  test(`real UI fits ${width}px with readable light/dark controls`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height: 820 });
    await page.goto("/");
    await page.getByRole("button", { name: /Quadratic equation/ }).click();
    await expect(page.locator(".solution-list")).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Run 1", exact: true }),
    ).toBeEnabled();
    await page.screenshot({
      path: `test-results/notebook-${width}-light.png`,
      fullPage: true,
    });
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth,
      ),
    ).toBe(true);
    await page
      .getByRole("button", { name: "Preferences", exact: true })
      .click();
    await page.getByLabel("Theme", { exact: true }).selectOption("dark");
    await page
      .getByRole("dialog")
      .getByRole("button", { name: "Close", exact: true })
      .click();
    expect(await page.locator("html").getAttribute("data-theme")).toBe("dark");
    await page.screenshot({
      path: `test-results/notebook-${width}-dark.png`,
      fullPage: true,
    });
  });
test("real editor completion, Greek Tab shorthand, Markdown and locale changes remain kernel-backed", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByRole("button", { name: "＋ Math", exact: true }).click();
  const editor = page.getByRole("textbox", {
    name: "Math input 1",
    exact: true,
  });
  await editor.fill("\\alpha");
  await editor.press("Tab");
  await expect(editor).toHaveText("α");
  await editor.fill("sq");
  await editor.press("Control+Space");
  await expect(page.locator(".cm-tooltip-autocomplete")).toContainText("sqrt");
  await editor.press("Escape");
  await editor.fill("2+2");
  await editor.press("Alt+Enter");
  await expect(
    page.getByRole("textbox", { name: "Math input 2", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "＋ Text", exact: true }).click();
  await page
    .getByRole("textbox", { name: "Text input 3", exact: true })
    .fill("# Reasoning\n\n**Exact** and $x^2$.");
  await expect(page.getByRole("heading", { name: "Reasoning" })).toBeVisible();
  await expect(page.locator(".markdown .katex")).toBeVisible();
  await page.getByRole("button", { name: "Preferences", exact: true }).click();
  await page.getByLabel("Language", { exact: true }).selectOption("zh-CN");
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "关闭", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "全部运行", exact: true }).first(),
  ).toBeVisible();
});
test("UI interruption keeps edits made while the real worker is computing and restores only definitions", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  await page.goto("/");
  await page.getByRole("button", { name: "＋ Math", exact: true }).click();
  const first = page.getByRole("textbox", {
    name: "Math input 1",
    exact: true,
  });
  await first.fill("let a=9");
  await first.press("Shift+Enter");
  const second = page.getByRole("textbox", {
    name: "Math input 2",
    exact: true,
  });
  await second.fill("factorial(1000000)");
  await second.press("ControlOrMeta+Enter");
  await expect(page.locator(".interrupt")).toBeVisible();
  await page.getByRole("button", { name: "＋ Text", exact: true }).click();
  const notes = page.getByRole("textbox", {
    name: "Text input 3",
    exact: true,
  });
  await notes.fill("first");
  await notes.fill("latest edit survives");
  await page.locator(".interrupt").click();
  await expect(
    page.getByRole("button", { name: "Run 2", exact: true }),
  ).toBeEnabled();
  await expect(notes).toHaveValue("latest edit survives");
  await second.fill("a+1");
  await second.press("ControlOrMeta+Enter");
  await expect(
    page.locator(".notebook-cell").nth(1).locator(".cell-output"),
  ).toContainText("10");
  expect(errors).toEqual([]);
});
