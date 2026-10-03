import type { UiState, NotebookController } from "../state/notebookStore";
import type { Messages } from "../i18n";
import { useStore } from "zustand";
import { featureProfile } from "./assistant/profile";
interface Props {
  state: UiState;
  controller: NotebookController;
  t: Messages;
  onSave: () => void;
  onOpen: () => void;
  onNew: () => void;
  onCommands: () => void;
  onSettings: (ai?: boolean) => void;
}
export function TopBar({
  state,
  controller,
  t,
  onSave,
  onOpen,
  onNew,
  onCommands,
  onSettings,
}: Props) {
  const ai = useStore(controller.ai.store);
  const configured = (
    ["translate", "explain", "chat", "complete", "fix"] as const
  ).some((feature) => featureProfile(state.config?.llm, feature));
  const aiState =
    ai.active > 0
      ? "working"
      : ai.error
        ? "error"
        : configured
          ? "ready"
          : "unconfigured";
  return (
    <header className="topbar">
      <div className="brand">
        <span className="brand-symbol" aria-hidden>
          ∑
        </span>
        <strong>OpenMath</strong>
        <span className="brand-separator">/</span>
      </div>
      <input
        className="notebook-title"
        aria-label={t.title}
        value={state.title}
        placeholder={t.untitled}
        onChange={(e) => controller.rename(e.target.value)}
      />
      <span className="save-dot" title={state.dirty ? t.dirty : t.ready}>
        {state.dirty ? "•" : ""}
      </span>
      <div className="top-actions">
        <label className="default-dialect">
          <span className="sr-only">{t.dialect}</span>
          <select
            aria-label={t.dialect}
            value={state.config?.general.dialect ?? "auto"}
            onChange={(e) =>
              void controller.configure({
                dialect: e.target.value as "auto" | "modern" | "wolfram",
              })
            }
          >
            <option value="auto">{t.auto}</option>
            <option value="modern">{t.modern}</option>
            <option value="wolfram">Wolfram</option>
          </select>
        </label>
        <button
          className="run-all primary"
          aria-label={t.runAll}
          onClick={() => void controller.runAll()}
          disabled={state.busy || !state.cells.some((c) => c.kind === "Math")}
        >
          ▶ <span>{t.runAll}</span>
        </button>
        {state.busy && (
          <button
            className="interrupt"
            aria-label={t.stop}
            onClick={() => void controller.interrupt()}
            title="⌘."
          >
            {t.stop} ■
          </button>
        )}
        <button className="toolbar-save" onClick={onSave} title="⌘S">
          {t.save}
        </button>
        <button
          className="command-trigger"
          aria-label={t.commands}
          onClick={onCommands}
        >
          ⌘K
        </button>
        <button
          aria-label={t.settings}
          title={t.settings}
          onClick={() => onSettings()}
        >
          ☷
        </button>
        <button
          className={`ai-indicator ai-${aiState}`}
          aria-label={t.aiSettings}
          title={
            ai.error ??
            (ai.active ? t.aiWorking : configured ? t.ready : t.configureAI)
          }
          onClick={() => onSettings(true)}
        >
          <span className="ai-status-dot" />
          AI
        </button>
        <details className="file-menu">
          <summary aria-label={t.open}>⋯</summary>
          <div>
            <button onClick={onNew}>{t.newNotebook}</button>
            <button onClick={onOpen}>{t.open}</button>
            <button onClick={onSave}>{t.save}</button>
            <button
              onClick={() => void controller.runAll()}
              disabled={state.busy}
            >
              {t.runAll}
            </button>
            <button onClick={onCommands}>{t.commands}</button>
          </div>
        </details>
      </div>
    </header>
  );
}
