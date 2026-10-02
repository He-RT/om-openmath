import { useEffect, useRef, useState } from "react";
import type { KernelClient } from "../../kernel/client";
export interface Explanation {
  text: string;
  status: "streaming" | "done" | "cancelled" | "error";
  error?: string;
}
export function useExplanation(
  kernel: KernelClient,
  cellId: string,
  outIndex: number,
  fallbackError: string,
) {
  const [items, setItems] = useState<Record<string, Explanation>>({});
  const jobs = useRef(new Map<string, string>());
  const active = useRef(false);
  const cancelled = useRef(new Set<string>());
  const pendingStarts = useRef(new Set<string>());
  useEffect(() => {
    active.current = true;
    const update = (key: string, patch: Partial<Explanation>) =>
      setItems((old) => ({
        ...old,
        [key]: { ...(old[key] ?? { text: "", status: "streaming" }), ...patch },
      }));
    const off = kernel.onEvent((event) => {
      if (!active.current) return;
      if (event.type === "kernel_restarted") {
        for (const [id, key] of jobs.current) {
          if (pendingStarts.current.has(id)) cancelled.current.add(id);
          update(key, { status: "cancelled" });
        }
        jobs.current.clear();
        return;
      }
      if (!("request_id" in event)) return;
      const key = jobs.current.get(event.request_id);
      if (key === undefined) return;
      if (event.type === "llm_delta")
        setItems((old) => ({
          ...old,
          [key]: {
            text: (old[key]?.text ?? "") + event.text,
            status: "streaming",
          },
        }));
      if (event.type === "llm_done") {
        jobs.current.delete(event.request_id);
        update(key, { status: "done" });
      }
      if (event.type === "llm_error") {
        jobs.current.delete(event.request_id);
        update(key, { status: "error", error: event.message });
      }
    });
    const owned = jobs.current;
    return () => {
      active.current = false;
      off();
      for (const request_id of owned.keys()) {
        if (pendingStarts.current.has(request_id))
          cancelled.current.add(request_id);
        void kernel.request({ type: "llm_cancel", request_id }).catch(() => {});
      }
      owned.clear();
    };
  }, [kernel, cellId, outIndex]);
  const cancel = (key: string) => {
    for (const [id, target] of jobs.current)
      if (target === key) {
        jobs.current.delete(id);
        if (pendingStarts.current.has(id)) cancelled.current.add(id);
        void kernel
          .request({ type: "llm_cancel", request_id: id })
          .catch(() => {});
      }
    setItems((old) => ({
      ...old,
      [key]: { text: old[key]?.text ?? "", status: "cancelled" },
    }));
  };
  const start = (stepId: string | null) => {
    if (!active.current) return;
    const key = stepId ?? "all";
    cancel(key);
    const id = crypto.randomUUID();
    pendingStarts.current.add(id);
    jobs.current.set(id, key);
    setItems((old) => ({ ...old, [key]: { text: "", status: "streaming" } }));
    void kernel
      .request({
        type: "llm_explain",
        request_id: id,
        cell_id: cellId,
        step_id: stepId,
        out_index: outIndex,
      })
      .then(
        (response) => {
          pendingStarts.current.delete(id);
          if (cancelled.current.delete(id)) {
            if (response.type === "llm_started")
              void kernel
                .request({ type: "llm_cancel", request_id: id })
                .catch(() => {});
            return;
          }
          if (!active.current || !jobs.current.has(id)) return;
          if (response.type !== "llm_started" || response.request_id !== id) {
            jobs.current.delete(id);
            setItems((old) => ({
              ...old,
              [key]: {
                text: old[key]?.text ?? "",
                status: "error",
                error:
                  response.type === "error" ? response.message : fallbackError,
              },
            }));
          }
        },
        () => {
          pendingStarts.current.delete(id);
          cancelled.current.delete(id);
          if (!active.current || !jobs.current.has(id)) return;
          jobs.current.delete(id);
          setItems((old) => ({
            ...old,
            [key]: {
              text: old[key]?.text ?? "",
              status: "error",
              error: fallbackError,
            },
          }));
        },
      );
  };
  return { items, start, cancel };
}
