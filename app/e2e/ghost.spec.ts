import { test, expect } from "@playwright/test";
test("actual CodeMirror ghost text requires consent, keeps prefixes and accepts partial/full text without evaluation", async ({
  page,
}) => {
  let count = 0;
  await page.route("https://ghost-provider.invalid/**", async (route) => {
    count++;
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      headers: { "access-control-allow-origin": "*" },
      body: JSON.stringify({
        choices: [
          {
            index: 0,
            message: { role: "assistant", content: " + sin(α)" },
            finish_reason: "stop",
          },
        ],
      }),
    });
  });
  await page.route("**/src/kernel/index.ts", async (route) => {
    await route.fulfill({
      contentType: "application/javascript",
      body: `import {WasmClient} from '/src/kernel/wasmClient.ts';
export async function createKernelClient(onCreated) {const client=new WasmClient();onCreated?.(client);await client.ready;
const r=await client.request({type:'get_config'});if(r.type!=='config')throw new Error('config');
r.config.llm.complete='deepseek';r.config.llm.profiles[0].base_url='https://ghost-provider.invalid/v1';r.config.llm.profiles[0].api_key='synthetic-ghost-key';r.config.llm.send_context=false;
await client.request({type:'set_config',config:r.config});return client;}`,
    });
  });
  await page.goto("/");
  await page.getByRole("button", { name: "＋ Math", exact: true }).click();
  const editor = page.getByRole("textbox", {
    name: "Math input 1",
    exact: true,
  });
  await editor.fill("sqrt(2)+3");
  await expect(page.getByRole("dialog")).toContainText("current code");
  expect(count).toBe(0);
  await page
    .getByRole("checkbox", {
      name: "Remember for this session and destination",
      exact: true,
    })
    .check();
  await page.getByRole("button", { name: "Send to AI", exact: true }).click();
  await expect(page.locator(".cm-ghost")).toHaveText(" + sin(α)");
  await page.screenshot({
    path: "test-results/ghost-completion.png",
    fullPage: true,
  });
  await expect(editor).toContainText("sqrt(2)+3");
  await editor.press("ControlOrMeta+ArrowRight");
  await expect(page.locator(".cm-ghost")).toHaveText("sin(α)");
  await editor.press("Tab");
  await expect(editor).toContainText("sqrt(2)+3 + sin(α)");
  await expect(page.locator(".notebook-cell")).toHaveAttribute(
    "data-status",
    "Stale",
  );
  expect(await page.locator(".execution-count").textContent()).toBe("[ ]");
  await editor.fill("sqrt(2)+4");
  await expect(page.locator(".cm-ghost")).toHaveText(" + sin(α)");
  await editor.press("Space");
  await expect(page.locator(".cm-ghost")).toHaveText("+ sin(α)");
  await editor.press("Escape");
  await expect(page.locator(".cm-ghost")).toHaveCount(0);
  await editor.fill("sq");
  await editor.press("Control+Space");
  await expect(page.locator(".cm-tooltip-autocomplete")).toContainText("sqrt");
  const before = count;
  await page.waitForTimeout(450);
  expect(count).toBe(before);
  await expect(page.locator(".cm-ghost")).toHaveCount(0);
  await editor.press("Escape");
  await editor.fill("\\alpha");
  const beforeGreek = count;
  await page.waitForTimeout(450);
  expect(count).toBe(beforeGreek);
  await editor.press("Tab");
  await expect(editor).toHaveText("α");
});
test("a pending real provider completion is cancelled on source/cursor change and cannot paint a late reply", async ({
  page,
}) => {
  let finish!: (value: void) => void;
  const gate = new Promise<void>((resolve) => (finish = resolve));
  let requested = false;
  await page.route("https://ghost-provider.invalid/**", async (route) => {
    requested = true;
    await gate;
    await route
      .fulfill({
        status: 200,
        contentType: "application/json",
        headers: { "access-control-allow-origin": "*" },
        body: JSON.stringify({
          choices: [
            {
              index: 0,
              message: { role: "assistant", content: " + 99" },
              finish_reason: "stop",
            },
          ],
        }),
      })
      .catch(() => {});
  });
  await page.route("**/src/kernel/index.ts", async (route) => {
    await route.fulfill({
      contentType: "application/javascript",
      body: `import {WasmClient} from '/src/kernel/wasmClient.ts';
export async function createKernelClient(onCreated) {const client=new WasmClient();onCreated?.(client);await client.ready;const r=await client.request({type:'get_config'});r.config.llm.complete='deepseek';r.config.llm.profiles[0].base_url='https://ghost-provider.invalid/v1';r.config.llm.profiles[0].api_key='synthetic-ghost-key';r.config.llm.send_context=false;await client.request({type:'set_config',config:r.config});return client;}`,
    });
  });
  await page.goto("/");
  await page.getByRole("button", { name: "＋ Math", exact: true }).click();
  const editor = page.getByRole("textbox", {
    name: "Math input 1",
    exact: true,
  });
  await editor.fill("sqrt(2)+3");
  await page.getByRole("button", { name: "Send to AI", exact: true }).click();
  await expect.poll(() => requested).toBe(true);
  await editor.press("ArrowLeft");
  finish();
  await page.waitForTimeout(400);
  await expect(page.locator(".cm-ghost")).toHaveCount(0);
  await expect(editor).toContainText("sqrt(2)+3");
});
