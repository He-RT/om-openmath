import { useState } from "react";
import type { OutputItem } from "../../kernel/generated/OutputItem";
import type { SolutionView } from "../../kernel/generated/SolutionView";
import type { BindingView } from "../../kernel/generated/BindingView";
import type { Verification } from "../../kernel/generated/Verification";
import type { Messages } from "../../i18n";
import { Katex } from "./Katex";
import { CopyButton } from "./Actions";
import { NumberLine } from "./NumberLine";
import { PlotPreview } from "./PlotPreview";
import type { KernelClient } from "../../kernel/client";
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
        latex={`${b.var_latex ?? `\\text{${b.var.replace(/[{}\\]/g, "")}}`} = ${radical && b.radicals ? b.radicals.latex : b.root_index ? `\\operatorname{Root}_{${b.root_index}}` : b.latex}`}
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
export function SolutionCards({
  item,
  t,
  onInsert,
  onSteps,
  kernel,
  active = true,
}: {
  item: Extract<OutputItem, { type: "solutions" }>;
  t: Messages;
  onInsert: (source: string) => void;
  onSteps: () => void;
  kernel: KernelClient;
  active?: boolean;
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
        ? t.verified
        : view.solutions.every((s) => s.verified !== "Unverified")
          ? t.numericVerified
          : t.verificationIncomplete
      : null;
  return (
    <div className="solution-result">
      <div className="solution-heading">
        <strong>{heading}</strong>
        {evidence && <span>{evidence}</span>}
      </div>
      <div className="solution-list">
        {groups.map(({ solution: s, multiplicity }, i) => (
          <div className="solution-chip" key={i}>
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
            <span
              className={`verification ${s.verified === "Unverified" ? "unverified" : ""}`}
              title={
                s.verified === "ByConstruction"
                  ? t.byConstruction
                  : typeof s.verified === "object"
                    ? `${s.verified.Numeric.digits} ${t.digits}`
                    : undefined
              }
            >
              {verified(s.verified)
                ? `✓ ${t.exact}`
                : s.verified === "Unverified"
                  ? t.unverified
                  : `≈ ${t.numeric}`}
            </span>
            {s.bindings.length > 0 && (
              <button
                className="substitute"
                onClick={() =>
                  onInsert(
                    `expr /. {${s.bindings.map((b) => `${b.var}->(${b.input_form})`).join(",")}}`.replace(
                      /->\(([-\d]+)\)/g,
                      "->$1",
                    ),
                  )
                }
              >
                {t.substitute}
              </button>
            )}
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
      <div className="solution-actions">
        {item.steps && <button onClick={onSteps}>{t.steps} ↗</button>}
        {item.plot && (
          <button
            aria-expanded={plotVisible}
            onClick={() => setPlotVisible(!plotVisible)}
          >
            {t.plot}
          </button>
        )}
        <CopyButton label={t.copyAll} source={item.input_form} t={t} />
      </div>
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
        <CopyButton label={t.copyModern} source={item.modern_form} t={t} />
      </details>
    </div>
  );
}
