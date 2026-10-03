import { expect, test } from "vitest";
import { renderNotebookExport } from "./export";
import type { UiCell } from "./notebookStore";
test("exports preserve sources, escape TeX text, and include only current computation outputs", () => {
  const cell: UiCell = {
    id: "a", kind: "Math", source: "2+2\n# ```", dialect: "Modern",
    revision: 1, status: "Done", defines: [], uses: [],
    output: { items: [{ type: "expr", out_index: 1, input_form: "4", modern_form: "4", latex: "4" }], messages: [], timing_ms: 0 },
  };
  const stale: UiCell = { ...cell, id: "stale", status: "Stale", output: { items: [{ type: "expr", out_index: 1, input_form: "4", modern_form: "4", latex: "OLD_OUTPUT" }], messages: [], timing_ms: 0 } };
  const text: UiCell = { id: "text", kind: "Text", dialect: "Auto", source: "\\input{private_file}%", revision: 0, status: "Done", defines: [], uses: [] };
  const markdown = renderNotebookExport("Sample", [cell, stale, text], "markdown");
  expect(markdown).toContain("````om");
  expect(markdown).toContain("$$\n4\n$$");
  expect(markdown).not.toContain("OLD_OUTPUT");
  const latex = renderNotebookExport("Sample #1", [cell, stale, text], "latex");
  expect(latex).toContain("\\title{Sample \\#1}");
  expect(latex).toContain("\\textbackslash{}input\\{private\\_file\\}\\%");
  expect(latex).not.toContain("OLD_OUTPUT");
  expect(latex).toContain("\\[4\\]");
});
