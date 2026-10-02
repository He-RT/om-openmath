import type { CellOutput } from "../../kernel/generated/CellOutput";
import type { Messages, Locale } from "../../i18n";
import type { KernelClient } from "../../kernel/client";
import { ExprView } from "./ExprView";
import { SolutionCards } from "./SolutionCards";
import { Messages as OutputMessages } from "./Messages";
export function OutputView({
  output,
  stale,
  t,
  language,
  kernel,
  onInsert,
  onSteps,
}: {
  output: CellOutput;
  stale: boolean;
  t: Messages;
  language: Locale;
  kernel: KernelClient;
  onInsert: (source: string) => void;
  onSteps: (outIndex: number) => void;
}) {
  return (
    <div
      className={`cell-output ${stale ? "output-stale" : ""}`}
      aria-label={t.allOutputs}
    >
      {stale && <div className="stale-label">{t.stale}</div>}
      {output.items.map((item, i) => (
        <div className="output-item" key={i}>
          {item.type === "expr" ? (
            <ExprView
              key={`${item.out_index}:${item.input_form}`}
              value={item}
              kernel={kernel}
              t={t}
              onInsert={onInsert}
            />
          ) : item.type === "solutions" ? (
            <SolutionCards
              key={`${item.out_index}:${item.input_form}`}
              item={item}
              kernel={kernel}
              t={t}
              onInsert={onInsert}
              onSteps={() => onSteps(item.out_index)}
            />
          ) : item.type === "error" ? (
            <div role="alert" className="output-error">
              {item.message}
            </div>
          ) : (
            <div className="plot-summary">
              <code>{item.request.exprs.join(", ")}</code>
              <span>
                {item.data.curves.length} {t.plot}
              </span>
            </div>
          )}
        </div>
      ))}
      <OutputMessages items={output.messages} t={t} language={language} />
      {output.items.length === 0 && output.messages.length === 0 && (
        <div className="suppressed-output">{t.suppress}</div>
      )}
      <span className={`timing ${output.timing_ms > 1000 ? "slow" : ""}`}>
        {output.timing_ms < 1 ? "< 1" : Math.round(output.timing_ms)} ms
      </span>
    </div>
  );
}
