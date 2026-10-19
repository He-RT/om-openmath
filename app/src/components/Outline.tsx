import { useMemo, useRef } from "react";
import { marked } from "marked";
import type { NotebookController, UiCell, UiState } from "../state/notebookStore";
import type { Messages } from "../i18n";
import { WorkbenchIcon } from "./WorkbenchIcon";
import { useResponsiveDialog } from "./useResponsiveDialog";
import { cellPresentation, presentationLabel } from "./cellPresentation";
export function outlineTitle(cell: UiCell): { label: string; heading: boolean } {
  if (cell.kind === "Text") {
    const heading = marked.lexer(cell.source).find(token => token.type === "heading" && token.depth <= 2);
    if (heading?.type === "heading") return { label: heading.tokens?.map(token => "text" in token ? String(token.text) : token.raw).join("") ?? heading.text, heading: true };
  }
  return { label: cell.source.trim().split("\n")[0]?.slice(0, 180) ?? "", heading: false };
}
export function Outline({ state, controller, t, visible, overlay, onClose }: { state: UiState; controller: NotebookController; t: Messages; visible: boolean; overlay: boolean; onClose: () => void }) {
  const root = useRef<HTMLDialogElement>(null);
  const cache = useRef(new Map<string, { source: string; kind: string; title: ReturnType<typeof outlineTitle> }>());
  useResponsiveDialog(root, visible, overlay);
  const rows = useMemo(() => {
    const current = new Map<string, { source: string; kind: string; title: ReturnType<typeof outlineTitle> }>();
    const rows = state.cells.map(cell => {
      const previous = cache.current.get(cell.id);
      const value = previous?.source === cell.source && previous.kind === cell.kind ? previous : { source: cell.source, kind: cell.kind, title: outlineTitle(cell) };
      current.set(cell.id, value);
      return { cell, ...value.title };
    });
    cache.current = current;
    return rows;
  }, [state.cells]);
  return <dialog className={`outline-host ${overlay ? "outline-overlay" : ""}`} ref={root} role={overlay ? "dialog" : "presentation"} aria-label={overlay ? t.outline : undefined} onCancel={event => { event.preventDefault(); onClose(); }}>
    <aside className="outline" aria-label={t.outline}>
      <header className="outline-brand"><span aria-hidden="true">∑</span><strong>OpenMath</strong>{overlay && <button aria-label={t.close} onClick={onClose}><WorkbenchIcon name="close" /></button>}</header>
      <nav className="outline-list" aria-label={t.outline}>
        {rows.map(({ cell, label, heading }, index) => {
          const presentation=cellPresentation(cell,t);
          return <button key={cell.id} className={`outline-row ${heading ? "outline-heading" : ""}`} aria-current={state.active === cell.id ? "location" : undefined} title={label || `${t[cell.kind === "Text" ? "text" : cell.kind === "Ask" ? "ask" : "math"]} ${index + 1}`} onClick={() => {
          controller.select(cell.id);
          const target = [...document.querySelectorAll<HTMLElement>(".notebook-cell[data-cell-id]")].find(node => node.dataset.cellId === cell.id);
          target?.scrollIntoView({ block: "nearest", behavior: "instant" });
          if (overlay) onClose();
        }}>
          {!heading && <span className={`outline-status status-${presentation.status} ${presentation.byAgent?'outline-by-agent':''}`} aria-label={presentationLabel(presentation,t)} />}
          <span>{label || `${t[cell.kind === "Text" ? "text" : cell.kind === "Ask" ? "ask" : "math"]} ${index + 1}`}</span>
        </button>;})}
      </nav>
      <footer>{state.cells.length} {t.cellCount}</footer>
    </aside>
  </dialog>;
}
