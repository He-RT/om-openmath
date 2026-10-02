import { useMemo, useRef, useState } from "react";
import type { KernelClient } from "../../kernel/client";
import type { KernelConfig } from "../../kernel/generated/KernelConfig";
import type { StepsView } from "../../kernel/generated/StepsView";
import type { StepView } from "../../kernel/generated/StepView";
import type { Locale, Messages } from "../../i18n";
import { PrivacyPrompt, hasConsent } from "../assistant/PrivacyPrompt";
import { useExplanation } from "./useExplanation";
import { StepNode, ExplanationView } from "./StepNode";
export function flattenSteps(steps: StepsView) {
  const result: StepView[] = [];
  const work = [...steps.root].reverse();
  while (work.length) {
    const step = work.pop();
    if (step) {
      result.push(step);
      work.push(...[...step.children].reverse());
    }
  }
  return result;
}
export interface StepsPanelProps {
  steps: StepsView;
  cellId: string;
  outIndex: number;
  current: boolean;
  config: KernelConfig | null;
  kernel: KernelClient;
  t: Messages;
  language: Locale;
  onSettings?: () => void;
}
const identities = new WeakMap<object, number>();
let nextIdentity = 0;
function identity(value: object | null) {
  if (!value) return 0;
  let id = identities.get(value);
  if (id === undefined) {
    id = ++nextIdentity;
    identities.set(value, id);
  }
  return id;
}
export function StepsPanel(props: StepsPanelProps) {
  const scope = [
    props.cellId,
    props.outIndex,
    props.current,
    identity(props.steps),
    props.language,
    identity(props.config),
  ].join(":");
  return <RecordedSteps key={scope} {...props} />;
}
function RecordedSteps({
  steps,
  cellId,
  outIndex,
  current,
  config,
  kernel,
  t,
  language,
  onSettings,
}: StepsPanelProps) {
  const list = useMemo(() => flattenSteps(steps), [steps]);
  const ids = useMemo(() => new Set(list.map((s) => s.id)), [list]);
  const [expanded, setExpanded] = useState(
    () => new Set(list.filter((s) => s.level === "Major").map((s) => s.id)),
  );
  const [all, setAll] = useState(false);
  const [highlight, setHighlight] = useState<string | null>(null);
  const [pending, setPending] = useState<string | null | undefined>(undefined);
  const root = useRef<HTMLElement>(null);
  const explanation = useExplanation(
    kernel,
    cellId,
    outIndex,
    t.explanationError,
  );
  const profiles =
    config?.llm.profiles.filter((p) => p.name === config.llm.explain) ?? [];
  const profile = profiles.length === 1 ? profiles[0] : undefined;
  const configured = !!(
    config?.llm.enabled &&
    profile &&
    (profile.kind === "openai_chat" || profile.kind === "anthropic") &&
    profile.base_url &&
    profile.model &&
    (profile.api_key !== null ||
      Object.keys(profile.extra_headers).length > 0 ||
      (kernel.kind === "tauri" && profile.api_key_env))
  );
  const disabled = !current || !configured || list.length === 0;
  const hint = !current
    ? t.staleExplanation
    : list.length === 0
      ? t.noSteps
      : t.configureAI;
  const explain = (id: string | null) => {
    if (disabled || !profile) return;
    if (hasConsent(kernel, profile)) explanation.start(id);
    else setPending(id);
  };
  const reference = (id: string) => {
    if (!ids.has(id)) return;
    const ancestors: string[] = [];
    const search = (nodes: StepView[], path: string[]): boolean => {
      for (const s of nodes) {
        if (s.id === id) {
          ancestors.push(...path, s.id);
          return true;
        }
        if (search(s.children, [...path, s.id])) return true;
      }
      return false;
    };
    search(steps.root, []);
    setExpanded((old) => new Set([...old, ...ancestors]));
    setHighlight(id);
    requestAnimationFrame(() => {
      const node = [
        ...(root.current?.querySelectorAll<HTMLElement>("[data-step-id]") ??
          []),
      ].find((n) => n.dataset.stepId === id);
      node?.scrollIntoView?.({
        block: "nearest",
        behavior: matchMedia("(prefers-reduced-motion: reduce)").matches
          ? "auto"
          : "smooth",
      });
      if (node && !matchMedia("(prefers-reduced-motion: reduce)").matches)
        node.animate?.(
          [
            { backgroundColor: "var(--accent-soft)" },
            { backgroundColor: "transparent" },
          ],
          { duration: 1200 },
        );
    });
  };
  return (
    <section className="steps-panel" ref={root} aria-label={t.steps}>
      <header className="steps-header">
        <div>
          <h2>{t.steps}</h2>
          <span>
            {list.length} {t.stepsRecorded} · Out[{outIndex}]
          </span>
        </div>
        <label>
          <input
            type="checkbox"
            checked={all}
            onChange={(e) => {
              setAll(e.target.checked);
              setExpanded(
                new Set(
                  list
                    .filter((s) => e.target.checked || s.level === "Major")
                    .map((s) => s.id),
                ),
              );
            }}
          />
          {t.showDetails}
        </label>
        <button
          className="explain-all"
          disabled={disabled || explanation.items.all?.status === "streaming"}
          title={disabled ? hint : undefined}
          onClick={() => explain(null)}
        >
          {t.explainAll}
        </button>
        {disabled && (
          <div className="explanation-hint">
            <span>{hint}</span>
            {onSettings && !configured && (
              <button onClick={onSettings}>{t.settings}</button>
            )}
          </div>
        )}
      </header>
      <ExplanationView
        {...(explanation.items.all ? { value: explanation.items.all } : {})}
        t={t}
        onCancel={() => explanation.cancel("all")}
        onReference={reference}
        knownIds={ids}
      />
      <ol className="step-timeline">
        {steps.root.map((step) => (
          <StepNode
            key={step.id}
            step={step}
            t={t}
            language={language}
            expanded={expanded}
            onToggle={(id) => {
              setAll(false);
              setExpanded((old) => {
                const next = new Set(old);
                if (next.has(id)) next.delete(id);
                else next.add(id);
                return next;
              });
            }}
            highlight={highlight}
            onExplain={(id) => explain(id)}
            disabled={disabled}
            disabledHint={hint}
            explanations={explanation.items}
            cancel={explanation.cancel}
            knownIds={ids}
            onReference={reference}
          />
        ))}
      </ol>
      {pending !== undefined && profile && (
        <PrivacyPrompt
          kernel={kernel}
          profile={profile}
          t={t}
          onClose={() => setPending(undefined)}
          onSend={() => {
            explanation.start(pending);
            setPending(undefined);
          }}
        />
      )}
    </section>
  );
}
