import type { UiCell } from "./notebookStore";
import { saveTextArtifact } from "./files";
export type ExportFormat = "markdown" | "latex";
const escapeTex = (value: string) => value.replace(/[\\{}&%$#_^~]/g, (c) => ({
  "\\": "\\textbackslash{}", "{": "\\{", "}": "\\}", "&": "\\&", "%": "\\%",
  "$": "\\$", "#": "\\#", "_": "\\_", "^": "\\textasciicircum{}", "~": "\\textasciitilde{}",
})[c]!);
function formulas(cell: UiCell): string[] {
  if (cell.status !== "Done") return [];
  return (cell.output?.items ?? []).flatMap((item) => {
    if (item.type === "expr") return [item.latex];
    if (item.type !== "solutions") return [];
    const view = item.view;
    if (view.region_latex) return [view.region_latex];
    if (view.kind === "none") return ["\\varnothing"];
    if (view.kind === "all") return ["\\text{All values}"];
    return view.solutions.map((solution) => {
      const bindings = solution.bindings.map((b) => `${b.var_latex ?? escapeTex(b.var)} = ${b.latex}`).join(",\\quad ");
      const condition = solution.condition_display_latex ?? solution.condition_latex;
      return bindings + (condition ? `\\quad\\text{if } ${condition}` : "");
    });
  });
}
export function renderNotebookExport(title: string, cells: readonly UiCell[], format: ExportFormat): string {
  if (format === "markdown") {
    return `# ${title || "OpenMath"}\n\n` + cells.map((cell) => {
      if (cell.kind === "Text") return `${cell.source}\n\n`;
      const runs = cell.source.match(/`+/g) ?? [];
      const fence = "`".repeat(Math.max(3, ...runs.map((s) => s.length + 1)));
      return `${fence}${cell.kind === "Math" ? "om" : "text"}\n${cell.source}\n${fence}\n\n` +
        formulas(cell).map((latex) => `$$\n${latex}\n$$\n\n`).join("");
    }).join("");
  }
  const body = cells.map((cell) => `\\begin{quote}\\ttfamily\n${escapeTex(cell.source).replace(/\n/g, "\n\\par\n")}\n\\end{quote}\n` +
    formulas(cell).map((latex) => `\\[${latex}\\]\n`).join("")).join("\n");
  return `\\documentclass{article}\n\\usepackage{amsmath,amssymb}\n\\title{${escapeTex(title || "OpenMath")}}\n\\date{}\n\\begin{document}\n\\maketitle\n${body}\\end{document}\n`;
}
export function exportNotebook(title: string, cells: readonly UiCell[], format: ExportFormat, kind: "wasm" | "tauri") {
  return saveTextArtifact(renderNotebookExport(title, cells, format), title || "OpenMath", format === "latex" ? "tex" : "md", kind);
}
