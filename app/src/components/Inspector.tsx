import { useEffect, useState } from "react";
import type { NotebookController, UiState } from "../state/notebookStore";
import type { Messages } from "../i18n";
import type { HoverInfo } from "../kernel/generated/HoverInfo";
import { toByte } from "./editor/positions";
export function Inspector({
  state,
  controller,
  t,
}: {
  state: UiState;
  controller: NotebookController;
  t: Messages;
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
  const steps =
    cell?.output?.items.flatMap((i) =>
      i.type === "solutions" && i.steps ? i.steps.root : [],
    ) ?? [];
  let stepCount = 0;
  const work = [...steps];
  while (work.length) {
    const step = work.pop();
    if (step) {
      stepCount++;
      work.push(...step.children);
    }
  }
  return (
    <aside className="inspector">
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
            <h2>{t.steps}</h2>
            <p className="muted">
              {stepCount ? `${stepCount} ${t.stepsRecorded}` : t.selectResult}
            </p>
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
