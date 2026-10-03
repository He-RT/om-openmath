import { createStore } from "zustand/vanilla";
import type { KernelClient, Request } from "./client";
export function observeKernel(kernel: KernelClient) {
  const store = createStore<{ active: number; error: string | null }>(() => ({
    active: 0,
    error: null,
  }));
  const jobs = new Set<string>(),
    cancelled = new Set<string>();
  const stop = (id: string) => {
    jobs.delete(id);
    cancelled.add(id);
    if (cancelled.size > 256) {
      const first = cancelled.values().next().value;
      if (first) cancelled.delete(first);
    }
    store.setState({ active: jobs.size });
  };
  const cancelAll = () => {
    for (const id of [...jobs]) {
      stop(id);
      void kernel
        .request({ type: "llm_cancel", request_id: id })
        .catch(() => {});
    }
  };
  const off = kernel.onEvent((event) => {
    if (event.type === "kernel_restarted") {
      jobs.clear();
      store.setState({ active: 0, error: null });
      return;
    }
    if (!("request_id" in event) || !jobs.has(event.request_id)) return;
    if (event.type === "llm_done") {
      jobs.delete(event.request_id);
      store.setState({ active: jobs.size });
    }
    if (event.type === "llm_error") {
      jobs.delete(event.request_id);
      store.setState({ active: jobs.size, error: event.message });
    }
  });
  const client: KernelClient = {
    kind: kernel.kind,
    get ready() {
      return kernel.ready;
    },
    onEvent: (callback) => kernel.onEvent(callback),
    request: async (request: Request) => {
      const start =
        [
          "llm_translate",
          "llm_explain",
          "llm_complete",
          "llm_chat",
          "llm_fix_error",
          "llm_test_profile",
        ].includes(request.type) && "request_id" in request
          ? request.request_id
          : null;
      if (start) {
        cancelled.delete(start);
        jobs.add(start);
        store.setState({ active: jobs.size, error: null });
      }
      if (request.type === "llm_cancel") stop(request.request_id);
      if (request.type === "load_notebook") cancelAll();
      try {
        const response = await kernel.request(request);
        if (start && response.type === "error" && !cancelled.has(start)) {
          jobs.delete(start);
          store.setState({ active: jobs.size, error: response.message });
        }
        return response;
      } catch (error) {
        if (start && !cancelled.has(start)) {
          jobs.delete(start);
          store.setState({ active: jobs.size, error: "AI request failed" });
        }
        throw error;
      }
    },
    interrupt: async () => {
      cancelAll();
      await kernel.interrupt();
    },
    dispose: () => {
      cancelAll();
      off();
      kernel.dispose();
    },
  };
  return {
    client,
    store,
    detach: () => {
      cancelAll();
      off();
    },
  };
}
