import { useStore } from "zustand";
import type { UiState, NotebookController } from "../state/notebookStore";
import type { Messages } from "../i18n";
import { featureProfile } from "./assistant/profile";
import { ToolbarMenu } from "./ToolbarMenu";
import { WorkbenchIcon } from "./WorkbenchIcon";
interface Props {
  state: UiState; controller: NotebookController; t: Messages;
  outlineVisible: boolean; inspectorVisible: boolean;
  onOutline: () => void; onInspector: () => void;
  onSave: () => void; onOpen: () => void; onNew: () => void;
  onExport: (format: "markdown" | "latex") => void;
  onCommands: () => void; onSettings: (ai?: boolean) => void; onDocs: () => void;
}
export function TopBar({ state, controller, t, outlineVisible, inspectorVisible, onOutline, onInspector, onSave, onOpen, onNew, onExport, onCommands, onSettings, onDocs }: Props) {
  const ai = useStore(controller.ai.store);
  const configured = (["translate", "explain", "chat", "complete", "fix"] as const).some(feature => featureProfile(state.config?.llm, feature));
  const aiState = ai.active > 0 ? "working" : ai.error ? "error" : configured ? "ready" : "unconfigured";
  const working=state.busy||state.agentRunning;
  const automatic = state.config?.general.reactive && state.config.general.auto_run_dependents;
  return <header className="topbar">
    <button className="toolbar-icon" aria-label={t.outline} aria-expanded={outlineVisible} onClick={onOutline}><WorkbenchIcon name="outline" /></button>
    <div className="document-heading">
      <input className="notebook-title" aria-label={t.title} value={state.title} placeholder={t.untitled} onChange={event => controller.rename(event.target.value)} />
      <span className="document-status" role="status">{state.stopping||state.agentStopping ? t.stopping : state.agentRunning ? t.aiWorking : state.busy ? t["cell.running"] : `${state.dirty ? t["doc.edited"] + " · " : ""}${t["kernel.ready"]}`}</span>
    </div>
    <div className="top-actions">
      <label className="toolbar-run-mode" title={t.runModeHint}><span className="sr-only">{t.runMode}</span><select aria-label={t.runMode} disabled={state.busy} value={automatic ? "auto" : "manual"} onChange={event => void controller.configure({ auto_run_dependents: event.target.value === "auto", ...(event.target.value === "auto" ? { reactive: true } : {}) })}><option value="manual">{t.manualRun}</option><option value="auto">{t.automaticRun}</option></select></label>
      <button className={working ? "run-all interrupt primary" : "run-all primary"} aria-label={working ? t.stop : t.runAll} title={working ? "⌘." : t.runAll} disabled={!working && !state.cells.some(cell => cell.kind === "Math")} onClick={() => void (working ? controller.stopWork() : controller.runAll())}><WorkbenchIcon name={working ? "stop" : "play"} /><span>{working ? t.stop : t.run}</span></button>
      <ToolbarMenu className="toolbar-add" label={t.add} icon={<><WorkbenchIcon name="plus" /><WorkbenchIcon name="chevron" /></>}><button onClick={() => controller.add("Math", "", state.active ?? undefined)}>{t.math}</button><button onClick={() => controller.add("Text", "", state.active ?? undefined)}>{t.text}</button></ToolbarMenu>
      <button className="toolbar-icon inspector-toggle" aria-label={t.inspector} aria-expanded={inspectorVisible} onClick={onInspector}><WorkbenchIcon name="inspector" /></button>
      <ToolbarMenu className="file-menu" label={t.more} icon={<WorkbenchIcon name="more" />}>
        <button onClick={onNew}>{t.newNotebook}</button><button onClick={onOpen}>{t.open}</button><button aria-label={t.save} onClick={onSave}>{t.save}<kbd>⌘S</kbd></button>
        <button onClick={() => onExport("markdown")}>{t.exportMarkdown}</button><button onClick={() => onExport("latex")}>{t.exportLatex}</button>
        <hr /><button className="mobile-menu-add" onClick={() => controller.add("Math")}>＋ {t.math}</button><button className="mobile-menu-add" onClick={() => controller.add("Text")}>＋ {t.text}</button>
        <button onClick={onDocs}>{t.functionCatalog}</button><button aria-label={t.commands} onClick={onCommands}>{t.commands}<kbd>⌘K</kbd></button>
        <hr /><button onClick={() => onSettings()}>{t.settings}</button>
        <button className={`ai-indicator ai-${aiState}`} title={ai.error ?? (ai.active ? t.aiWorking : configured ? t.ready : t.configureAI)} onClick={() => onSettings(true)}><span className="ai-status-dot" />{t.aiSettings}</button>
      </ToolbarMenu>
    </div>
  </header>;
}
