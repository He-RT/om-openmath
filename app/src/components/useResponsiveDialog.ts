import { useLayoutEffect, useRef, type RefObject } from "react";
/** Keep one DOM owner: desktop visibility, native modal focus/inert on small screens. */
export function useResponsiveDialog(root: RefObject<HTMLDialogElement | null>, visible: boolean, modal: boolean) {
  const wasModal = useRef(false);
  useLayoutEffect(() => {
    const element = root.current;
    if (!element) return;
    if (wasModal.current && element.open) element.close?.();
    wasModal.current = false;
    element.open = false;
    if (!visible) return;
    if (modal && element.showModal) { element.showModal(); wasModal.current = true; }
    else element.open = true;
    return () => { if (wasModal.current && element.open) element.close?.(); };
  }, [root, visible, modal]);
}
