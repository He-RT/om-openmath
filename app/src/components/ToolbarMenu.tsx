import { useEffect, useRef, useState, type ReactNode } from "react";
export function ToolbarMenu({ label, children, icon, className = "" }: { label: string; children: ReactNode; icon: ReactNode; className?: string }) {
  const root = useRef<HTMLDetailsElement>(null);
  const [expanded, setExpanded] = useState(false);
  useEffect(() => {
    const outside = (event: PointerEvent) => { if (root.current && !root.current.contains(event.target as Node)) root.current.open = false; };
    document.addEventListener("pointerdown", outside);
    return () => document.removeEventListener("pointerdown", outside);
  }, []);
  return <details className={`toolbar-menu ${className}`} ref={root} onToggle={event => setExpanded(event.currentTarget.open)} onKeyDown={(event) => {
    const menu = root.current;
    if (!menu) return;
    if (event.key === "Escape") { event.preventDefault(); menu.open = false; setExpanded(false); menu.querySelector("summary")?.focus(); }
    if (["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) {
      event.preventDefault(); menu.open = true; setExpanded(true);
      const key=event.key;
      requestAnimationFrame(() => {
        if (!menu.open) return;
        const buttons = [...menu.querySelectorAll<HTMLButtonElement>("button:not(:disabled)")].filter(button => button.getClientRects().length > 0);
        const current = buttons.indexOf(document.activeElement as HTMLButtonElement);
        const index = key === "Home" ? 0 : key === "End" ? buttons.length - 1 : key === "ArrowDown" ? (current + 1) % buttons.length : (current <= 0 ? buttons.length - 1 : current - 1);
        buttons[index]?.focus();
      });
    }
  }}>
    <summary role="button" aria-expanded={expanded} aria-label={label} title={label}>{icon}</summary>
    <div className="toolbar-menu-items" hidden={!expanded} role="group" aria-label={label} onClick={(event) => {
      if ((event.target as HTMLElement).closest("button")) { if (root.current) {root.current.open = false;root.current.querySelector("summary")?.focus();} }
    }}>{children}</div>
  </details>;
}
