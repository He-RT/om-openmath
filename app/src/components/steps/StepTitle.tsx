import type { StepView } from "../../kernel/generated/StepView";
import type { Locale } from "../../i18n";
import { stepsEn } from "../../i18n/steps.en";
import { stepsZhCN } from "../../i18n/steps.zh-CN";
import { Katex } from "../output/Katex";
const textFields = new Set([
  "verification",
  "method",
  "reason",
  "formula",
  "sign",
  "op",
  "order",
  "signs",
  "label",
  "symbol",
  "tag",
  "text",
  "level",
]);
/** Substitute actual params only, retaining remaining evidence below the title. */
export function StepTitle({
  step,
  language,
}: {
  step: StepView;
  language: Locale;
}) {
  const template =
    (language === "en" ? stepsEn : stepsZhCN)[step.title_key] ?? step.rule_id;
  const parts = template.split(/(\{[a-z_]+\})/);
  return (
    <span>
      {parts.map((part, i) => {
        const key = /^\{([a-z_]+)\}$/.exec(part)?.[1];
        if (!key) return part;
        const value = step.params[key];
        if (value === undefined) return null;
        return textFields.has(key) ? (
          <span key={i}>{value}</span>
        ) : (
          <Katex key={i} latex={value} />
        );
      })}
    </span>
  );
}
export function StepParams({
  step,
  language,
}: {
  step: StepView;
  language: Locale;
}) {
  return (
    <dl className="step-params">
      {Object.entries(step.params).map(([key, value]) => (
        <div key={key}>
          <dt>{paramLabel(key, language)}</dt>
          <dd>{textFields.has(key) ? value : <Katex latex={value} />}</dd>
        </div>
      ))}
    </dl>
  );
}
function paramLabel(key: string, language: Locale) {
  const labels: Record<string, [string, string]> = {
    factor: ["factor", "因子"],
    factors: ["factors", "因子"],
    parts: ["parts", "分解"],
    new_var: ["variable", "新变量"],
    def: ["definition", "定义"],
    var: ["variable", "变量"],
    value: ["value", "值"],
    results: ["results", "结果"],
    cond: ["condition", "条件"],
    solution: ["candidate", "候选解"],
    excluded: ["excluded", "已排除"],
    digits: ["digits", "位数"],
    matrix: ["matrix", "矩阵"],
    basis: ["basis", "基"],
    branches: ["branches", "分支"],
    constants: ["parameters", "参数"],
    verification: ["verification", "验证"],
    method: ["method", "方法"],
    reason: ["reason", "原因"],
    formula: ["formula", "公式"],
    func: ["function", "函数"],
    result: ["result", "结果"],
    points: ["points", "端点"],
    signs: ["signs", "符号"],
    label: ["branch", "分支"],
    text: ["message", "消息"],
  };
  return labels[key]?.[language === "en" ? 0 : 1] ?? key;
}
