import type { NotebookController, UiState } from "../state/notebookStore";
import type { Messages, Locale } from "../i18n";
import { Cell } from "./Cell";
import { Katex } from "./output/Katex";
const examples = [
  "solve(x^2 + 2*x == 3, x)",
  "solve([x+y==10, x-y==2], [x,y])",
  "reduce(x^2 < 4, x, Reals)",
];
export function Notebook({
  state,
  controller,
  t,
  language,
}: {
  state: UiState;
  controller: NotebookController;
  t: Messages;
  language: Locale;
}) {
  return (
    <main className="notebook" aria-label={state.title || t.untitled}>
      {state.cells.length === 0 ? (
        <div className="empty-notebook">
          <span className="eyebrow">OpenMath</span>
          <h1>{t.emptyTitle}</h1>
          <p>{t.emptyText}</p>
          <div className="empty-formula">
            <Katex latex="x^2 + 2x = 3" display />
          </div>
          <div className="example-list">
            {[t.quadratic, t.systemExample, t.inequality].map((label, i) => (
              <button
                key={label}
                onClick={() => {
                  const id = controller.add("Math", examples[i] ?? "");
                  controller.edit(id, { dialect: "Modern" });
                  void controller.run(id);
                }}
              >
                <span>{label}</span>
                <code>{examples[i]}</code>
                <span>↗</span>
              </button>
            ))}
          </div>
        </div>
      ) : (
        state.cells.map((cell, index) => (
          <Cell
            key={cell.id}
            cell={cell}
            index={index}
            controller={controller}
            t={t}
            language={language}
            settingsKey={`${state.config?.general.dialect}/${state.config?.general.constants}`}
            active={state.active === cell.id}
            focusKey={state.focus}
            busy={state.busy}
          />
        ))
      )}
      <div className="add-cell-bar">
        <button className="add-math" onClick={() => controller.add("Math")}>
          ＋ {t.math}
        </button>
        <button onClick={() => controller.add("Text")}>＋ {t.text}</button>
        <button onClick={() => controller.add("Ask")}>＋ {t.ask}</button>
      </div>
      <footer className="notebook-footer">
        <span>{t.shortcut}</span>
        <span>
          {controller.kernel.kind === "wasm" ? "Web · WASM" : "Desktop · Rust"}
        </span>
      </footer>
    </main>
  );
}
