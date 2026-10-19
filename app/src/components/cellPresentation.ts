import type { CellOutput } from "../kernel/generated/CellOutput";
import type { UiCell } from "../state/notebookStore";
import type { Messages } from "../i18n";
export type CellPresentationStatus = "pending" | "running" | "verified" | "exact" | "numeric" | "geometry" | "structured" | "conditional" | "stale" | "error" | "unevaluated" | "stopped" | "unconfirmed";
export interface CellPresentationFacts {
  running?: boolean; stopped?: boolean; hasResult?: boolean; sourceChanged?: boolean; staleReason?: string;
  error?: boolean; conditional?: boolean; verified?: boolean; unevaluated?: boolean; hasMetadata?: boolean; nature?: string; outputKind?: string; structured?: boolean; byAgent?: boolean;
}
export interface CellPresentation { status: CellPresentationStatus; byAgent: boolean; reason?: string }
/** Same precedence as the native CellPresentationState. Missing evidence remains explicit. */
export function deriveCellPresentation(f: CellPresentationFacts): CellPresentation {
  const status: CellPresentationStatus = f.running ? "running" : f.stopped ? "stopped" : !f.hasResult ? "pending" : f.sourceChanged || f.staleReason ? "stale" : f.error ? "error" : !f.hasMetadata ? "unconfirmed" : f.outputKind === "plot" || f.outputKind === "scene3_d" ? "geometry" : f.conditional ? "conditional" : f.verified ? "verified" : f.unevaluated ? "unevaluated" : f.structured ? "structured" : f.nature === "machine" || f.nature === "high_precision" ? "numeric" : "exact";
  return { status, byAgent: f.byAgent ?? false, ...(status === "stale" && f.staleReason ? { reason: f.staleReason } : {}) };
}
export function outputFacts(output: CellOutput): CellPresentationFacts {
  const expressions = output.items.filter(item => item.type === "expr"), solutions = output.items.filter(item => item.type === "solutions");
  const solutionRows = solutions.flatMap(item => item.view.solutions), last = output.items.at(-1);
  return {
    hasResult: true, error: output.items.some(item => item.type === "error") || output.messages.some(item => item.level === "Error"),
    hasMetadata: output.items.length === 0 || expressions.some(item => item.nature !== undefined) || solutions.length > 0 && !solutionRows.some(row => row.verified === "Unverified") || last?.type === "plot" || last?.type === "scene3_d" || last?.type === "explore",
    structured:last?.type==='expr'&&!!last.presentation&&last.presentation.kind!=='scalar',
    outputKind: last?.type ?? "", nature: expressions.at(-1)?.nature ?? (solutionRows.some(row => typeof row.verified === "object") ? "high_precision" : ""),
    conditional: solutionRows.some(row => !!row.condition_latex),
    verified: output.items.every(item => item.type === "solutions") && solutionRows.length > 0 && solutionRows.every(row => row.verified === "Exact"),
    unevaluated: last?.type === "expr" && !last.presentation && last.nature === "symbolic" && last.unchanged === true,
  };
}
export function cellPresentation(cell: UiCell, t: Messages): CellPresentation {
  const f = cell.output ? outputFacts(cell.output) : {};
  return deriveCellPresentation({ ...f, running: !!cell.stopRequested || cell.status === "Running" || cell.status === "Queued", stopped: !!cell.stopped,
    sourceChanged: !!cell.resultSource && (cell.source !== cell.resultSource.source || cell.dialect !== cell.resultSource.dialect),
    ...(cell.status === "Stale" && cell.output ? { staleReason: cell.resultSource && (cell.source !== cell.resultSource.source || cell.dialect !== cell.resultSource.dialect) ? t.sourceChanged : t.definitionsChanged } : {}),
    error: f.error || cell.status === "Error",byAgent:cell.byAgent===true,
  });
}
export function presentationLabel(p: CellPresentation, t: Messages) {
  if (p.status === "stale") return `${t["cell.stale"]}${p.reason ? " · " + p.reason : ""}`;
  if (p.status === "geometry") return t.geometryReady;
  if (p.status === "unevaluated") return t.unevaluated;
  if (p.status === "unconfirmed") return t.resultUnconfirmed;
  return t[`cell.${p.status}`];
}
