import { useEffect, useRef } from "react";
export function useDesktopMenu(kind: "wasm" | "tauri", actions: Record<string, () => void>, onError: () => void) {
  const current = useRef({ actions, onError });
  current.current = { actions, onError };
  useEffect(() => {
    if (kind !== "tauri") return;
    let active = true;
    let dispose: (() => void) | undefined;
    void import("@tauri-apps/api/event").then(({ listen }) =>
      listen<string>("openmath:menu", (event) => {
        if (active) current.current.actions[event.payload]?.();
      }),
    ).then((stop) => { if (active) dispose = stop; else stop(); })
      .catch(() => { if (active) current.current.onError(); });
    return () => { active = false; dispose?.(); };
  }, [kind]);
}
