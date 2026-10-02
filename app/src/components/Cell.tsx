import { useState } from "react";
import type { NotebookController, UiCell } from "../state/notebookStore";
import type { Messages, Locale } from "../i18n";
import type { Dialect } from "../kernel/generated/Dialect";
import { MathEditor } from "./editor/MathEditor";
import { Markdown } from "./TextCell";
import { OutputView } from "./output/OutputView";
interface Props {
  cell: UiCell;
  index: number;
  controller: NotebookController;
  t: Messages;
  language: Locale;
  settingsKey: string;
  active: boolean;
  focusKey: number;
  busy: boolean;
}
export function Cell({
  cell,
  index,
  controller,
  t,
  language,
  settingsKey,
  active,
  focusKey,
  busy,
}: Props) {
  const [selection, setSelection] = useState<{
    action: string;
    source: string;
    dialect: Dialect;
  } | null>(null);
  const [variable, setVariable] = useState("x");
  const selectedAction = (
    action: string,
    source: string,
    dialect: Dialect,
    axis = "x",
  ) => {
    const wl = dialect === "Wolfram";
    const name = action[0]?.toUpperCase() + action.slice(1);
    const code =
      action === "plot"
        ? wl
          ? `Plot[${source}, {${axis}, -5, 5}]`
          : `plot(${source}, [${axis}, -5, 5])`
        : action === "solve"
          ? wl
            ? `Solve[${source}, ${axis}]`
            : `solve(${source}, ${axis})`
          : wl
            ? `${name}[${source}]`
            : `${action}(${source})`;
    const id = controller.add("Math", code, cell.id);
    controller.edit(id, { dialect });
    void controller.run(id);
    setSelection(null);
  };
  return (
    <section
      className={`notebook-cell ${active ? "cell-active" : ""}`}
      data-cell-id={cell.id}
      data-status={cell.status}
      onMouseDown={() => controller.select(cell.id)}
    >
      <div className="cell-rail">
        <span
          className={`status-dot status-${cell.status.toLowerCase()}`}
          title={t[cell.status]}
        />
        <span className="execution-count">
          {cell.kind === "Math" ? `[${cell.exec_count ?? " "}]` : "✎"}
        </span>
      </div>
      <div className="cell-body">
        <div className="cell-toolbar">
          <span className="cell-number">
            {String(index + 1).padStart(2, "0")}
          </span>
          <select
            aria-label={`${t.math} / ${t.text} ${index + 1}`}
            value={cell.kind}
            onChange={(e) =>
              controller.edit(cell.id, {
                kind: e.target.value as UiCell["kind"],
              })
            }
          >
            <option value="Math">{t.math}</option>
            <option value="Text">{t.text}</option>
            <option value="Ask">{t.ask}</option>
          </select>
          {cell.kind === "Math" && (
            <select
              className="cell-dialect"
              aria-label={`${t.dialect} ${index + 1}`}
              value={cell.dialect}
              onChange={(e) =>
                controller.edit(cell.id, { dialect: e.target.value as Dialect })
              }
            >
              <option value="Auto">{t.auto}</option>
              <option value="Modern">{t.modern}</option>
              <option value="Wolfram">Wolfram</option>
            </select>
          )}
          <span className="cell-status-label">{t[cell.status]}</span>
          <div className="cell-actions">
            {cell.kind === "Math" && (
              <button
                className="run-cell"
                aria-label={`${t.run} ${index + 1}`}
                onClick={() => void controller.run(cell.id)}
                disabled={busy}
              >
                ▶
              </button>
            )}
            <button
              aria-label={`${t.moveUp} ${index + 1}`}
              onClick={() => controller.move(cell.id, -1)}
            >
              ↑
            </button>
            <button
              aria-label={`${t.moveDown} ${index + 1}`}
              onClick={() => controller.move(cell.id, 1)}
            >
              ↓
            </button>
            <button
              aria-label={`${t.delete} ${index + 1}`}
              onClick={() => controller.remove(cell.id)}
            >
              ×
            </button>
          </div>
        </div>
        {cell.kind === "Math" ? (
          <MathEditor
            id={cell.id}
            index={index}
            source={cell.source}
            dialect={cell.dialect}
            settingsKey={settingsKey}
            client={controller.kernel}
            t={t}
            language={language}
            llm={controller.store.getState().config?.llm ?? null}
            busy={busy}
            focus={active}
            focusKey={focusKey}
            onChange={(source) => controller.edit(cell.id, { source })}
            onFocus={() => controller.select(cell.id)}
            onRun={(mode) => void controller.run(cell.id, mode)}
            onAction={(source) => {
              controller.edit(cell.id, { source });
              void controller.run(cell.id);
            }}
            onSelect={(action, source, dialect) => {
              if (action === "solve" || action === "plot") {
                setSelection({ action, source, dialect });
                setVariable(cell.uses[0] ?? "x");
              } else selectedAction(action, source, dialect);
            }}
          />
        ) : (
          <>
            <textarea
              className="prose-editor"
              aria-label={`${cell.kind === "Text" ? t.textInput : t.question} ${index + 1}`}
              placeholder={cell.kind === "Ask" ? t.question : t.text}
              value={cell.source}
              onFocus={() => controller.select(cell.id)}
              onChange={(e) =>
                controller.edit(cell.id, { source: e.target.value })
              }
            />
            {cell.kind === "Text" && cell.source && (
              <Markdown source={cell.source} />
            )}
            {cell.kind === "Ask" && (
              <p className="muted ask-hint">{t.askHint}</p>
            )}
          </>
        )}
        {cell.kind === "Math" && cell.output && (
          <OutputView
            output={cell.output}
            stale={
              cell.status === "Stale" ||
              cell.status === "Running" ||
              cell.status === "Queued"
            }
            t={t}
            language={language}
            kernel={controller.kernel}
            onInsert={(source) => {
              const id = controller.add("Math", source, cell.id);
              controller.edit(id, { dialect: "Wolfram" });
            }}
            onSteps={(outIndex) => {
              controller.select(cell.id);
              controller.store.setState({
                panel: "steps",
                stepSelection: { cellId: cell.id, outIndex },
              });
            }}
          />
        )}
        {selection && (
          <form
            className="variable-prompt"
            onSubmit={(e) => {
              e.preventDefault();
              selectedAction(
                selection.action,
                selection.source,
                selection.dialect,
                variable,
              );
            }}
          >
            <label>
              {t.variable}
              <input
                autoFocus
                aria-label={t.variable}
                value={variable}
                onChange={(e) => setVariable(e.target.value)}
              />
            </label>
            <button type="submit">{t.apply}</button>
            <button type="button" onClick={() => setSelection(null)}>
              {t.close}
            </button>
          </form>
        )}
      </div>
    </section>
  );
}
