import { useEffect, useRef, useState } from "react";
import type { KernelClient, Request } from "../../kernel/client";
import type { ChatMessage } from "../../kernel/generated/ChatMessage";
import type { LlmTaskView } from "../../state/assistant";
export type LlmAction =
  | { type: "llm_translate"; text: string; cell_id: string | null }
  | { type: "llm_fix_error"; cell_id: string }
  | { type: "llm_chat"; messages: ChatMessage[] };
export function useLlmTask(
  kernel: KernelClient,
  scope: string,
  fallback: string,
) {
  const [value, setValue] = useState<LlmTaskView | null>(null);
  const state = useRef({
    alive: false,
    id: null as string | null,
    pending: new Set<string>(),
    cancelled: new Set<string>(),
  });
  const cancel = () => {
    const current = state.current;
    if (!current.id) return;
    const request_id = current.id;
    current.id = null;
    if (current.pending.has(request_id)) current.cancelled.add(request_id);
    void kernel.request({ type: "llm_cancel", request_id }).catch(() => {});
    if (current.alive)
      setValue((old) => (old ? { ...old, status: "cancelled" } : null));
  };
  useEffect(() => {
    const current = state.current;
    current.alive = true;
    setValue(null);
    const off = kernel.onEvent((event) => {
      if (!current.alive) return;
      if (event.type === "kernel_restarted") {
        cancel();
        return;
      }
      if (!("request_id" in event) || event.request_id !== current.id) return;
      setValue((old) => {
        if (!old) return old;
        if (event.type === "llm_delta")
          return { ...old, text: old.text + event.text };
        if (event.type === "llm_suggestion")
          return {
            ...old,
            suggestions: [...old.suggestions, event.suggestion],
          };
        if (event.type === "llm_tool_call")
          return {
            ...old,
            tools: [
              ...old.tools,
              {
                name: event.name,
                arguments: event.arguments,
                result: event.result_summary,
              },
            ],
          };
        if (event.type === "llm_done") return { ...old, status: "done" };
        if (event.type === "llm_error")
          return { ...old, status: "error", error: event.message };
        return old;
      });
      if (event.type === "llm_done" || event.type === "llm_error")
        current.id = null;
    });
    return () => {
      cancel();
      current.alive = false;
      off();
    };
  }, [kernel, scope]);
  const start = (action: LlmAction) => {
    const current = state.current;
    if (!current.alive) return;
    cancel();
    const id = crypto.randomUUID();
    current.id = id;
    current.pending.add(id);
    setValue({ id, text: "", suggestions: [], tools: [], status: "streaming" });
    void kernel.request({ ...action, request_id: id } as Request).then(
      (response) => {
        current.pending.delete(id);
        if (current.cancelled.delete(id)) {
          if (response.type === "llm_started")
            void kernel
              .request({ type: "llm_cancel", request_id: id })
              .catch(() => {});
          return;
        }
        if (!current.alive || current.id !== id) return;
        if (response.type !== "llm_started" || response.request_id !== id) {
          current.id = null;
          setValue((old) =>
            old
              ? {
                  ...old,
                  status: "error",
                  error:
                    response.type === "error" ? response.message : fallback,
                }
              : null,
          );
        }
      },
      () => {
        current.pending.delete(id);
        current.cancelled.delete(id);
        if (current.alive && current.id === id) {
          current.id = null;
          setValue((old) =>
            old ? { ...old, status: "error", error: fallback } : null,
          );
        }
      },
    );
  };
  return { value, start, cancel };
}
