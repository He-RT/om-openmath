import { fixtureProfile } from "./profileFixture";
import { test, expect, type Page } from "@playwright/test";
async function configure(page: Page) {
  await fixtureProfile(page, "https://ai-provider.invalid/v1", "synthetic-ui-key");
}
function stream(content: string) {
  return `data: ${JSON.stringify({ choices: [{ index: 0, delta: { content }, finish_reason: "stop" }] })}\n\ndata: [DONE]\n\n`;
}
test("actual Ask translation and error repair stay proposals until explicit insert/run/apply", async ({
  page,
}) => {
  let count = 0;
  await configure(page);
  await page.route("https://ai-provider.invalid/**", async (route) => {
    count++;
    const body = route.request().postDataJSON() as {
      messages: { content: string }[];
    };
    const fix = body.messages[0]!.content.includes("Repair the supplied");
    await route.fulfill({
      status: 200,
      contentType: "text/event-stream",
      headers: { "access-control-allow-origin": "*" },
      body: stream(
        JSON.stringify({
          wolfram: fix ? "Solve[x^2==9,x]" : "Solve[x^2==4,x]",
          explanation: "Proposed source",
        }),
      ),
    });
  });
  await page.goto("/");
  await page.getByRole("button", { name: "＋ Ask AI", exact: true }).click();
  const ask = page.getByRole("textbox", {
    name: "Question input 1",
    exact: true,
  });
  await ask.pressSequentially("Solve x squared equals four.");
  await ask.press("Enter");
  await expect(page.getByRole("dialog")).toBeVisible();
  expect(count).toBe(0);
  await page.getByRole("button", { name: "Send to AI", exact: true }).click();
  await expect(page.locator(".suggestion-card")).toContainText(
    "Proposed source",
  );
  await expect(
    page.getByRole("textbox", { name: "Math input 2", exact: true }),
  ).toHaveCount(0);
  await page
    .locator(".suggestion-card")
    .getByRole("button", { name: "Insert as code", exact: true })
    .click();
  await expect(page.locator(".notebook-cell").nth(1)).toHaveAttribute(
    "data-status",
    "Stale",
  );
  await expect(page.locator(".solution-result")).toHaveCount(0);
  await page.getByRole("button", { name: "Run 2", exact: true }).click();
  await expect(page.locator(".solution-chip")).toHaveCount(2);
  const math = page.getByRole("textbox", { name: "Math input 2", exact: true });
  await math.fill("solve(x^2==9,x");
  await math.press("ControlOrMeta+Enter");
  await expect(page.locator(".notebook-cell").nth(1)).toHaveAttribute(
    "data-status",
    "Error",
  );
  await page.getByRole("button", { name: "Fix with AI", exact: true }).click();
  await page.getByRole("button", { name: "Send to AI", exact: true }).click();
  await expect(page.locator(".repair-diff")).toContainText("solve");
  await expect(math).toContainText("solve(x^2==9,x");
  await page
    .locator(".repair-suggestion")
    .getByRole("button", { name: "Apply", exact: true })
    .click();
  await expect(page.locator(".notebook-cell").nth(1)).toHaveAttribute(
    "data-status",
    "Stale",
  );
  await page.getByRole("button", { name: "Run 2", exact: true }).click();
  await expect(page.locator(".solution-list")).toContainText("3");
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.screenshot({
    path: "test-results/ai-ask-repair.png",
    fullPage: true,
  });
  expect(await page.locator(".ai-indicator").getAttribute("class")).toContain(
    "ai-ready",
  );
});
for (const width of [375, 1280])
  test(`assistant performs readonly CAS tools and retains genuine conversation at ${width}px`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height: 900 });
    await configure(page);
    const bodies: { messages: { role: string; content: string }[] }[] = [];
    await page.route("https://ai-provider.invalid/**", async (route) => {
      const body = route.request().postDataJSON() as {
        messages: { role: string; content: string }[];
      };
      bodies.push(body);
      const last = body.messages.at(-1)!;
      let response: string;
      if (last.role === "tool")
        response = stream("CAS returned **x = -2** and **x = 2**. $x^2=4$.");
      else if (last.content.includes("follow-up"))
        response = stream(
          "The previous readonly result remains in this conversation.",
        );
      else
        response = `data: ${JSON.stringify({
          choices: [
            {
              index: 0,
              delta: {
                tool_calls: [
                  {
                    index: 0,
                    id: "calc",
                    type: "function",
                    function: {
                      name: "solve",
                      arguments: JSON.stringify({
                        equations: ["x^2==4"],
                        variables: ["x"],
                        domain: "Reals",
                      }),
                    },
                  },
                  {
                    index: 1,
                    id: "prop",
                    type: "function",
                    function: {
                      name: "propose_cell",
                      arguments: JSON.stringify({
                        code: "Solve[x^2==4,x]",
                        dialect: "wolfram",
                      }),
                    },
                  },
                ],
              },
              finish_reason: "tool_calls",
            },
          ],
        })}\n\ndata: [DONE]\n\n`;
      await route.fulfill({
        status: 200,
        contentType: "text/event-stream",
        headers: { "access-control-allow-origin": "*" },
        body: response,
      });
    });
    await page.goto("/");
    await page.getByRole("button", { name: "＋ Math", exact: true }).click();
    const math = page.getByRole("textbox", {
      name: "Math input 1",
      exact: true,
    });
    await math.fill("2+2");
    await math.press("ControlOrMeta+Enter");
    await page
      .locator(".inspector-header")
      .getByRole("button", { name: "Assistant", exact: true })
      .click();
    const input = page.getByRole("textbox", {
      name: "Message to assistant",
      exact: true,
    });
    await input.fill("Use solve and propose code; inspect @cell1");
    await input.press("Enter");
    await page
      .getByRole("checkbox", {
        name: "Remember for this session and destination",
        exact: true,
      })
      .check();
    await page.getByRole("button", { name: "Send to AI", exact: true }).click();
    await expect(page.locator(".chat-assistant")).toContainText("CAS returned");
    await expect(page.locator(".tool-call-card")).toHaveCount(2);
    await expect(page.locator(".chat-assistant .katex")).not.toHaveCount(0);
    expect(bodies[0]!.messages.at(-1)!.content).toContain('"outputs":["4"]');
    const tool = bodies[1]!.messages.find((m) => m.role === "tool");
    expect(tool?.content).toContain("-2");
    expect(tool?.content).toContain("2");
    await expect(
      page.getByRole("textbox", { name: "Math input 2", exact: true }),
    ).toHaveCount(0);
    await page
      .locator(".chat-assistant .suggestion-card")
      .getByRole("button", { name: "Insert as code", exact: true })
      .click();
    await expect(page.locator(".notebook-cell").nth(1)).toHaveAttribute(
      "data-status",
      "Stale",
    );
    await page
      .locator(".inspector-header")
      .getByRole("button", { name: "Docs", exact: true })
      .click();
    await page
      .locator(".inspector-header")
      .getByRole("button", { name: "Assistant", exact: true })
      .click();
    await expect(page.locator(".chat-assistant")).toContainText("CAS returned");
    await page
      .getByRole("textbox", { name: "Message to assistant", exact: true })
      .fill("A follow-up about that result.");
    await page.getByRole("button", { name: "Send", exact: true }).click();
    await expect(page.locator(".chat-assistant").last()).toContainText(
      "previous readonly result",
    );
    expect(
      bodies
        .at(-1)!
        .messages.some(
          (m) =>
            m.role === "assistant" &&
            m.content.includes("Recorded read-only CAS events"),
        ),
    ).toBe(true);
    await page.screenshot({
      path: `test-results/ai-assistant-${width}.png`,
      fullPage: true,
    });
  });

test("global AI state reflects a real pending request, cancellation and provider rejection", async ({
  page,
}) => {
  await configure(page);
  let release!: (value: void) => void;
  const gate = new Promise<void>((resolve) => (release = resolve));
  let calls = 0;
  await page.route("https://ai-provider.invalid/**", async (route) => {
    calls++;
    if (calls === 1) await gate;
    await route
      .fulfill({
        status: 401,
        contentType: "application/json",
        headers: { "access-control-allow-origin": "*" },
        body: JSON.stringify({ error: { message: "Credential rejected" } }),
      })
      .catch(() => {});
  });
  await page.goto("/");
  await page.getByRole("button", { name: "＋ Ask AI", exact: true }).click();
  const ask = page.getByRole("textbox", {
    name: "Question input 1",
    exact: true,
  });
  await ask.fill("Solve x equals one.");
  await ask.press("Enter");
  await page
    .getByRole("checkbox", {
      name: "Remember for this session and destination",
      exact: true,
    })
    .check();
  await page.getByRole("button", { name: "Send to AI", exact: true }).click();
  await expect(page.locator(".ai-indicator")).toHaveClass(/ai-working/);
  await page
    .locator(".ai-working")
    .getByRole("button", { name: "Cancel", exact: true })
    .click();
  release();
  await expect(page.locator(".ai-indicator")).toHaveClass(/ai-ready/);
  await page
    .locator(".ask-actions")
    .getByRole("button", { name: "Ask AI", exact: true })
    .click();
  await expect(page.locator(".ai-indicator")).toHaveClass(/ai-error/);
  await expect(page.locator(".ask-cell").getByRole("alert")).toContainText(
    "401",
  );
});
