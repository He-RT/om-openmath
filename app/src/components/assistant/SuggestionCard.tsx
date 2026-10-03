import type { Suggestion } from "../../kernel/generated/Suggestion";
import type { Dialect } from "../../kernel/generated/Dialect";
import type { Messages } from "../../i18n";
import { Katex } from "../output/Katex";
export function suggestionSource(suggestion: Suggestion, dialect: Dialect) {
  return dialect === "Wolfram" ? suggestion.wolfram : suggestion.modern;
}
export function SuggestionCard({
  suggestion,
  dialect,
  t,
  onInsert,
  onRun,
  onEdit,
}: {
  suggestion: Suggestion;
  dialect: Dialect;
  t: Messages;
  onInsert: (source: string) => void;
  onRun?: (source: string) => void;
  onEdit?: (source: string) => void;
}) {
  const source = suggestionSource(suggestion, dialect);
  return (
    <article className="suggestion-card">
      <span className="suggestion-label">
        {t.aiSuggestion} · {t.notExecuted}
      </span>
      <Katex display latex={suggestion.latex} />
      <pre>{source}</pre>
      <p>{suggestion.explanation}</p>
      <div className="suggestion-actions">
        <button onClick={() => onInsert(source)}>{t.insertCode}</button>
        {onRun && <button onClick={() => onRun(source)}>{t.run}</button>}
        {onEdit && <button onClick={() => onEdit(source)}>{t.edit}</button>}
      </div>
    </article>
  );
}
