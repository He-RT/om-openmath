import { test, expect } from "@playwright/test";
for (const width of [375, 1280])
  test(`real CAS slider intersections, wheel/pointer pan and crosshair work at ${width}px`, async ({
    page,
  }) => {
    const errors: string[] = [];
    page.on("console", (m) => {
      if (m.type() === "error") errors.push(m.text());
    });
    await page.setViewportSize({ width, height: 900 });
    await page.goto("/");
    if (width < 900)
      await page
        .locator(".inspector")
        .getByRole("button", { name: "Close", exact: true })
        .click();
    await page.getByRole("button", { name: "＋ Math", exact: true }).click();
    const editor = page.getByRole("textbox", {
      name: "Math input 1",
      exact: true,
    });
    await editor.fill("solve(x^2==a,x)");
    await editor.press("ControlOrMeta+Enter");
    await editor.press("Escape");
    await page
      .locator(".solution-actions")
      .getByRole("button", { name: "Plot", exact: true })
      .click();
    const plot = page.locator(".plot-view");
    await expect(plot).toHaveAttribute("aria-busy", "false");
    await expect(plot.locator(".solution-point")).toHaveCount(2);
    const slider = plot.getByRole("slider", { name: "a", exact: true });
    await slider.press("End");
    for (let i = 0; i < 20; i++) await slider.press("ArrowLeft");
    await expect(plot.locator(".plot-slider output")).toHaveText("4");
    await expect(plot).toHaveAttribute("aria-busy", "false");
    await expect(plot.locator(".plot-point-label").first()).toContainText("4)");
    expect(await plot.locator(".plot-point-label").allTextContents()).toEqual([
      "(-2, 4)",
      "(2, 4)",
    ]);
    const svg = plot.getByRole("img", { name: "Plot", exact: true });
    await svg.scrollIntoViewIfNeeded();
    const box = await svg.boundingBox();
    if (!box) throw new Error("plot bounds");
    expect(box.height).toBe(320);
    await page.mouse.move(box.x + box.width * 0.6, box.y + 160);
    await expect(plot.locator(".plot-crosshair line")).toHaveCount(2);
    await expect(plot.locator(".plot-readout")).toContainText("Coordinates: (");
    const before = await plot
      .locator(".plot-curve path")
      .first()
      .getAttribute("d");
    await page.mouse.wheel(0, -180);
    await expect(plot).toHaveAttribute("aria-busy", "false");
    await expect
      .poll(() => plot.locator(".plot-curve path").first().getAttribute("d"))
      .not.toBe(before);
    const zoomed = await plot
      .locator(".plot-curve path")
      .first()
      .getAttribute("d");
    await page.mouse.down();
    await page.mouse.move(box.x + box.width * 0.7, box.y + 190, { steps: 5 });
    await page.mouse.up();
    await expect
      .poll(() => plot.locator(".plot-curve path").first().getAttribute("d"))
      .not.toBe(zoomed);
    await expect(plot).toHaveAttribute("aria-busy", "false");
    await svg.dblclick();
    await expect(plot).toHaveAttribute("aria-busy", "false");
    await expect(slider).toHaveValue("4");
    await page.screenshot({
      path: `test-results/plot-${width}-light.png`,
      fullPage: true,
    });
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth,
      ),
    ).toBe(true);
    await page
      .getByRole("banner")
      .getByRole("button", { name: "Preferences", exact: true })
      .click();
    await page.getByLabel("Theme", { exact: true }).selectOption("dark");
    await page
      .getByRole("dialog")
      .getByRole("button", { name: "Close", exact: true })
      .click();
    await page.screenshot({
      path: `test-results/plot-${width}-dark.png`,
      fullPage: true,
    });
    expect(errors).toEqual([]);
  });
test("explicit plots retain discontinuities and parameter values with no real solutions clear old intersections", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByRole("button", { name: "＋ Math", exact: true }).click();
  const editor = page.getByRole("textbox", {
    name: "Math input 1",
    exact: true,
  });
  await editor.fill("plot(1/x,[x,-5,5])");
  await editor.press("ControlOrMeta+Enter");
  await editor.press("Escape");
  await expect(page.locator(".plot-curve path")).toHaveCount(2);
  await editor.fill("solve(x^2==a,x,reals)");
  await editor.press("ControlOrMeta+Enter");
  await editor.press("Escape");
  await page
    .locator(".solution-actions")
    .getByRole("button", { name: "Plot", exact: true })
    .click();
  const plot = page.locator(".plot-view");
  await expect(plot.locator(".solution-point")).toHaveCount(2);
  await plot.getByRole("slider", { name: "a", exact: true }).press("Home");
  await expect(plot.locator(".solution-point")).toHaveCount(0);
  await expect(plot).toHaveAttribute("aria-busy", "false");
  await editor.fill("ContourPlot[x^2+y^2==4,{x,-3,3},{y,-3,3}]");
  await editor.press("ControlOrMeta+Enter");
  await editor.press("Escape");
  await expect(page.locator(".plot-curve path").first()).toBeVisible();
  await expect(page.locator(".plot-axis-name").last()).toHaveText("y");
  const contour = page.locator(".plot-curve path").first();
  const initial = await contour.getAttribute("d");
  await page.getByRole("img", { name: "Plot", exact: true }).press("ArrowUp");
  await expect.poll(() => contour.getAttribute("d")).not.toBe(initial);
});

test("real extended density, region, vectors, histograms and log coordinates expose kernel geometry", async ({ page }) => {
  await page.setViewportSize({width: 1280,height: 960});
  await page.goto('/');
  await page.getByRole('button',{name:'＋ Math',exact:true}).click();
  const editor=page.getByRole('textbox',{name:'Math input 1',exact:true});
  const run=async (source:string)=>{
    await editor.fill(source);await editor.press('ControlOrMeta+Enter');await editor.press('Escape');
    await expect(page.locator('.notebook-cell').first()).toHaveAttribute('data-status','Done');
    await expect(page.locator('.plot-view')).toHaveAttribute('aria-busy','false');
  };
  for(const [source,count] of [
    ['plot(x*y,x:-2..2,y:-2..2,view:"density")',9216],
    ['data_plot([[0,2],[4,6]],kind:"heatmap")',4],
    ['histogram([1,1,2,3,3],bins:3)',3],
  ] as const){
    await run(source);
    await expect(page.locator('.plot-view rect[fill]')).toHaveCount(count);
    await page.screenshot({path:`test-results/extended-${count}.png`,fullPage:true});
  }
  await run('field_plot([-y,x],x:-2..2,y:-2..2)');
  await expect(page.locator('.plot-view g[fill="none"]')).toHaveCount(400);
  await page.screenshot({path:'test-results/extended-field.png',fullPage:true});
  await run('region_plot(x^2+y^2<1,x:-2..2,y:-2..2)');
  expect(await page.locator('.plot-view rect[fill]').count()).toBeGreaterThan(1700);
  await run('parametric_plot([cos(t),sin(t)],t:0..2*pi)');
  await expect(page.locator('.plot-curve path')).toHaveCount(1);
  await run('plot(x^2,x:1..1000,scale:"log_log")');
  await page.setViewportSize({width:390,height:900});
  const close=page.locator('.inspector').getByRole('button',{name:'Close',exact:true});
  if(await close.isVisible())await close.click();
  await page.locator('.plot-view summary').click();
  await expect(page.locator('.plot-sample-data')).toContainText('"log_log"');
  await expect(page.locator('.plot-sample-data')).toContainText('1000000');
  expect(await page.evaluate(()=>document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await editor.scrollIntoViewIfNeeded();
  await page.screenshot({path:'test-results/extended-log-narrow.png',fullPage:true});
});
