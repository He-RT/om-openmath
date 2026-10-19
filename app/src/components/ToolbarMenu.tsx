import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type ReactNode } from "react";
export function ToolbarMenu({ label, children, icon, className = "", menuRole='group' }: { label: string; children: ReactNode; icon: ReactNode; className?: string;menuRole?:'group'|'menu' }) {
  const root = useRef<HTMLDetailsElement>(null);
  const items=useRef<HTMLDivElement>(null);
  const [expanded, setExpanded] = useState(false);
  const [placement,setPlacement]=useState<CSSProperties>({});
  useLayoutEffect(()=>{
    if(!expanded) return;
    const measure=()=>{
      const anchor=root.current?.querySelector("summary")?.getBoundingClientRect(),content=items.current;
      if(!anchor || !content) return;
      const width=Math.min(230,innerWidth-24),height=Math.min(content.scrollHeight,innerHeight-24);
      setPlacement({position:"fixed",right:"auto",left:Math.max(12,Math.min(anchor.right-width,innerWidth-width-12)),top:anchor.bottom+4+height>innerHeight-12 ? Math.max(12,anchor.top-height-4):anchor.bottom+4,width,maxHeight:innerHeight-24,zIndex:40});
    };
    measure();window.addEventListener("resize",measure);document.addEventListener("scroll",measure,true);
    return ()=>{window.removeEventListener("resize",measure);document.removeEventListener("scroll",measure,true);};
  },[expanded]);
  useEffect(() => {
    const outside = (event: PointerEvent) => { if (root.current && !root.current.contains(event.target as Node)) root.current.open = false; };
    document.addEventListener("pointerdown", outside);
    return () => document.removeEventListener("pointerdown", outside);
  }, []);
  return <details className={`toolbar-menu ${className}`} ref={root} onToggle={event => setExpanded(event.currentTarget.open)} onKeyDown={(event) => {
    const menu = root.current;
    if (!menu) return;
    if ((event.target as HTMLElement).closest("select,input,textarea")) return;
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
    <div className="toolbar-menu-items" ref={items} style={placement} hidden={!expanded} role={menuRole} aria-label={label} onClick={(event) => {
      if ((event.target as HTMLElement).closest("button")) { if (root.current) {root.current.open = false;root.current.querySelector("summary")?.focus();} }
    }}>{children}</div>
  </details>;
}
