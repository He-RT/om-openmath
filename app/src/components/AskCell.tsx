import { useEffect, useRef, useState } from "react";
import type { NotebookController, UiCell } from "../state/notebookStore";
import type { Messages } from "../i18n";
import { featureProfile, configScope } from "./assistant/profile";
import { useLlmTask } from "./assistant/useLlmTask";
import { PrivacyPrompt, hasConsent } from "./assistant/PrivacyPrompt";
import { SuggestionCard } from "./assistant/SuggestionCard";
import { TaskState } from "./assistant/TaskState";
export function AskCell({
  cell,
  index,
  controller,
  t,
  busy,
}: {
  cell: UiCell;
  index: number;
  controller: NotebookController;
  t: Messages;
  busy: boolean;
}) {
  const state = controller.store.getState(),
    profile = featureProfile(state.config?.llm, "translate");
  const task = useLlmTask(
    controller.kernel,
    `${state.documentGeneration}:${cell.id}:${cell.revision}:${configScope(state.config?.llm)}`,
    t.aiRequestError,
  );
  const [privacy, setPrivacy] = useState(false),
    dialect =
      state.config?.general.dialect === "wolfram" ? "Wolfram" : "Modern";
  useEffect(
    () => setPrivacy(false),
    [state.documentGeneration, cell.revision, profile],
  );
  const sending = useRef(false);
  const send = async () => {
    if (sending.current) return;
    sending.current = true;
    setPrivacy(false);
    await controller.flush();
    sending.current = false;
    const current = controller.store
      .getState()
      .cells.find((c) => c.id === cell.id);
    if (
      configScope(controller.store.getState().config?.llm) !==
        configScope(state.config?.llm) ||
      controller.store.getState().documentGeneration !==
        state.documentGeneration ||
      current?.revision !== cell.revision ||
      current.source !== cell.source ||
      current.kind !== "Ask"
    )
      return;
    task.start({ type: "llm_translate", text: cell.source, cell_id: cell.id });
  };
  const request = () => {
    if (
      !profile ||
      busy ||
      !cell.source.trim() ||
      task.value?.status === "streaming" ||
      sending.current
    )
      return;
    if (hasConsent(controller.kernel, profile, "translate:names")) void send();
    else setPrivacy(true);
  };
  const insert = (source: string, run = false) => {
    const id = controller.add("Math", source, cell.id);
    controller.edit(id, { dialect });
    if (run) void controller.run(id);
    else controller.select(id, true);
  };
  return (
    <div className="ask-cell">
      <textarea
        className="prose-editor"
        aria-label={`${t.question} ${index + 1}`}
        value={cell.source}
        placeholder={t.question}
        onFocus={() => controller.select(cell.id)}
        onChange={(e) => controller.edit(cell.id, { source: e.target.value })}
        onKeyDown={(e) => {
          if (e.key === "Enter" && !e.shiftKey && !e.nativeEvent.isComposing) {
            e.preventDefault();
            request();
          }
        }}
      />
      <div className="ask-actions">
        <button
          disabled={
            !profile ||
            busy ||
            !cell.source.trim() ||
            task.value?.status === "streaming"
          }
          onClick={request}
        >
          {t.askAI}
        </button>
        {!profile && <span className="muted">{t.configureAI}</span>}
      </div>
      <TaskState value={task.value} t={t} onCancel={task.cancel} />
      {task.value?.suggestions.map((suggestion, i) => (
        <SuggestionCard
          key={i}
          suggestion={suggestion}
          dialect={dialect}
          t={t}
          onInsert={(source) => insert(source)}
          onRun={(source) => insert(source, true)}
          onEdit={(source) => {
            controller.edit(cell.id, { source, kind: "Math", dialect });
            controller.select(cell.id, true);
          }}
        />
      ))}
      {privacy && profile && (
        <PrivacyPrompt
          kernel={controller.kernel}
          profile={profile}
          scope="translate:names"
          disclosure={t.translatePrivacy}
          t={t}
          onClose={() => setPrivacy(false)}
          onSend={() => void send()}
        />
      )}
    </div>
  );
}
