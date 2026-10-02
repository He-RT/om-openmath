import { useEffect, useRef, useState } from "react";
import type { KernelClient } from "../../kernel/client";
import type { PlotData } from "../../kernel/generated/PlotData";
import type { PlotRequest } from "../../kernel/generated/PlotRequest";
/** Serial actual sampling with revision checks and one coalesced latest tail. */
export function usePlotSampler(
  kernel: KernelClient,
  request: PlotRequest,
  initial: PlotData | undefined,
  active: boolean,
  fallback: string,
  onData: (data: PlotData) => void,
) {
  const [data, setData] = useState(initial ?? null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const owner = useRef({
    alive: false,
    revision: 0,
    inFlight: false,
    timer: undefined as ReturnType<typeof setTimeout> | undefined,
    queued: null as { request: PlotRequest; revision: number } | null,
    lastStart: 0,
  });
  const accepted = useRef(onData);
  accepted.current = onData;
  const pump = () => {
    const state = owner.current;
    if (!state.alive || state.inFlight || state.timer || !state.queued) return;
    const job = state.queued;
    state.queued = null;
    state.inFlight = true;
    state.lastStart = Date.now();
    void kernel
      .request({ type: "sample_plot", request: job.request })
      .then(
        (response) => {
          if (!state.alive || job.revision !== state.revision) return;
          if (response.type === "plot") {
            setData(response.data);
            setError(null);
            accepted.current(response.data);
          } else
            setError(response.type === "error" ? response.message : fallback);
        },
        () => {
          if (state.alive && job.revision === state.revision)
            setError(fallback);
        },
      )
      .finally(() => {
        state.inFlight = false;
        if (!state.alive) return;
        if (!state.queued && !state.timer) setPending(false);
        pump();
      });
  };
  const sample = (
    next: PlotRequest,
    mode: "view" | "parameter" | "now" = "view",
  ) => {
    const state = owner.current;
    if (!state.alive || !active) return;
    state.queued = { request: next, revision: ++state.revision };
    setPending(true);
    setError(null);
    if (state.timer) clearTimeout(state.timer);
    const delay =
      mode === "view"
        ? 150
        : mode === "parameter"
          ? Math.max(0, 33 - (Date.now() - state.lastStart))
          : 0;
    state.timer = setTimeout(() => {
      state.timer = undefined;
      pump();
    }, delay);
  };
  useEffect(() => {
    const state = owner.current;
    state.alive = true;
    if (active && !initial) sample(request, "now");
    const off = kernel.onEvent((e) => {
      if (e.type === "kernel_restarted") {
        state.revision++;
        state.queued = null;
        if (state.timer) clearTimeout(state.timer);
        state.timer = undefined;
        setPending(false);
        setError(fallback);
      }
    });
    return () => {
      state.alive = false;
      state.revision++;
      state.queued = null;
      if (state.timer) clearTimeout(state.timer);
      off();
    };
    // Each source/active scope owns a freshly keyed component; all mutable state lives here.
  }, [kernel]);
  return { data, pending, error, sample };
}
