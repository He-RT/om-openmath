import { useEffect, useState } from "react";
import type { NotebookController, UiState } from "../state/notebookStore";
import { locale, type Messages } from "../i18n";
import { StepsPanel } from "./steps/StepsPanel";
import type { HoverInfo } from "../kernel/generated/HoverInfo";
import { toByte } from "./editor/positions";
export function Inspector({
  state,
  controller,
  t,
  onSettings,
}: {
  state: UiState;
  controller: NotebookController;
  t: Messages;
  onSettings: () => void;
}) {
  const [docs, setDocs] = useState<HoverInfo | null>(null);
  const [functionName, setFunctionName] = useState("Solve");
  const [values, setValues] = useState<
    { name: string; info: HoverInfo | null }[]
  >([]);
  useEffect(() => {
    let current = true;
    void controller.kernel
      .request({
        type: "hover",
        source: functionName,
        cursor: toByte(functionName, functionName.length),
        dialect: "Wolfram",
      })
      .then((response) => {
        if (current && response.type === "hover") setDocs(response.info);
      })
      .catch(() => {});
    return () => {
      current = false;
    };
  }, [controller, functionName, state.config?.general.language]);
  useEffect(() => {
    let current = true;
    void controller.kernel
      .request({ type: "get_variables" })
      .then((response) => {
        if (current && response.type === "variables")
          setValues(response.items.map(([name, info]) => ({ name, info })));
      })
      .catch(() => {});
    return () => {
      current = false;
    };
  }, [controller, state.cells, state.config?.general.language]);
  if (!state.panel) return null;
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
    <aside
      className={`inspector ${state.panel === "steps" ? "inspector-steps" : ""}`}
    >
      <div className="inspector-header">
        <nav>
          {(["docs", "variables", "steps", "assistant"] as const).map(
            (panel) => (
              <button
                key={panel}
                aria-pressed={state.panel === panel}
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
          ×
        </button>
      </div>
      <div className="inspector-content">
        {state.panel === "docs" ? (
          <>
            <div className="doc-picker">
              {[
                "Solve",
                "Factor",
                "Simplify",
                "Expand",
                "Reduce",
                "N",
                "Plot",
              ].map((name) => (
                <button
                  key={name}
                  className={name === functionName ? "selected" : ""}
                  onClick={() => setFunctionName(name)}
                >
                  {name}
                </button>
              ))}
            </div>
            {docs && (
              <article className="function-doc">
                <h2>{functionName}</h2>
                <code className="signature">{docs.signature}</code>
                <p>{docs.summary}</p>
                {docs.examples.map((example) => (
                  <button
                    className="doc-example"
                    key={example}
                    onClick={() => {
                      const id = controller.add(
                        "Math",
                        example,
                        state.active ?? undefined,
                      );
                      controller.edit(id, { dialect: "Wolfram" });
                      void controller.run(id);
                    }}
                  >
                    <code>{example}</code>
                    <span>↗</span>
                  </button>
                ))}
              </article>
            )}
          </>
        ) : state.panel === "variables" ? (
          <>
            <h2>{t.variables}</h2>
            {values.length === 0 ? (
              <p className="muted">{t.noVariables}</p>
            ) : (
              values.map(({ name, info }) => (
                <div className="variable-row" key={name}>
                  <code>{name}</code>
                  <code>{info?.value ?? "—"}</code>
                </div>
              ))
            )}
          </>
        ) : state.panel === "steps" ? (
          <>
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
                <h2>{t.steps}</h2>
                <p className="muted">{t.selectResult}</p>
              </>
            )}
          </>
        ) : (
          <>
            <h2>{t.assistant}</h2>
            <p className="muted">{t.assistantHint}</p>
            <button onClick={() => controller.add("Ask")}>＋ {t.ask}</button>
          </>
        )}
      </div>
    </aside>
  );
}
