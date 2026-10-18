import type { ReactNode } from "react";
import type { Messages } from "../i18n";
import { presentationLabel, type CellPresentation } from "./cellPresentation";
export function CellStatusLine({ presentation, t, timing, summary, actions, onRun }: { presentation: CellPresentation; t: Messages; timing?: number | undefined; summary?: ReactNode; actions?: ReactNode; onRun?: (() => void) | undefined }) {
  const symbol = presentation.status === "numeric" ? "≈" : presentation.status === "conditional" ? "ⓘ" : presentation.status === "error" ? "⚠" : ["verified", "exact", "geometry"].includes(presentation.status) ? "✓" : "○";
  return <div className={`cell-status-line presentation-${presentation.status}`}>
    <span className="result-status"><span aria-hidden="true">{symbol} </span>{presentationLabel(presentation, t)}</span>
    {summary}
    {timing !== undefined && <span className={timing > 1000 ? "timing slow" : "timing"}>{timing < 1 ? "< 1" : Math.round(timing)} ms</span>}
    {presentation.status === "stale" && onRun && <button onClick={onRun}>{t.runToUpdate}</button>}
    {actions && <span className="result-status-actions">{actions}</span>}
  </div>;
}
