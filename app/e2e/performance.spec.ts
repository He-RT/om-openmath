import { test, expect } from "@playwright/test";
import { execFileSync } from "node:child_process";
import { mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
const projectRoot = fileURLToPath(new URL("../../", import.meta.url));
const measurementFolder = process.env.OPENMATH_MEASUREMENT_DIR ? resolve(process.env.OPENMATH_MEASUREMENT_DIR) : resolve(projectRoot,"target/pre-alpha");
test("all53 original authority rows execute through cold production WASM workers below1s", async ({ page }) => {
  test.skip(process.env.OPENMATH_PERFORMANCE !== "1", "Explicit release performance acceptance");
  test.setTimeout(180_000);
  expect(process.env.OPENMATH_E2E_PREVIEW).toBe("1");
  const cases = JSON.parse(execFileSync("python3", ["-c", "import json,tomllib,pathlib,sys;print(json.dumps([{'id':c['id'],'input':c['input']} for c in tomllib.loads(pathlib.Path(sys.argv[1]).read_text())['case']]))", resolve(projectRoot, "tests/corpus/solve.toml")], { encoding: "utf8" })) as { id: string; input: string }[];
  expect(cases).toHaveLength(53);
  const assets = resolve(projectRoot,"app/dist/assets");
  const candidates = await Promise.all((await readdir(assets)).filter(name => /^worker-.*\.js$/.test(name)).map(async name => ({ name, source: await readFile(resolve(assets,name),"utf8") })));
  const kernels = candidates.filter(asset => asset.source.includes("Kernel initialization failed"));
  expect(kernels, "Exactly one production math worker, excluding the agent worker").toHaveLength(1);
  const worker = kernels[0]!.name;
  await page.goto("/");
  const rows = await page.evaluate(async ({ cases, worker }) => {
    const rows: { id: string; input_form: string; timing_ms: number; roundtrip_ms: number; startup_ms: number }[] = [];
    for (const entry of cases) {
      const opened = performance.now();
      const port = new Worker(`/assets/${worker}`, { type: "module" });
      try {
        await new Promise<void>((resolve, reject) => {
          port.onerror = () => reject(new Error("Worker startup failed"));
          port.onmessage = (event: MessageEvent<{type?: string}>) => { if (event.data.type === "ready") resolve(); else if (event.data.type === "error") reject(new Error("Worker initialization failed")); };
          port.postMessage({ type: "init", config: JSON.stringify({ general: { auto_plot: false, reactive: false, auto_run_dependents: false }, llm: { enabled: false } }) });
        });
        const startup_ms = performance.now() - opened;
        const started = performance.now();
        const output = await new Promise<{type: string; output: { timing_ms: number; items: { input_form?: string }[] }}>((resolve, reject) => {
          port.onmessage = (event) => event.data.response ? resolve(event.data.response.body) : reject(new Error("Worker evaluation failed"));
          port.postMessage({ type: "request", id: 1, envelope: JSON.stringify({ id: 1, body: { type: "evaluate", cell_id: "acceptance", source: entry.input, dialect: "Wolfram" } }) });
        });
        if (output.type !== "evaluated") throw new Error(`Evaluation #${entry.id} failed`);
        const input_form = output.output.items.at(-1)?.input_form;
        if (!input_form) throw new Error(`No math output #${entry.id}`);
        rows.push({ id: entry.id, input_form, timing_ms: output.output.timing_ms, roundtrip_ms: performance.now() - started, startup_ms });
      } finally { port.terminate(); }
    }
    return rows;
  }, { cases, worker });
  const folder = measurementFolder;
  await mkdir(folder, { recursive: true });
  await writeFile(resolve(folder, "wasm-corpus.json"), JSON.stringify(rows, null, 2) + "\n");
  expect(rows.filter((row) => row.timing_ms >= 1000)).toEqual([]);
  expect(rows.filter((row) => row.roundtrip_ms >= 1000)).toEqual([]);
});


test("production 10KiB editor input reaches a painted frame within50ms p95", async ({ page }) => {
  test.skip(process.env.OPENMATH_PERFORMANCE !== "1", "Explicit release performance acceptance");
  test.setTimeout(90_000);
  expect(process.env.OPENMATH_E2E_PREVIEW).toBe("1");
  await page.goto("/");
  await page.getByRole("button", { name: "＋ Math", exact: true }).click();
  const editor = page.getByRole("textbox", { name: "Math input 1", exact: true });
  const source = "#" + "x".repeat(10_239);
  await editor.fill(source);
  await expect(editor).toHaveText(source);
  await editor.press("ControlOrMeta+End");
  const idle_frames_ms = await page.evaluate(async () => {
    const values: number[] = [];let previous = performance.now();
    for(let i=0;i<30;i++) {const now = await new Promise<number>((resolve) => requestAnimationFrame(resolve));values.push(now-previous);previous=now;}
    return values;
  });
  await editor.evaluate((element) => {
    const measurement = { samples: [] as number[], lengths: [] as number[] };
    Object.assign(window, { openmathInputMeasurement: measurement });
    element.addEventListener("beforeinput", () => {
      const start = performance.now();
      // The second callback bounds the first frame's paint from above; it does not
      // mistake a before-paint callback or a React state change for visible output.
      requestAnimationFrame(() => requestAnimationFrame(() => {
        measurement.samples.push(performance.now() - start);
        measurement.lengths.push(element.textContent?.length ?? 0);
      }));
    }, { capture: true });
  });
  for (let i = 0; i < 100; i++) {
    await page.keyboard.insertText(i % 4 === 0 ? "中" : i % 4 === 1 ? "🙂" : "x");
    await page.waitForFunction((count) => {
      const data = (window as unknown as { openmathInputMeasurement: { samples: number[] } }).openmathInputMeasurement;
      return data.samples.length >= count;
    }, i + 1, { polling: "raf" });
  }
  const measurement = await page.evaluate((idle_frames_ms) => {
    const data = (window as unknown as { openmathInputMeasurement: { samples: number[]; lengths: number[] } }).openmathInputMeasurement;
    const sorted = [...data.samples].sort((a, b) => a - b);
    const gl = document.createElement("canvas").getContext("webgl");
    const extension = gl?.getExtension("WEBGL_debug_renderer_info");
    const renderer = gl && extension ? gl.getParameter(extension.UNMASKED_RENDERER_WEBGL) as string : "unavailable";
    return { source_bytes: 10_240, samples_ms: data.samples, idle_frames_ms, renderer, rendered_utf16_lengths: data.lengths,
      p95_ms: sorted[Math.ceil(sorted.length * 0.95) - 1], max_ms: sorted.at(-1),
      first_contentful_paint_ms: performance.getEntriesByName("first-contentful-paint")[0]?.startTime,
      definition: "trusted beforeinput to second requestAnimationFrame, an upper bound on the next visible frame" };
  }, idle_frames_ms);
  const folder = measurementFolder;
  await mkdir(folder, { recursive: true });
  await writeFile(resolve(folder, "web-input.json"), JSON.stringify(measurement, null, 2) + "\n");
  expect(measurement.samples_ms).toHaveLength(100);
  expect(measurement.rendered_utf16_lengths.every((length, index) => length > 10_240 + index)).toBe(true);
  expect(measurement.p95_ms).toBeLessThanOrEqual(50);
});
