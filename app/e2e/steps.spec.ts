import { test, expect } from "@playwright/test";
for (const width of [375, 1280])
  test(`recorded steps are real and local controls work at ${width}px`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height: 900 });
    await page.goto("/");
    await page.getByRole("button", { name: /Quadratic equation/ }).click();
    await page
      .locator(".solution-actions")
      .getByRole("button", { name: /Steps/ })
      .click();
    await expect(page.locator(".step-node").first()).toBeVisible();
    await expect(page.locator(".step-timeline")).toContainText("S1");
    await expect(
      page
        .locator(".steps-panel")
        .getByRole("button", { name: "Explain all", exact: true }),
    ).toBeDisabled();
    await page
      .getByRole("checkbox", { name: "Show all details", exact: true })
      .check();
    await expect(
      page.locator(".step-transformation .katex").first(),
    ).toBeVisible();
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth,
      ),
    ).toBe(true);
    await page.screenshot({
      path: `test-results/steps-${width}-light.png`,
      fullPage: true,
    });
    await page
      .getByRole("banner")
      .getByRole("button", { name: "Preferences", exact: true })
      .click();
    await page.getByLabel("Theme", { exact: true }).selectOption("dark");
    await page.getByLabel("Language", { exact: true }).selectOption("zh-CN");
    await page
      .getByRole("dialog")
      .getByRole("button", { name: "关闭", exact: true })
      .click();
    await expect(
      page.getByRole("checkbox", { name: "显示全部细节", exact: true }),
    ).toBeVisible();
    await page.screenshot({
      path: `test-results/steps-${width}-dark.png`,
      fullPage: true,
    });
  });
test("browser streams explanations for the selected real output and sends only its recorded source/steps", async ({
  page,
}) => {
  const bodies: Record<string, unknown>[] = [];
  await page.route("https://steps-provider.invalid/**", async (route) => {
    const body = route.request().postDataJSON() as Record<string, unknown>;
    bodies.push(body);
    const messages = body.messages as { role: string; content: string }[];
    const data = JSON.parse(messages[1]!.content) as {
      input: string;
      steps: { root: { id: string; children: { id: string }[] }[] };
    };
    const target =
      data.steps.root[0]!.children[0]?.id ?? data.steps.root[0]!.id;
    const reply = `**Recorded** [${target}] $x^2$.\n\n${data.input}`;
    await route.fulfill({
      status: 200,
      contentType: "text/event-stream",
      headers: { "access-control-allow-origin": "*" },
      body: `data: ${JSON.stringify({ choices: [{ index: 0, delta: { content: reply }, finish_reason: "stop" }] })}\n\ndata: [DONE]\n\n`,
    });
  });
  await page.route("**/src/kernel/index.ts", async (route) => {
    await route.fulfill({
      contentType: "application/javascript",
      body: `import {WasmClient} from '/src/kernel/wasmClient.ts';
export async function createKernelClient(onCreated) {
  const client=new WasmClient();onCreated?.(client);await client.ready;
  const r=await client.request({type:'get_config'});if(r.type!=='config')throw new Error('config');
  r.config.llm.profiles[0].base_url='https://steps-provider.invalid/v1';r.config.llm.profiles[0].api_key='synthetic-step-key';r.config.llm.send_context=false;
  await client.request({type:'set_config',config:r.config});
  await client.request({type:'load_notebook',file:{version:1,title:'Recorded steps',cells:[{id:'multi',source:'solve(x^2==4,x)\\nsolve(y^2==9,y)',kind:'Math',dialect:'Modern'}]}});
  return client;
}`,
    });
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Run 1", exact: true }).click();
  await expect(page.locator(".solution-result")).toHaveCount(2);
  await page
    .locator(".solution-result")
    .first()
    .getByRole("button", { name: /Steps/ })
    .click();
  await expect(page.getByLabel("Steps output")).toHaveValue("1");
  await page.getByRole("button", { name: "Explain all", exact: true }).click();
  await expect(page.getByRole("dialog")).toContainText(
    "https://steps-provider.invalid/v1",
  );
  expect(bodies).toHaveLength(0);
  await page.getByRole("button", { name: "Send to AI", exact: true }).click();
  await expect(page.locator(".step-explanation")).toContainText("Recorded");
  expect(bodies).toHaveLength(1);
  const messages = bodies[0]!.messages as { content: string }[];
  const payload = JSON.parse(messages[1]!.content) as {
    input: string;
    result: string;
    steps: { root: unknown[] };
  };
  expect(payload.input).toContain("x^2");
  expect(payload.input).not.toContain("y^2");
  expect(payload.result).toContain("2");
  expect(payload.steps.root.length).toBeGreaterThan(0);
  await page.locator(".step-reference").first().click();
  await expect(page.locator(".step-highlight")).toBeVisible();
});
