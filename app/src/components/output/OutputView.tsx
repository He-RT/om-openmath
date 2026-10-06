import type { CellOutput } from "../../kernel/generated/CellOutput";
import type { Messages, Locale } from "../../i18n";
import type { KernelClient } from "../../kernel/client";
import { ExprView } from "./ExprView";
import { SolutionCards } from "./SolutionCards";
import { Messages as OutputMessages } from "./Messages";
import { PlotView } from "../plot/PlotView";
import { ExploreView } from "../explore/ExploreView";
import { ValueView } from './ValueView';
export function OutputView({
  output,
  stale,
  t,
  language,
  kernel,
  onInsert,
  onSteps,
  cellId = '',
}: {
  output: CellOutput;
  stale: boolean;
  t: Messages;
  language: Locale;
  kernel: KernelClient;
  onInsert: (source: string) => void;
  onSteps: (outIndex: number) => void;
  cellId?: string;
}) {
  return (
    <div
      className={`cell-output ${stale ? "output-stale" : ""}`}
      aria-label={t.allOutputs}
    >
      {stale && <div className="stale-label">{t.stale}</div>}
      {output.items.map((item, i) => (
        <div className="output-item" key={i}>
          {item.type === "explore" ? <ExploreView key={`${cellId}:${item.out_index}:${item.view_id}`} item={item} cellId={cellId} fresh={!stale} kernel={kernel} t={t} language={language} onInsert={onInsert}/> : item.type === "expr" ? (
            item.presentation ? <ValueView key={`${item.out_index}:${item.presentation.view_id}`}
              initial={item.presentation} cellId={cellId} outIndex={item.out_index} value={item}
              fresh={!stale} kernel={kernel} language={language} t={t} onInsert={onInsert} /> : <ExprView
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
              active={!stale}
            />
          ) : item.type === "error" ? (
            <div role="alert" className="output-error">
              {item.message}
            </div>
          ) : (
            <PlotView
              request={item.request}
              data={item.data}
              kernel={kernel}
              t={t}
              active={!stale}
            />
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
