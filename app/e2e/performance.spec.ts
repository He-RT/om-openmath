import { test, expect } from "@playwright/test";
import { execFileSync } from "node:child_process";
import { mkdir, readdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
test("all53 original authority rows execute through cold production WASM workers below1s", async ({ page }) => {
  test.skip(process.env.OPENMATH_PERFORMANCE !== "1", "Explicit release performance acceptance");
  test.setTimeout(180_000);
  expect(process.env.OPENMATH_E2E_PREVIEW).toBe("1");
  const cases = JSON.parse(execFileSync("python3", ["-c", "import json,tomllib,pathlib;print(json.dumps([{'id':c['id'],'input':c['input']} for c in tomllib.loads(pathlib.Path('../tests/corpus/solve.toml').read_text())['case']]))"], { encoding: "utf8" })) as { id: string; input: string }[];
  expect(cases).toHaveLength(53);
  const worker = (await readdir("dist/assets")).find((name) => /^worker-.*\.js$/.test(name));
  if (!worker) throw new Error("Missing production worker asset");
  await page.goto("/");
  const rows = await page.evaluate(async ({ cases, worker }) => {
    const rows: { id: string; input_form: string; timing_ms: number }[] = [];
    for (const entry of cases) {
      const port = new Worker(`/assets/${worker}`, { type: "module" });
      try {
        await new Promise<void>((resolve, reject) => {
          port.onerror = () => reject(new Error("Worker startup failed"));
          port.onmessage = (event: MessageEvent<{type?: string}>) => { if (event.data.type === "ready") resolve(); else if (event.data.type === "error") reject(new Error("Worker initialization failed")); };
          port.postMessage({ type: "init", config: JSON.stringify({ general: { auto_plot: false, reactive: false, auto_run_dependents: false }, llm: { enabled: false } }) });
        });
        const output = await new Promise<{type: string; output: { timing_ms: number; items: { input_form?: string }[] }}>((resolve, reject) => {
          port.onmessage = (event) => event.data.response ? resolve(event.data.response.body) : reject(new Error("Worker evaluation failed"));
          port.postMessage({ type: "request", id: 1, envelope: JSON.stringify({ id: 1, body: { type: "evaluate", cell_id: "acceptance", source: entry.input, dialect: "Wolfram" } }) });
        });
        if (output.type !== "evaluated") throw new Error(`Evaluation #${entry.id} failed`);
        const input_form = output.output.items.at(-1)?.input_form;
        if (!input_form) throw new Error(`No math output #${entry.id}`);
        rows.push({ id: entry.id, input_form, timing_ms: output.output.timing_ms });
      } finally { port.terminate(); }
    }
    return rows;
  }, { cases, worker });
  const folder = resolve("../target/pre-alpha");
  await mkdir(folder, { recursive: true });
  await writeFile(resolve(folder, "wasm-corpus.json"), JSON.stringify(rows, null, 2) + "\n");
  expect(rows.filter((row) => row.timing_ms >= 1000)).toEqual([]);
});
