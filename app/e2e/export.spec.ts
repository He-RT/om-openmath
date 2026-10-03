import { expect, test } from "@playwright/test";
import { readFile } from "node:fs/promises";
test("production exports contain actual CAS roots and exclude edited stale output", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: /Quadratic equation/ }).click();
  await expect(page.locator(".solution-list")).toContainText("-3");
  await page.getByRole("textbox", { name: "Notebook title" }).fill("Export acceptance");
  async function download(label: string) {
    await page.getByRole("button", { name: "Command palette", exact: true }).click();
    const pending = page.waitForEvent("download");
    await page.getByRole("option", { name: label, exact: true }).click();
    const artifact = await pending;
    const path = await artifact.path();
    if (!path) throw new Error("Missing download");
    return { name: artifact.suggestedFilename(), content: await readFile(path, "utf8") };
  }
  const markdown = await download("Export Markdown");
  expect(markdown.name).toBe("Export acceptance.md");
  expect(markdown.content).toContain("x = -3");
  const latex = await download("Export LaTeX");
  expect(latex.content).toContain("\\begin{document}");
  expect(latex.content).toContain("x = -3");
  await page.getByRole("textbox", { name: "Math input 1", exact: true }).fill("2+2");
  const stale = await download("Export Markdown");
  expect(stale.content).toContain("2+2");
  expect(stale.content).not.toContain("x = -3");
});
