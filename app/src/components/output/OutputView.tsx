import type { CellOutput } from "../../kernel/generated/CellOutput";
import type { Messages } from "../../i18n";
import { Katex } from "./Katex";
export function OutputView({
  output,
  stale,
  t,
}: {
  output: CellOutput;
  stale: boolean;
  t: Messages;
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
            <Katex display latex={item.latex} label={item.input_form} />
          ) : item.type === "solutions" ? (
            <pre className="source-result">{item.input_form}</pre>
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
      {output.messages.length > 0 && (
        <details
          className="messages"
          open={output.messages.some((m) => m.level === "Error")}
        >
          <summary>
            {output.messages.length} {t.messages}
          </summary>
          {output.messages.map((m, i) => (
            <div className={`message ${m.level.toLowerCase()}`} key={i}>
              <code>
                {m.symbol}::{m.tag}
              </code>{" "}
              — {m.text}
            </div>
          ))}
        </details>
      )}
      {output.items.length === 0 && output.messages.length === 0 && (
        <div className="suppressed-output">{t.suppress}</div>
      )}
      <span className={`timing ${output.timing_ms > 1000 ? "slow" : ""}`}>
        {output.timing_ms < 1 ? "< 1" : Math.round(output.timing_ms)} ms
      </span>
    </div>
  );
}
