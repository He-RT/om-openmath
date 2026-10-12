# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: notebook.spec.ts >> real definition edits cascade, errors remain local and command palette runs the active cell
- Location: e2e/notebook.spec.ts:40:1

# Error details

```
Error: expect(locator).toContainText(expected) failed

Locator: locator('.notebook-cell').nth(1).locator('.cell-output')
Expected substring: "4"
Received string:    "E023: 未闭合的括号：需要 )1 MessagesParse::E023 — 未闭合的括号：需要 )< 1 ms"
Timeout: 5000ms

Call log:
  - Expect "toContainText" locator('.notebook-cell').nth(1).locator('.cell-output') with timeout 5000ms
  - waiting for locator('.notebook-cell').nth(1).locator('.cell-output')
    14 × locator resolved to <div aria-label="Result" class="cell-output ">…</div>
       - unexpected value "E023: 未闭合的括号：需要 )1 MessagesParse::E023 — 未闭合的括号：需要 )< 1 ms"

```

```yaml
- alert: "E023: 未闭合的括号：需要 )"
- group:
  - text: 1 Messages
  - code: Parse::E023
  - text: — 未闭合的括号：需要 )
- text: < 1 ms
```

# Test source

```ts
  1   | import { test, expect } from "@playwright/test";
  2   | test("real notebook UI edits/runs Unicode, applies parser fixes and source-only downloads", async ({
  3   |   page,
  4   | }) => {
  5   |   await page.goto("/");
  6   |   await page.getByRole("button", { name: /Quadratic equation/ }).click();
  7   |   await expect(page.locator(".solution-list")).toContainText("-3");
  8   |   const editor = page.getByRole("textbox", {
  9   |     name: "Math input 1",
  10  |     exact: true,
  11  |   });
  12  |   await editor.fill("# 🙂\nsolve(α^2==4, α");
  13  |   await page.getByRole("button", { name: "Quick fix", exact: true }).click();
  14  |   await expect(editor).toContainText("solve(α^2==4, α)");
  15  |   await editor.press("ControlOrMeta+Enter");
  16  |   await expect(page.locator(".solution-list")).toContainText("α");
  17  |   await page
  18  |     .getByRole("textbox", { name: "Notebook title" })
  19  |     .fill("Unicode equations");
  20  |   const downloaded = page.waitForEvent("download");
  21  |   await page.getByRole("button", { name: "Save", exact: true }).first().click();
  22  |   const file = await downloaded;
  23  |   expect(file.suggestedFilename()).toBe("Unicode equations.omnb");
  24  |   const path = await file.path();
  25  |   if (!path) throw new Error("No download");
  26  |   const { readFile } = await import("node:fs/promises");
  27  |   const content = JSON.parse(await readFile(path, "utf8")) as {
  28  |     title: string;
  29  |     cells: { source: string }[];
  30  |     config?: unknown;
  31  |   };
  32  |   expect(content.title).toBe("Unicode equations");
  33  |   expect(content.cells[0]?.source).toContain("α");
  34  |   expect(content.config).toBeUndefined();
  35  |   await editor.press("Shift+Enter");
  36  |   await expect(
  37  |     page.getByRole("textbox", { name: "Math input 2", exact: true }),
  38  |   ).toBeVisible();
  39  | });
  40  | test("real definition edits cascade, errors remain local and command palette runs the active cell", async ({
  41  |   page,
  42  | }) => {
  43  |   await page.goto("/");
  44  |   await page.getByRole("button", { name: "＋ Math", exact: true }).click();
  45  |   await page
  46  |     .getByRole("textbox", { name: "Math input 1", exact: true })
  47  |     .fill("let a=2");
  48  |   await page
  49  |     .getByRole("textbox", { name: "Math input 1", exact: true })
  50  |     .press("Shift+Enter");
  51  |   const second = page.getByRole("textbox", {
  52  |     name: "Math input 2",
  53  |     exact: true,
  54  |   });
  55  |   await second.fill("a+1");
  56  |   await second.press("ControlOrMeta+Enter");
  57  |   await expect(
  58  |     page.locator(".notebook-cell").nth(1).locator(".cell-output"),
  59  |   ).toContainText("3");
  60  |   const first = page.getByRole("textbox", {
  61  |     name: "Math input 1",
  62  |     exact: true,
  63  |   });
  64  |   await first.fill("let a=5");
  65  |   await first.press("ControlOrMeta+Enter");
  66  |   await expect(
  67  |     page.locator(".notebook-cell").nth(1).locator(".cell-output"),
  68  |   ).toContainText("6");
  69  |   await second.fill("solve(");
  70  |   await second.press("ControlOrMeta+Enter");
  71  |   await expect(page.locator(".notebook-cell").nth(1)).toHaveAttribute(
  72  |     "data-status",
  73  |     "Error",
  74  |   );
  75  |   await second.fill("2+2");
  76  |   await second.press("ControlOrMeta+k");
  77  |   await page.getByRole("option", { name: /^Run ⌘/ }).click();
  78  |   await expect(
  79  |     page.locator(".notebook-cell").nth(1).locator(".cell-output"),
> 80  |   ).toContainText("4");
      |     ^ Error: expect(locator).toContainText(expected) failed
  81  | });
  82  | for (const width of [375, 720, 1280])
  83  |   test(`real UI fits ${width}px with readable light/dark controls`, async ({
  84  |     page,
  85  |   }) => {
  86  |     await page.setViewportSize({ width, height: 820 });
  87  |     await page.goto("/");
  88  |     await page.getByRole("button", { name: /Quadratic equation/ }).click();
  89  |     await expect(page.locator(".solution-list")).toBeVisible();
  90  |     await expect(
  91  |       page.getByRole("button", { name: "Run 1", exact: true }),
  92  |     ).toBeEnabled();
  93  |     await page.screenshot({
  94  |       path: `test-results/notebook-${width}-light.png`,
  95  |       fullPage: true,
  96  |     });
  97  |     expect(
  98  |       await page.evaluate(
  99  |         () => document.documentElement.scrollWidth <= innerWidth,
  100 |       ),
  101 |     ).toBe(true);
  102 |     await page
  103 |       .getByRole("button", { name: "Preferences", exact: true })
  104 |       .click();
  105 |     await page.getByLabel("Theme", { exact: true }).selectOption("dark");
  106 |     await page
  107 |       .getByRole("dialog")
  108 |       .getByRole("button", { name: "Close", exact: true })
  109 |       .click();
  110 |     expect(await page.locator("html").getAttribute("data-theme")).toBe("dark");
  111 |     await page.screenshot({
  112 |       path: `test-results/notebook-${width}-dark.png`,
  113 |       fullPage: true,
  114 |     });
  115 |   });
  116 | test("real editor completion, Greek Tab shorthand, Markdown and locale changes remain kernel-backed", async ({
  117 |   page,
  118 | }) => {
  119 |   await page.goto("/");
  120 |   await page.getByRole("button", { name: "＋ Math", exact: true }).click();
  121 |   const editor = page.getByRole("textbox", {
  122 |     name: "Math input 1",
  123 |     exact: true,
  124 |   });
  125 |   await editor.fill("\\alpha");
  126 |   await editor.press("Tab");
  127 |   await expect(editor).toHaveText("α");
  128 |   await editor.fill("sq");
  129 |   await editor.press("Control+Space");
  130 |   await expect(page.locator(".cm-tooltip-autocomplete")).toContainText("sqrt");
  131 |   await editor.press("Escape");
  132 |   await editor.fill("2+2");
  133 |   await editor.press("Alt+Enter");
  134 |   await expect(
  135 |     page.getByRole("textbox", { name: "Math input 2", exact: true }),
  136 |   ).toBeVisible();
  137 |   await page.getByRole("button", { name: "＋ Text", exact: true }).click();
  138 |   await page
  139 |     .getByRole("textbox", { name: "Text input 3", exact: true })
  140 |     .fill("# Reasoning\n\n**Exact** and $x^2$.");
  141 |   await expect(page.getByRole("heading", { name: "Reasoning" })).toBeVisible();
  142 |   await expect(page.locator(".markdown .katex")).toBeVisible();
  143 |   await page.getByRole("button", { name: "Preferences", exact: true }).click();
  144 |   await page.getByLabel("Language", { exact: true }).selectOption("zh-CN");
  145 |   await page
  146 |     .getByRole("dialog")
  147 |     .getByRole("button", { name: "关闭", exact: true })
  148 |     .click();
  149 |   await expect(
  150 |     page.getByRole("button", { name: "全部运行", exact: true }).first(),
  151 |   ).toBeVisible();
  152 | });
  153 | test("UI interruption keeps edits made while the real worker is computing and restores only definitions", async ({
  154 |   page,
  155 | }) => {
  156 |   const errors: string[] = [];
  157 |   page.on("console", (message) => {
  158 |     if (message.type() === "error") errors.push(message.text());
  159 |   });
  160 |   await page.goto("/");
  161 |   await page.getByRole("button", { name: "＋ Math", exact: true }).click();
  162 |   const first = page.getByRole("textbox", {
  163 |     name: "Math input 1",
  164 |     exact: true,
  165 |   });
  166 |   await first.fill("let a=9");
  167 |   await first.press("Shift+Enter");
  168 |   const second = page.getByRole("textbox", {
  169 |     name: "Math input 2",
  170 |     exact: true,
  171 |   });
  172 |   await second.fill("factorial(1000000)");
  173 |   await second.press("ControlOrMeta+Enter");
  174 |   await expect(page.locator(".interrupt")).toBeVisible();
  175 |   await page.getByRole("button", { name: "＋ Text", exact: true }).click();
  176 |   const notes = page.getByRole("textbox", {
  177 |     name: "Text input 3",
  178 |     exact: true,
  179 |   });
  180 |   await notes.fill("first");
```