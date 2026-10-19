import { useEffect, useRef, useState } from "react";
import type { NotebookController, UiState } from "../state/notebookStore";
import { locale, type Messages } from "../i18n";
import { StepsPanel } from "./steps/StepsPanel";
import { AssistantPanel } from "./assistant/AssistantPanel";
import { VariablesPanel } from "./VariablesPanel";
import { useResponsiveDialog } from "./useResponsiveDialog";
import { WorkbenchIcon } from "./WorkbenchIcon";
export function Inspector({
  state,
  controller,
  t,
  onSettings,
  phone,
}: {
  state: UiState;
  controller: NotebookController;
  t: Messages;
  onSettings: () => void;
  phone: boolean;
}) {
  const root = useRef<HTMLDialogElement>(null);
  useResponsiveDialog(root, !!state.panel, phone);
  const [lastPanel, setLastPanel] = useState<NonNullable<UiState["panel"]>>("assistant");
  useEffect(() => { if (state.panel) setLastPanel(state.panel); }, [state.panel]);
  const panel = state.panel ?? lastPanel;
  const cell = state.cells.find((c) => c.id === state.active);
  const results =
    cell?.output?.items.filter((i) => i.type === "solutions" && i.steps) ?? [];
  const selected =
    results.find(
      (i) =>
        i.type === "solutions" &&
        i.out_index ===
          (state.stepSelection?.cellId === cell?.id
            ? state.stepSelection?.outIndex
            : undefined),
    ) ?? results.at(-1);
  return (
    <dialog ref={root} className="inspector-host" role={phone ? "dialog" : "presentation"} aria-label={phone ? t.inspector : undefined} onCancel={event => { event.preventDefault(); controller.store.setState({ panel: null }); }}>
    <aside className={`inspector inspector-${panel}`} aria-label={t.inspector}>
      <div className="inspector-grabber" aria-hidden="true" />
      <div className="inspector-header">
        <nav role="tablist" aria-label={t.inspector}>
          {(["assistant", "steps", "variables"] as const).map(
            (panel) => (
              <button
                key={panel}
                role="tab"
                id={`inspector-tab-${panel}`}
                aria-controls={`inspector-pane-${panel}`}
                aria-selected={state.panel === panel}
                tabIndex={state.panel === panel ? 0 : -1}
                onKeyDown={event => {
                  const tabs = ["assistant", "steps", "variables"] as const;
                  if (["ArrowRight", "ArrowLeft", "Home", "End"].includes(event.key)) {
                    event.preventDefault();
                    const index = tabs.indexOf(panel), next = event.key === "Home" ? tabs[0] : event.key === "End" ? tabs[2] : tabs[(index + (event.key === "ArrowRight" ? 1 : 2)) % 3]!;
                    controller.store.setState({ panel: next }); document.getElementById(`inspector-tab-${next}`)?.focus();
                  }
                }}
                onClick={() => controller.store.setState({ panel })}
              >
                {t[panel]}
              </button>
            ),
          )}
        </nav>
        <button
          aria-label={t.close}
          onClick={() => controller.store.setState({ panel: null })}
        >
          <WorkbenchIcon name="close" />
        </button>
      </div>
      <div className="inspector-content" role="tabpanel" id="inspector-pane-variables" aria-labelledby="inspector-tab-variables" hidden={panel !== "variables"}>
        <VariablesPanel key={state.documentGeneration} cells={state.cells} controller={controller}
          generation={state.documentGeneration} busy={state.busy} language={locale(state.config?.general.language)} t={t} phone={phone} />
      </div>
      <div className="inspector-content" role="tabpanel" id="inspector-pane-steps" aria-labelledby="inspector-tab-steps" hidden={panel !== "steps"}>
            {results.length > 1 && (
              <label className="steps-output-picker">
                {t.allOutputs}
                <select
                  aria-label={t.stepsOutput}
                  value={
                    selected?.type === "solutions" ? selected.out_index : ""
                  }
                  onChange={(e) => {
                    if (cell)
                      controller.store.setState({
                        stepSelection: {
                          cellId: cell.id,
                          outIndex: Number(e.target.value),
                        },
                      });
                  }}
                >
                  {results.map(
                    (i) =>
                      i.type === "solutions" && (
                        <option key={i.out_index} value={i.out_index}>
                          Out[{i.out_index}]
                        </option>
                      ),
                  )}
                </select>
              </label>
            )}
            {cell && selected?.type === "solutions" && selected.steps ? (
              <StepsPanel
                cellId={cell.id}
                cellNumber={state.cells.indexOf(cell)+1}
                source={cell.resultSource?.source ?? cell.source}
                outIndex={selected.out_index}
                steps={selected.steps}
                current={cell.status === "Done" && !state.busy}
                config={state.config}
                kernel={controller.kernel}
                t={t}
                language={locale(state.config?.general.language)}
                onSettings={onSettings}
              />
            ) : (
              <>
                <h2>{cell ? t.cellSteps.replace('{n}',String(state.cells.indexOf(cell)+1)) : t.steps}</h2>
                {cell && <pre className="steps-source">{cell.resultSource?.source ?? cell.source}</pre>}
                <p className="muted">{cell ? t.noSteps : t.selectResult}</p>
              </>
            )}
      </div>
      <div className="inspector-content" role="tabpanel" id="inspector-pane-assistant" aria-labelledby="inspector-tab-assistant" hidden={panel !== "assistant"}>
        <AssistantPanel state={state} controller={controller} t={t} onSettings={onSettings} />
      </div>
    </aside>
    </dialog>
  );
}
