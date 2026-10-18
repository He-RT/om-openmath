import type { StepView } from "../../kernel/generated/StepView";
import type { Locale, Messages } from "../../i18n";
import type { Explanation } from "./useExplanation";
import { Katex } from "../output/Katex";
import { Markdown } from "../TextCell";
import { StepTitle, StepParams } from "./StepTitle";
export function ExplanationView({
  value,
  t,
  onCancel,
  onReference,
  knownIds,
}: {
  value?: Explanation;
  t: Messages;
  onCancel: () => void;
  onReference: (id: string) => void;
  knownIds: Set<string>;
}) {
  if (!value) return null;
  return (
    <div className="step-explanation">
      {value.text && (
        <Markdown
          source={value.text}
          t={t}
          stepReferences={{ knownIds, onReference, label: t.goToStep }}
        />
      )}
      {value.status === "streaming" && (
        <div className="explanation-status">
          <span role="status">{t.explaining}</span>
          <button onClick={onCancel}>{t.cancelExplanation}</button>
        </div>
      )}
      {value.status === "error" && (
        <p className="output-error" role="alert">
          {value.error}
        </p>
      )}
      {value.status === "cancelled" && (
        <p className="muted">{t.explanationCancelled}</p>
      )}
    </div>
  );
}
export function StepNode({
  step,
  t,
  language,
  expanded,
  onToggle,
  highlight,
  onExplain,
  disabled,
  disabledHint,
  explanations,
  cancel,
  knownIds,
  onReference,
  showExplain = true,
}: {
  step: StepView;
  t: Messages;
  language: Locale;
  expanded: Set<string>;
  onToggle: (id: string) => void;
  highlight: string | null;
  onExplain: (id: string) => void;
  disabled: boolean;
  disabledHint: string;
  explanations: Record<string, Explanation>;
  cancel: (id: string) => void;
  knownIds: Set<string>;
  onReference: (id: string) => void;
  showExplain?: boolean;
}) {
  const open = expanded.has(step.id);
  return (
    <li
      className={`step-node ${highlight === step.id ? "step-highlight" : ""}`}
      data-step-id={step.id}
    >
      <div className="step-heading">
        <button
          className="step-toggle"
          aria-expanded={open}
          onClick={() => onToggle(step.id)}
          aria-label={`${t.toggleStep} ${step.id}`}
        >
          <span className="step-id" title={step.id}>{step.id.replace(/^S/,'')}</span>
          <StepTitle step={step} language={language} />
          <span className="step-chevron">{open ? "−" : "＋"}</span>
        </button>
      </div>
      {open && (
        <div className="step-detail">
          {(step.before_latex.length > 0 || step.after_latex.length > 0) && (
            <div className="step-transformation">
              <div>
                {(step.after_latex.length ? step.after_latex : step.before_latex).map((latex, i) => (
                  <Katex key={i} display latex={latex} />
                ))}
              </div>
            </div>
          )}
          {(Object.keys(step.params).length > 0 || step.after_latex.length > 0 && step.before_latex.length > 0) && <details className="step-evidence">
            <summary>{t.stepEvidence}</summary>
            <StepParams step={step} language={language} />
            {step.after_latex.length > 0 && step.before_latex.map((latex,i)=><Katex key={i} display latex={latex}/>)}
          </details>}
          {showExplain && <button
            className="step-why"
            aria-label={`${t.why} ${step.id}`}
            title={disabled ? disabledHint : undefined}
            disabled={disabled || explanations[step.id]?.status === "streaming"}
            onClick={() => onExplain(step.id)}
          >
            {t.why}
          </button>}
          <ExplanationView
            {...(explanations[step.id] ? { value: explanations[step.id] } : {})}
            t={t}
            onCancel={() => cancel(step.id)}
            onReference={onReference}
            knownIds={knownIds}
          />
        </div>
      )}
      {open && step.children.length > 0 && (
        <ol className="step-children">
          {step.children.map((child) => (
            <StepNode
              key={child.id}
              step={child}
              t={t}
              language={language}
              expanded={expanded}
              onToggle={onToggle}
              highlight={highlight}
              onExplain={onExplain}
              disabled={disabled}
              disabledHint={disabledHint}
              explanations={explanations}
              cancel={cancel}
              knownIds={knownIds}
              onReference={onReference}
              showExplain={showExplain}
            />
          ))}
        </ol>
      )}
    </li>
  );
}
