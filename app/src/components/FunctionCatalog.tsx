import { useEffect, useRef, useState } from "react";
import type { NotebookController } from "../state/notebookStore";
import type { HoverInfo } from "../kernel/generated/HoverInfo";
import type { Messages } from "../i18n";
import { toByte } from "./editor/positions";
import { useResponsiveDialog } from "./useResponsiveDialog";
export function FunctionCatalog({ open, onClose, controller, t }: { open: boolean; onClose: () => void; controller: NotebookController; t: Messages }) {
  const root = useRef<HTMLDialogElement>(null);
  useResponsiveDialog(root, open, true);
  const [name, setName] = useState("Solve"), [docs, setDocs] = useState<HoverInfo | null>(null);
  const language = controller.store.getState().config?.general.language;
  useEffect(() => {
    if (!open) return;
    let current = true;
    setDocs(null);
    void controller.kernel.request({ type: "hover", source: name, cursor: toByte(name, name.length), dialect: "Wolfram" }).then(response => { if (current && response.type === "hover") setDocs(response.info); }).catch(() => {});
    return () => { current = false; };
  }, [controller, name, open, language]);
  return <dialog className="function-catalog" ref={root} aria-label={t.functionCatalog} onCancel={event => { event.preventDefault(); onClose(); }}>
    <header><h2>{t.functionCatalog}</h2><button aria-label={t.close} onClick={onClose}>×</button></header>
    <label className="catalog-search">{t.searchFunction}<input value={name} onChange={event => setName(event.target.value)} spellCheck={false} /></label>
    <div className="doc-picker">{["Solve", "Factor", "Simplify", "Expand", "Reduce", "N", "Plot"].map(item => <button key={item} onClick={() => setName(item)} aria-pressed={name === item}>{item}</button>)}</div>
    {docs ? <article className="function-doc"><h3>{name}</h3><code className="signature">{docs.signature}</code><p>{docs.summary}</p>{docs.examples.map(example => <button key={example} className="doc-example" onClick={() => {
      const id = controller.add("Math", example, controller.store.getState().active ?? undefined);
      controller.edit(id, { dialect: "Wolfram" }); onClose(); void controller.run(id);
    }}><code>{example}</code><span>↗</span></button>)}</article> : <p className="muted">{t.selectFunction}</p>}
  </dialog>;
}
