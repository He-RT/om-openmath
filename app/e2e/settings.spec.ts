import { test, expect } from "@playwright/test";
function pong() {
  return `data: ${JSON.stringify({ choices: [{ index: 0, delta: { content: "pong" }, finish_reason: "stop" }] })}\n\ndata: [DONE]\n\n`;
}
test("real draft connection test does not save; settings persist metadata and only opt-in credentials across reload", async ({
  page,
}) => {
  let calls = 0;
  await page.route("https://settings-provider.invalid/**", async (route) => {
    calls++;
    expect(route.request().headers().authorization).toBe(
      "Bearer synthetic-settings-key",
    );
    const body = route.request().postDataJSON() as {
      model: string;
      max_tokens: number;
    };
    expect(body.model).toBe("fixture");
    expect(body.max_tokens).toBe(8);
    await route.fulfill({
      status: 200,
      contentType: "text/event-stream",
      body: pong(),
      headers: { "access-control-allow-origin": "*" },
    });
  });
  await page.goto("/");
  await page.getByRole("button", { name: "AI settings", exact: true }).click();
  const dialog = page.getByRole("dialog");
  await expect(
    dialog.getByRole("tab", { name: "AI models", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  await dialog
    .getByLabel("Base URL", { exact: true })
    .fill("https://settings-provider.invalid/v1");
  await dialog.getByLabel("Model", { exact: true }).fill("fixture");
  await dialog
    .getByLabel("API Key", { exact: true })
    .fill("synthetic-settings-key");
  await dialog
    .getByRole("button", { name: "Test connection", exact: true })
    .click();
  await expect(dialog.getByRole("status")).toContainText("pong");
  expect(calls).toBe(1);
  await dialog.getByRole("button", { name: "Close", exact: true }).click();
  await page.getByRole("button", { name: "AI settings", exact: true }).click();
  await expect(page.getByLabel("Base URL", { exact: true })).toHaveValue(
    "https://api.deepseek.com/v1",
  );
  await page
    .getByLabel("Base URL", { exact: true })
    .fill("https://settings-provider.invalid/v1");
  await page.getByLabel("Model", { exact: true }).fill("fixture");
  await page
    .getByLabel("API Key", { exact: true })
    .fill("synthetic-settings-key");
  await page
    .getByRole("button", { name: "Save AI settings", exact: true })
    .click();
  await expect(page.getByRole("status")).toContainText("AI settings saved");
  const stored = await page.evaluate(() =>
    localStorage.getItem("openmath-settings-v1"),
  );
  expect(stored).toContain("settings-provider.invalid");
  expect(stored).not.toContain("synthetic-settings-key");
  await page.reload();
  await page.getByRole("button", { name: "AI settings", exact: true }).click();
  await expect(page.getByLabel("Base URL", { exact: true })).toHaveValue(
    "https://settings-provider.invalid/v1",
  );
  await expect(page.getByLabel("API Key", { exact: true })).toHaveValue("");
  await page
    .getByLabel("API Key", { exact: true })
    .fill("synthetic-settings-key");
  await page
    .getByRole("checkbox", {
      name: "Remember credentials and advanced values in this browser",
      exact: true,
    })
    .check();
  await page
    .getByRole("button", { name: "Save AI settings", exact: true })
    .click();
  await expect(page.getByRole("status")).toContainText("AI settings saved");
  expect(
    await page.evaluate(() => localStorage.getItem("openmath-settings-v1")),
  ).toContain("synthetic-settings-key");
  await page.reload();
  await page.getByRole("button", { name: "AI settings", exact: true }).click();
  await page
    .getByRole("button", { name: "Test connection", exact: true })
    .click();
  await expect(page.getByRole("status")).toContainText("pong");
  await page
    .getByRole("checkbox", {
      name: "Remember credentials and advanced values in this browser",
      exact: true,
    })
    .uncheck();
  await page
    .getByRole("button", { name: "Save AI settings", exact: true })
    .click();
  await expect(page.getByRole("status")).toContainText("AI settings saved");
  expect(
    await page.evaluate(() => localStorage.getItem("openmath-settings-v1")),
  ).not.toContain("synthetic-settings-key");
});
for (const width of [375, 1280])
  test(`profile CRUD, JSON validation, preset examples and feature mapping work at ${width}px`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height: 900 });
    await page.goto("/");
    await page
      .getByRole("button", { name: "AI settings", exact: true })
      .click();
    await page
      .getByRole("button", { name: "Duplicate profile", exact: true })
      .click();
    await expect(page.getByLabel("Profile name", { exact: true })).toHaveValue(
      "deepseek-copy",
    );
    await page
      .getByRole("button", { name: "Delete profile", exact: true })
      .click();
    await page.getByRole("option", { name: "deepseek", exact: true }).click();
    await page.getByLabel("Preset", { exact: true }).selectOption("openai");
    await expect(page.getByLabel("Base URL", { exact: true })).toHaveValue(
      "https://api.openai.com/v1",
    );
    await expect(page.getByLabel("Model", { exact: true })).toHaveValue(
      "gpt-4.1-mini",
    );
    await page
      .getByLabel("Extra headers (JSON)", { exact: true })
      .fill('{"x":');
    await expect(page.getByRole("alert")).toContainText("JSON object");
    await expect(
      page.getByRole("button", { name: "Test connection", exact: true }),
    ).toBeDisabled();
    await page
      .getByLabel("Extra headers (JSON)", { exact: true })
      .fill('{"X-Synthetic":"one"}');
    await page
      .getByRole("tab", { name: "Feature mapping", exact: true })
      .click();
    await page
      .getByLabel("Profile for complete", { exact: true })
      .selectOption("");
    await page
      .getByRole("button", { name: "Save AI settings", exact: true })
      .click();
    await expect(page.getByRole("status")).toContainText("AI settings saved");
    await page.getByRole("tab", { name: "AI models", exact: true }).click();
    await page.screenshot({
      path: `test-results/settings-${width}-light.png`,
      fullPage: true,
    });
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth,
      ),
    ).toBe(true);
    await page.getByRole("tab", { name: "General", exact: true }).click();
    await page.getByLabel("Theme", { exact: true }).selectOption("dark");
    await page.getByLabel("Language", { exact: true }).selectOption("zh-CN");
    await page.getByRole("tab", { name: "AI 模型", exact: true }).click();
    await page.screenshot({
      path: `test-results/settings-${width}-dark.png`,
      fullPage: true,
    });
  });
test("explicit keyless remote profile can be saved and used without disabling its mapped AI features", async ({
  page,
}) => {
  await page.route("https://keyless-provider.invalid/**", async (route) => {
    expect(route.request().headers().authorization).toBeUndefined();
    const request = route.request().postDataJSON() as { max_tokens: number };
    const text =
      request.max_tokens === 8
        ? "pong"
        : JSON.stringify({
            wolfram: "Solve[x==1,x]",
            explanation: "Keyless source proposal",
          });
    await route.fulfill({
      status: 200,
      contentType: "text/event-stream",
      body: `data: ${JSON.stringify({ choices: [{ index: 0, delta: { content: text }, finish_reason: "stop" }] })}\n\ndata: [DONE]\n\n`,
      headers: { "access-control-allow-origin": "*" },
    });
  });
  await page.goto("/");
  await page.getByRole("button", { name: "AI settings", exact: true }).click();
  await page
    .getByLabel("Base URL", { exact: true })
    .fill("https://keyless-provider.invalid/v1");
  await page.getByLabel("Model", { exact: true }).fill("fixture");
  await page
    .getByRole("checkbox", { name: "API key required", exact: true })
    .uncheck();
  await page
    .getByRole("button", { name: "Save AI settings", exact: true })
    .click();
  await expect(page.getByRole("status")).toContainText("AI settings saved");
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Close", exact: true })
    .click();
  await expect(page.locator(".ai-indicator")).toHaveClass(/ai-ready/);
  await page.getByRole("button", { name: "＋ Ask AI", exact: true }).click();
  const input = page.getByRole("textbox", {
    name: "Question input 1",
    exact: true,
  });
  await input.fill("Solve x equals one.");
  await input.press("Enter");
  await page.getByRole("button", { name: "Send to AI", exact: true }).click();
  await expect(page.locator(".suggestion-card")).toContainText(
    "Keyless source proposal",
  );
});
