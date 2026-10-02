import { test, expect } from "@playwright/test";
test("real solution cards retain repeated rules, use numeric CAS projection, copy and insert without executing", async ({
  page,
  context,
}) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await page.goto("/");
  await page.getByRole("button", { name: "＋ Math", exact: true }).click();
  const editor = page.getByRole("textbox", {
    name: "Math input 1",
    exact: true,
  });
  await editor.fill("solve((x-1)^2==0,x)");
  await editor.press("ControlOrMeta+Enter");
  await expect(page.locator(".solution-chip")).toHaveCount(1);
  await expect(page.locator(".multiplicity")).toHaveText("double root");
  await page.getByRole("button", { name: "Copy all", exact: true }).click();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(
    "{{x -> 1}, {x -> 1}}",
  );
  await page.getByRole("button", { name: "Substitute…", exact: true }).click();
  await expect(
    page.getByRole("textbox", { name: "Math input 2", exact: true }),
  ).toContainText("expr /. {x->1}");
  await expect(page.locator(".notebook-cell").nth(1)).toHaveAttribute(
    "data-status",
    "Stale",
  );
  await editor.fill("sqrt(2)");
  await editor.press("ControlOrMeta+Enter");
  await editor.press("Escape");
  await page.locator(".expr-view").hover();
  await page.getByRole("button", { name: "Numeric ≈", exact: true }).click();
  await expect(page.locator(".expr-view .formula")).toHaveAttribute(
    "aria-label",
    /1\.4142135623730950488/,
  );
  const index = await page.locator(".execution-count").first().textContent();
  await page.getByRole("button", { name: "Exact", exact: true }).click();
  await expect(page.locator(".expr-view .formula")).toHaveAttribute(
    "aria-label",
    "2^(1/2)",
  );
  await expect(page.locator(".execution-count").first()).toHaveText(
    index ?? "",
  );
  await page.getByRole("button", { name: "Copy Wolfram", exact: true }).click();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(
    "2^(1/2)",
  );
  await page
    .getByRole("button", { name: "Insert into new cell", exact: true })
    .click();
  await expect(
    page.getByRole("textbox", { name: "Math input 2", exact: true }),
  ).toContainText("2^(1/2)");
});
for (const width of [375, 1280])
  test(`actual regions/families/root output fits ${width}px and sampled plot uses real WASM`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height: 900 });
    await page.goto("/");
    await page.getByRole("button", { name: "＋ Math", exact: true }).click();
    const editor = page.getByRole("textbox", {
      name: "Math input 1",
      exact: true,
    });
    await editor.fill("reduce(x^2<=4,x)");
    await editor.press("ControlOrMeta+Enter");
    await expect(
      page.getByRole("img", { name: "Number line x" }),
    ).toBeVisible();
    await expect(page.locator(".endpoint-closed")).toHaveCount(2);
    await editor.press("Escape");
    await page.locator(".solution-heading").click();
    await page.screenshot({
      path: `test-results/region-${width}.png`,
      fullPage: true,
    });
    await editor.fill("solve(sin(x)==1/2,x)");
    await editor.press("ControlOrMeta+Enter");
    await expect(page.locator(".solution-condition").first()).toContainText(
      "k",
    );
    await editor.press("Escape");
    await page.locator(".solution-heading").click();
    await page.screenshot({
      path: `test-results/family-${width}.png`,
      fullPage: true,
    });
    await editor.fill("solve(x^5-x+1==0,x,reals)");
    await editor.press("ControlOrMeta+Enter");
    await expect(page.locator(".root-label").first()).toContainText("Root 1 ≈");
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth,
      ),
    ).toBe(true);
    await editor.press("Escape");
    await page.locator(".solution-heading").click();
    await page.screenshot({
      path: `test-results/root-${width}.png`,
      fullPage: true,
    });
    await editor.fill("solve(x^2==4,x)");
    await editor.press("ControlOrMeta+Enter");
    await page
      .locator(".solution-actions")
      .getByRole("button", { name: "Plot", exact: true })
      .click();
    await expect(page.locator(".sampled-curve path").first()).toBeVisible();
    await expect(page.locator(".solution-point")).toHaveCount(2);
  });
