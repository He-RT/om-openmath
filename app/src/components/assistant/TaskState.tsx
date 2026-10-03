import type { LlmTaskView } from "../../state/assistant";
import type { Messages } from "../../i18n";
export function TaskState({
  value,
  t,
  onCancel,
}: {
  value: LlmTaskView | null | undefined;
  t: Messages;
  onCancel: () => void;
}) {
  if (!value) return null;
  return (
    <>
      {value.status === "streaming" && (
        <div className="ai-working">
          <span className="ai-skeleton" />
          <span role="status">{t.aiWorking}</span>
          <button onClick={onCancel}>{t.cancel}</button>
        </div>
      )}
      {value.status === "error" && (
        <p role="alert" className="output-error">
          {value.error}
        </p>
      )}
      {value.status === "cancelled" && <p className="muted">{t.aiCancelled}</p>}
    </>
  );
}
