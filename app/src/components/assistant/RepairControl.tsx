import { useState } from "react";
import type { NotebookController, UiCell } from "../../state/notebookStore";
import type { Messages } from "../../i18n";
import { featureProfile, configScope } from "./profile";
import { useLlmTask } from "./useLlmTask";
import { PrivacyPrompt, hasConsent } from "./PrivacyPrompt";
import { suggestionSource } from "./SuggestionCard";
import { TaskState } from "./TaskState";
export function RepairControl({
  cell,
  controller,
  t,
  busy,
}: {
  cell: UiCell;
  controller: NotebookController;
  t: Messages;
  busy: boolean;
}) {
  const state = controller.store.getState(),
    profile = featureProfile(state.config?.llm, "fix");
  const task = useLlmTask(
    controller.kernel,
    `${state.documentGeneration}:${cell.id}:${cell.revision}:${configScope(state.config?.llm)}`,
    t.aiRequestError,
  );
  const [privacy, setPrivacy] = useState(false),
    dialect =
      cell.dialect === "Wolfram" || state.config?.general.dialect === "wolfram"
        ? "Wolfram"
        : "Modern";
  const send = async () => {
    setPrivacy(false);
    await controller.flush();
    if (!valid()) return;
    task.start({ type: "llm_fix_error", cell_id: cell.id });
  };
  const valid = () => {
    const current = controller.store
      .getState()
      .cells.find((c) => c.id === cell.id);
    return (
      controller.store.getState().documentGeneration ===
        state.documentGeneration &&
      configScope(controller.store.getState().config?.llm) ===
        configScope(state.config?.llm) &&
      current?.revision === cell.revision &&
      current.source === cell.source &&
      current.status === "Error"
    );
  };
  return (
    <div className="repair-control">
      <button
        disabled={!profile || busy || task.value?.status === "streaming"}
        onClick={() => {
          if (!profile || !valid()) return;
          if (hasConsent(controller.kernel, profile, "fix:diagnostics"))
            void send();
          else setPrivacy(true);
        }}
      >
        {t.fixAI}
      </button>
      <TaskState value={task.value} t={t} onCancel={task.cancel} />
      {task.value?.suggestions.map((s, i) => {
        const source = suggestionSource(s, dialect);
        return (
          <article className="repair-suggestion" key={i}>
            <p>{s.explanation}</p>
            <div className="repair-diff">
              <div>
                <span>{t.before}</span>
                <pre>{cell.source}</pre>
              </div>
              <div>
                <span>{t.after}</span>
                <pre>{source}</pre>
              </div>
            </div>
            <button
              disabled={busy}
              onClick={() => {
                if (valid()) {
                  controller.edit(cell.id, { source, dialect });
                  controller.select(cell.id, true);
                }
              }}
            >
              {t.apply}
            </button>
          </article>
        );
      })}
      {privacy && profile && (
        <PrivacyPrompt
          kernel={controller.kernel}
          profile={profile}
          scope="fix:diagnostics"
          disclosure={t.fixPrivacy}
          t={t}
          onClose={() => setPrivacy(false)}
          onSend={() => void send()}
        />
      )}
    </div>
  );
}
