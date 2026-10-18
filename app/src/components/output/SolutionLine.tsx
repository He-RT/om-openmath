import { useState } from "react";
import type { OutputItem } from "../../kernel/generated/OutputItem";
import type { SolutionView } from "../../kernel/generated/SolutionView";
import type { BindingView } from "../../kernel/generated/BindingView";
import type { Verification } from "../../kernel/generated/Verification";
import type { Messages } from "../../i18n";
import { Katex } from "./Katex";
import { CopyMenu } from "./Actions";
import { NumberLine } from "./NumberLine";
import { PlotPreview } from "./PlotPreview";
import type { KernelClient } from "../../kernel/client";
import type { CellPresentation } from "../cellPresentation";
import { CellStatusLine } from "../CellStatusLine";
export function groupSolutions(solutions: SolutionView[]) {
  const groups = new Map<
    string,
    { solution: SolutionView; multiplicity: number }
  >();
  for (const solution of solutions) {
    const key = JSON.stringify([
      solution.bindings.map((b) => [b.var, b.input_form]),
      solution.condition_latex,
      solution.verified,
    ]);
    const group = groups.get(key);
    if (group) group.multiplicity++;
    else groups.set(key, { solution, multiplicity: 1 });
  }
  return [...groups.values()];
}
function verified(v: Verification) {
  return v === "Exact" || v === "ByConstruction";
}
function Binding({ binding: b, t }: { binding: BindingView; t: Messages }) {
  const [radical, setRadical] = useState(false);
  const numeric = b.numeric?.replace(/`[+-]?[\d.]+/g, "");
  return (
    <div
      className="solution-binding"
      title={numeric ? `${b.var} ≈ ${numeric}` : undefined}
    >
      {b.root_index && (
        <span className="root-label">
          Root {b.root_index}
          {numeric && (
            <>
              {" "}
              ≈ <code>{numeric}</code>
            </>
          )}
        </span>
      )}
      <Katex
        latex={`${b.var_latex ?? `\\text{${b.var.replace(/[{}\\]/g, "")}}`} = ${radical && b.radicals ? b.radicals.latex : b.latex}`}
        label={`${b.var} = ${b.input_form}`}
      />
      {b.radicals && (
        <button aria-pressed={radical} onClick={() => setRadical(!radical)}>
          {radical ? t.rootForm : t.radicalForm}
        </button>
      )}
    </div>
  );
}
export function SolutionLine({
  item,
  t,
  onInsert,
  onSteps,
  kernel,
  active = true,
  presentation,
  timing,
  onRun,
}: {
  item: Extract<OutputItem, { type: "solutions" }>;
  t: Messages;
  onInsert: (source: string) => void;
  onSteps: () => void;
  kernel: KernelClient;
  active?: boolean;
  presentation: CellPresentation;
  timing?: number | undefined;
  onRun?: (() => void) | undefined;
}) {
  const [plotVisible, setPlotVisible] = useState(false);
  const { view } = item;
  const groups = groupSolutions(view.solutions);
  const heading =
    view.kind === "none"
      ? t.noSolutions
      : view.kind === "all"
        ? t.allValues
        : view.kind === "region"
          ? t.realRegion
          : `${view.solutions.length} ${view.solutions.length === 1 ? t.oneSolution : t.solutions}`;
  const evidence =
    view.kind === "finite" && view.solutions.length > 0
      ? view.solutions.every((s) => verified(s.verified))
        ? view.solutions.every(s => s.verified === "Exact") ? null : t.byConstruction
        : view.solutions.every((s) => s.verified !== "Unverified")
          ? t.numericVerified
          : t.verificationIncomplete
      : null;
  return (
    <div className="solution-result">
      <div className="solution-list">
      {view.kind !== "finite" && <span className="solution-set-label">{heading}</span>}
      {view.solutions.length > 4 ? <div className="solution-table-scroll" tabIndex={0}><table className="solution-table"><thead><tr><th>#</th><th>{view.solutions.every(row=>verified(row.verified)) ? t.exactValue:t.bindingValue}</th><th>{t.approximateValue}</th></tr></thead><tbody>{groups.map(({solution:s,multiplicity},index)=><tr key={index}><th scope="row">{index+1}{multiplicity>1 && <span className="multiplicity"> · {multiplicity===2 ? t.doubleRoot:`${t.multiplicity} ${multiplicity}`}</span>}</th><td>{s.bindings.map(b=><Binding key={b.var} binding={b} t={t}/>)}{s.condition_latex && <span className="solution-condition"><Katex latex={s.condition_display_latex ?? s.condition_latex}/></span>}</td><td>{s.bindings.map(b=><div key={b.var}><code>{b.numeric ? `${b.var} ≈ ${b.numeric.replace(/`[+-]?[\d.]+/g, "")}`:"—"}</code></div>)}</td></tr>)}</tbody></table></div> :
        groups.map(({ solution: s, multiplicity }, i) => (
          <div className="solution-entry" key={i}>
            {s.bindings.map((b) => (
              <Binding key={b.var} binding={b} t={t} />
            ))}
            {multiplicity > 1 && (
              <span className="multiplicity">
                {multiplicity === 2
                  ? t.doubleRoot
                  : `${t.multiplicity} ${multiplicity}`}
              </span>
            )}
            {s.condition_latex && (
              <div className="solution-condition">
                <Katex latex={s.condition_display_latex ?? s.condition_latex} />
              </div>
            )}
            {s.verified === "Unverified" && <span className="verification unverified">{t.unverified}</span>}
          </div>
        ))}
      </div>
      {view.region_latex && (
        <Katex display latex={view.region_latex} label={item.input_form} />
      )}
      {view.intervals.length > 0 && (
        <NumberLine
          intervals={view.intervals}
          variable={view.vars[0] ?? "x"}
          t={t}
        />
      )}
      <CellStatusLine presentation={presentation} t={t} timing={timing} onRun={onRun} summary={<>{view.kind === "finite" && <span className="solution-heading">{heading}</span>}{evidence && <span>{evidence}</span>}{view.solutions.some(s=>typeof s.verified === "object") && <span>{Math.min(...view.solutions.flatMap(s=>typeof s.verified === "object" ? [s.verified.Numeric.digits]:[]))} {t.digits}</span>}</>} actions={<span className="solution-actions">
        {item.steps && <button onClick={onSteps} disabled={!active}>{t.steps} ↗</button>}
        {item.plot && (
          <button
            aria-expanded={plotVisible}
            onClick={() => setPlotVisible(!plotVisible)}
          >
            {t.plot}
          </button>
        )}
        <CopyMenu value={item} t={t} all>
          {groups.some(group=>group.solution.bindings.length>0) && <hr/>}
          {groups.map(({solution},index)=>solution.bindings.length>0 && <button key={index} onClick={()=>onInsert(`expr /. {${solution.bindings.map(binding=>`${binding.var}->(${binding.input_form})`).join(",")}}`.replace(/->\(([-\d]+)\)/g,"->$1"))}>{t.substitute}{groups.length>1 ? ` ${index+1}`:""}</button>)}
        </CopyMenu>
      </span>} />
      {plotVisible && item.plot && (
        <PlotPreview
          request={item.plot}
          kernel={kernel}
          t={t}
          active={active}
        />
      )}
      <details className="solution-source">
        <summary>{t.source}</summary>
        <pre className="source-result">{item.input_form}</pre>
      </details>
    </div>
  );
}
