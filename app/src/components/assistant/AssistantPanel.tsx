import { useEffect, useRef, useState } from "react";
import type { NotebookController, UiState } from "../../state/notebookStore";
import type { Messages } from "../../i18n";
import type { LlmTaskView } from "../../state/assistant";
import { conversationHistory, expandReferences } from "../../state/assistant";
import { useLlmTask } from "./useLlmTask";
import { featureProfile, configScope } from "./profile";
import { hasConsent, PrivacyPrompt } from "./PrivacyPrompt";
import { TaskState } from "./TaskState";
import { SuggestionCard } from "./SuggestionCard";
import { ToolCallCard } from "./ToolCallCard";
import { Markdown } from "../TextCell";
export function AssistantPanel({
  state,
  controller,
  t,
}: {
  state: UiState;
  controller: NotebookController;
  t: Messages;
}) {
  const profile = featureProfile(state.config?.llm, "chat");
  const scope = `${state.documentGeneration}:${configScope(state.config?.llm)}`;
  const task = useLlmTask(controller.kernel, scope, t.aiRequestError);
  const [input, setInput] = useState(""),
    [error, setError] = useState<string | null>(null),
    [privacy, setPrivacy] = useState<{ input: string; sent: string } | null>(
      null,
    );
  const turn = useRef<string | null>(null),
    latest = useRef<LlmTaskView | null>(null);
  latest.current = task.value;
  const mounted = useRef(true),
    version = useRef(0),
    sending = useRef(false);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      version.current++;
    };
  }, []);
  const dialect =
    state.config?.general.dialect === "wolfram" ? "Wolfram" : "Modern";
  useEffect(() => {
    if (!task.value || !turn.current) return;
    const id = turn.current;
    controller.store.setState((s) => ({
      assistantTurns: s.assistantTurns.map((item) =>
        item.id === id ? { ...item, result: task.value! } : item,
      ),
    }));
  }, [task.value, controller]);
  useEffect(
    () => () => {
      const value = latest.current,
        id = turn.current;
      if (value?.status === "streaming" && id)
        controller.store.setState((s) => ({
          assistantTurns: s.assistantTurns.map((item) =>
            item.id === id
              ? { ...item, result: { ...value, status: "cancelled" } }
              : item,
          ),
        }));
    },
    [controller, scope],
  );
  const send = async (candidate: { input: string; sent: string }) => {
    if (sending.current) return;
    sending.current = true;
    const token = version.current;
    setPrivacy(null);
    setInput("");
    setError(null);
    await controller.flush();
    sending.current = false;
    const current = controller.store.getState(),
      id = crypto.randomUUID();
    if (
      !mounted.current ||
      token !== version.current ||
      current.documentGeneration !== state.documentGeneration ||
      configScope(current.config?.llm) !== configScope(state.config?.llm)
    )
      return;
    try {
      candidate = {
        ...candidate,
        sent: expandReferences(candidate.input, current.cells),
      };
    } catch {
      setError(t.referenceError);
      return;
    }
    turn.current = id;
    const messages = conversationHistory(current.assistantTurns);
    messages.push({
      role: "user",
      content: candidate.sent,
      tool_calls: [],
      tool_call_id: null,
    });
    controller.store.setState({
      assistantTurns: [
        ...current.assistantTurns,
        { id, input: candidate.input, sent: candidate.sent },
      ],
    });
    task.start({ type: "llm_chat", messages });
  };
  const request = () => {
    if (
      !profile ||
      state.busy ||
      !input.trim() ||
      task.value?.status === "streaming" ||
      sending.current
    )
      return;
    try {
      const candidate = { input, sent: expandReferences(input, state.cells) };
      if (hasConsent(controller.kernel, profile, "chat:references"))
        void send(candidate);
      else setPrivacy(candidate);
    } catch {
      setError(t.referenceError);
    }
  };
  const insert = (source: string) => {
    const id = controller.add("Math", source, state.active ?? undefined);
    controller.edit(id, { dialect });
    controller.select(id, true);
  };
  return (
    <section className="assistant-panel">
      <header className="assistant-header">
        <h2>{t.assistant}</h2>
        <button
          onClick={() => {
            task.cancel();
            version.current++;
            controller.store.setState({ assistantTurns: [] });
          }}
        >
          {t.newConversation}
        </button>
      </header>
      {state.assistantTurns.map((item) => (
        <article className="chat-turn" key={item.id}>
          <div className="chat-user">
            <span>{t.you}</span>
            <p>{item.input}</p>
          </div>
          {item.result && (
            <div className="chat-assistant">
              <span>{t.assistant}</span>
              {item.result.text && <Markdown source={item.result.text} />}
              <TaskState value={item.result} t={t} onCancel={task.cancel} />
              {item.result.tools.map((tool, i) => (
                <ToolCallCard key={i} tool={tool} t={t} />
              ))}
              {item.result.suggestions.map((s, i) => (
                <SuggestionCard
                  key={i}
                  suggestion={s}
                  dialect={dialect}
                  t={t}
                  onInsert={insert}
                />
              ))}
            </div>
          )}
        </article>
      ))}
      {!profile && <p className="muted">{t.configureAI}</p>}
      {error && (
        <p role="alert" className="output-error">
          {error}
        </p>
      )}
      <form
        className="chat-composer"
        onSubmit={(e) => {
          e.preventDefault();
          request();
        }}
      >
        <textarea
          aria-label={t.chatInput}
          placeholder={t.chatPlaceholder}
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => {
            if (
              e.key === "Enter" &&
              !e.shiftKey &&
              !e.nativeEvent.isComposing
            ) {
              e.preventDefault();
              request();
            }
          }}
        />
        <button
          disabled={
            !profile ||
            state.busy ||
            !input.trim() ||
            task.value?.status === "streaming"
          }
          type="submit"
        >
          {t.send}
        </button>
      </form>
      {privacy && profile && (
        <PrivacyPrompt
          kernel={controller.kernel}
          profile={profile}
          scope="chat:references"
          disclosure={t.chatPrivacy}
          t={t}
          onClose={() => setPrivacy(null)}
          onSend={() => void send(privacy)}
        />
      )}
    </section>
  );
}
