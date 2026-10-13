# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: settings.spec.ts >> profile CRUD, JSON validation, preset examples and feature mapping work at 1280px
- Location: e2e/settings.spec.ts:123:3

# Error details

```
Error: expect(locator).toHaveText(expected) failed

Locator: getByRole('status').filter({ hasText: /^AI settings saved$/ })
Expected: "AI settings saved"
Timeout: 5000ms
Error: element(s) not found

Call log:
  - Expect "toHaveText" getByRole('status').filter({ hasText: /^AI settings saved$/ }) with timeout 5000ms
  - waiting for getByRole('status').filter({ hasText: /^AI settings saved$/ })

```

```yaml
- banner:
  - strong: OpenMath
  - text: /
  - textbox "Notebook title":
    - /placeholder: Untitled notebook
  - text: Dialect
  - combobox "Dialect":
    - option "Automatic" [selected]
    - option "Modern"
    - option "Wolfram"
  - button "Run all" [disabled]: ▶ Run all
  - button "Save"
  - button "Command palette": ⌘K
  - button "Preferences": ☷
  - button "AI settings": AI
  - group: ⋯
- main "Untitled notebook":
  - text: OpenMath
  - heading "Start with an expression" [level=1]
  - paragraph: Write mathematics, keep your reasoning together.
  - math: x 2 + 2 x = 3
  - button "Quadratic equation solve(x^2 + 2*x == 3, x) ↗":
    - text: Quadratic equation
    - code: solve(x^2 + 2*x == 3, x)
    - text: ↗
  - button "Linear system solve([x+y==10, x-y==2], [x,y]) ↗":
    - text: Linear system
    - code: solve([x+y==10, x-y==2], [x,y])
    - text: ↗
  - button "Real inequality reduce(x^2 < 4, x, Reals) ↗":
    - text: Real inequality
    - code: reduce(x^2 < 4, x, Reals)
    - text: ↗
  - button "＋ Math"
  - button "＋ Text"
  - button "＋ Ask AI"
  - text: Shift Enter to run · ⌘/Ctrl K for commands Web · WASM
- complementary:
  - navigation:
    - button "Docs" [pressed]
    - button "Variables"
    - button "Steps"
    - button "Assistant"
  - button "Close": ×
  - button "Solve"
  - button "Factor"
  - button "Simplify"
  - button "Expand"
  - button "Reduce"
  - button "N"
  - button "Plot"
  - article:
    - heading "Solve" [level=2]
    - code: Solve[eqs, vars, domain, options]
    - paragraph: Solve equations while preserving source restrictions.
    - button "Solve[x^2==2,x] ↗":
      - code: Solve[x^2==2,x]
      - text: ↗
- dialog "Preferences":
  - banner:
    - heading "Preferences" [level=2]
    - button "Close": ×
  - tablist:
    - tab "General"
    - tab "AI models"
    - tab "Feature mapping" [selected]
  - checkbox "Enable AI features" [checked]
  - text: Enable AI features Translation
  - combobox "Profile for translate":
    - option "Disabled"
    - option "deepseek" [selected]
  - text: Explanation
  - combobox "Profile for explain":
    - option "Disabled"
    - option "deepseek" [selected]
  - text: Assistant
  - combobox "Profile for chat":
    - option "Disabled"
    - option "deepseek" [selected]
  - text: Completion
  - combobox "Profile for complete":
    - option "Disabled" [selected]
    - option "deepseek"
    - option "deepseek-fim"
  - text: Fix with AI
  - combobox "Profile for fix":
    - option "Disabled"
    - option "deepseek" [selected]
  - checkbox "Allow notebook context for completion" [checked]
  - text: Allow notebook context for completion
  - status: AI settings saved
  - button "Save AI settings"
```

# Test source

```ts
  69  |     .getByLabel("API Key", { exact: true })
  70  |     .fill("synthetic-settings-key");
  71  |   await page
  72  |     .getByRole("button", { name: "Save AI settings", exact: true })
  73  |     .click();
  74  |   await expect(page.getByRole("status").filter({ hasText: /^AI settings saved$/ })).toHaveText("AI settings saved");
  75  |   const stored = await page.evaluate(() =>
  76  |     localStorage.getItem("openmath-settings-v1"),
  77  |   );
  78  |   expect(stored).toContain("settings-provider.invalid");
  79  |   expect(stored).not.toContain("synthetic-settings-key");
  80  |   await page.reload();
  81  |   await page.getByRole("button", { name: "AI settings", exact: true }).click();
  82  |   await expect(page.getByLabel("Base URL", { exact: true })).toHaveValue(
  83  |     "https://settings-provider.invalid/v1",
  84  |   );
  85  |   await expect(page.getByLabel("API Key", { exact: true })).toHaveValue("");
  86  |   await page
  87  |     .getByLabel("API Key", { exact: true })
  88  |     .fill("synthetic-settings-key");
  89  |   await page
  90  |     .getByRole("checkbox", {
  91  |       name: "Remember credentials and advanced values in this browser",
  92  |       exact: true,
  93  |     })
  94  |     .check();
  95  |   await page
  96  |     .getByRole("button", { name: "Save AI settings", exact: true })
  97  |     .click();
  98  |   await expect(page.getByRole("status").filter({ hasText: /^AI settings saved$/ })).toHaveText("AI settings saved");
  99  |   expect(
  100 |     await page.evaluate(() => localStorage.getItem("openmath-settings-v1")),
  101 |   ).toContain("synthetic-settings-key");
  102 |   await page.reload();
  103 |   await page.getByRole("button", { name: "AI settings", exact: true }).click();
  104 |   await page
  105 |     .getByRole("button", { name: "Test connection", exact: true })
  106 |     .click();
  107 |   await expect(page.getByRole("status")).toContainText("pong");
  108 |   await page
  109 |     .getByRole("checkbox", {
  110 |       name: "Remember credentials and advanced values in this browser",
  111 |       exact: true,
  112 |     })
  113 |     .uncheck();
  114 |   await page
  115 |     .getByRole("button", { name: "Save AI settings", exact: true })
  116 |     .click();
  117 |   await expect(page.getByRole("status").filter({ hasText: /^AI settings saved$/ })).toHaveText("AI settings saved");
  118 |   expect(
  119 |     await page.evaluate(() => localStorage.getItem("openmath-settings-v1")),
  120 |   ).not.toContain("synthetic-settings-key");
  121 | });
  122 | for (const width of [375, 1280])
  123 |   test(`profile CRUD, JSON validation, preset examples and feature mapping work at ${width}px`, async ({
  124 |     page,
  125 |   }) => {
  126 |     await page.setViewportSize({ width, height: 900 });
  127 |     await page.goto("/");
  128 |     await page
  129 |       .getByRole("button", { name: "AI settings", exact: true })
  130 |       .click();
  131 |     await page
  132 |       .getByRole("button", { name: "Duplicate profile", exact: true })
  133 |       .click();
  134 |     await expect(page.getByLabel("Profile name", { exact: true })).toHaveValue(
  135 |       "deepseek-copy",
  136 |     );
  137 |     await page
  138 |       .getByRole("button", { name: "Delete profile", exact: true })
  139 |       .click();
  140 |     await page.getByRole("option", { name: "deepseek", exact: true }).click();
  141 |     await page.getByLabel("Preset", { exact: true }).selectOption("openai");
  142 |     await expect(page.getByLabel("Base URL", { exact: true })).toHaveValue(
  143 |       "https://api.openai.com/v1",
  144 |     );
  145 |     await expect(page.getByLabel("Model", { exact: true })).toHaveValue(
  146 |       "gpt-4.1-mini",
  147 |     );
  148 |     await page
  149 |       .getByLabel("Extra headers (JSON)", { exact: true })
  150 |       .fill('{"x":');
  151 |     await expect(page.getByRole("alert")).toContainText("JSON object");
  152 |     await expect(
  153 |       page.getByRole("button", { name: "Test connection", exact: true }),
  154 |     ).toBeDisabled();
  155 |     await page
  156 |       .getByLabel("Extra headers (JSON)", { exact: true })
  157 |       .fill('{"X-Synthetic":"one"}');
  158 |     await expect(page.getByRole("alert")).toHaveCount(0);
  159 |     await expect(page.getByRole("button",{name:"Test connection",exact:true})).toBeEnabled();
  160 |     await page
  161 |       .getByRole("tab", { name: "Feature mapping", exact: true })
  162 |       .click();
  163 |     await page
  164 |       .getByLabel("Profile for complete", { exact: true })
  165 |       .selectOption("");
  166 |     await page
  167 |       .getByRole("button", { name: "Save AI settings", exact: true })
  168 |       .click();
> 169 |     await expect(page.getByRole("status").filter({ hasText: /^AI settings saved$/ })).toHaveText("AI settings saved");
      |                                                                                       ^ Error: expect(locator).toHaveText(expected) failed
  170 |     await page.getByRole("tab", { name: "AI models", exact: true }).click();
  171 |     await page.screenshot({
  172 |       path: `test-results/settings-${width}-light.png`,
  173 |       fullPage: true,
  174 |     });
  175 |     expect(
  176 |       await page.evaluate(
  177 |         () => document.documentElement.scrollWidth <= innerWidth,
  178 |       ),
  179 |     ).toBe(true);
  180 |     await page.getByRole("tab", { name: "General", exact: true }).click();
  181 |     await page.getByLabel("Theme", { exact: true }).selectOption("dark");
  182 |     await page.getByLabel("Language", { exact: true }).selectOption("zh-CN");
  183 |     await page.getByRole("tab", { name: "AI 模型", exact: true }).click();
  184 |     await page.screenshot({
  185 |       path: `test-results/settings-${width}-dark.png`,
  186 |       fullPage: true,
  187 |     });
  188 |   });
  189 | test("explicit keyless remote profile can be saved and used without disabling its mapped AI features", async ({
  190 |   page,
  191 | }) => {
  192 |   await page.route("https://keyless-provider.invalid/**", async (route) => {
  193 |     expect(route.request().headers().authorization).toBeUndefined();
  194 |     const request = route.request().postDataJSON() as { max_tokens: number };
  195 |     const text =
  196 |       request.max_tokens === 8
  197 |         ? "pong"
  198 |         : JSON.stringify({
  199 |             wolfram: "Solve[x==1,x]",
  200 |             explanation: "Keyless source proposal",
  201 |           });
  202 |     await route.fulfill({
  203 |       status: 200,
  204 |       contentType: "text/event-stream",
  205 |       body: `data: ${JSON.stringify({ choices: [{ index: 0, delta: { content: text }, finish_reason: "stop" }] })}\n\ndata: [DONE]\n\n`,
  206 |       headers: { "access-control-allow-origin": "*" },
  207 |     });
  208 |   });
  209 |   await page.goto("/");
  210 |   await page.getByRole("button", { name: "AI settings", exact: true }).click();
  211 |   await page
  212 |     .getByLabel("Base URL", { exact: true })
  213 |     .fill("https://keyless-provider.invalid/v1");
  214 |   await page.getByLabel("Model", { exact: true }).fill("fixture");
  215 |   await page
  216 |     .getByRole("checkbox", { name: "API key required", exact: true })
  217 |     .uncheck();
  218 |   await page
  219 |     .getByRole("button", { name: "Save AI settings", exact: true })
  220 |     .click();
  221 |   await expect(page.getByRole("status").filter({ hasText: /^AI settings saved$/ })).toHaveText("AI settings saved");
  222 |   await page
  223 |     .getByRole("dialog")
  224 |     .getByRole("button", { name: "Close", exact: true })
  225 |     .click();
  226 |   await expect(page.locator(".ai-indicator")).toHaveClass(/ai-ready/);
  227 |   await page.getByRole("button", { name: "＋ Ask AI", exact: true }).click();
  228 |   const input = page.getByRole("textbox", {
  229 |     name: "Question input 1",
  230 |     exact: true,
  231 |   });
  232 |   await input.fill("Solve x equals one.");
  233 |   await input.press("Enter");
  234 |   await page.getByRole("button", { name: "Send to AI", exact: true }).click();
  235 |   await expect(page.locator(".suggestion-card")).toContainText(
  236 |     "Keyless source proposal",
  237 |   );
  238 | });
  239 | 
```