const paths = {
  outline: "M3 4h18v16H3z M9 4v16 M5 8h2 M5 11h2",
  inspector: "M3 4h18v16H3z M15 4v16",
  plus: "M12 4v16 M4 12h16",
  chevron: "M7 9l5 5 5-5",
  play: "M8 5l11 7-11 7z",
  stop: "M6 6h12v12H6z",
  more: "M5 12h.01 M12 12h.01 M19 12h.01",
  close: "M6 6l12 12 M18 6L6 18",
  chat: "M4 4h16v12H9l-5 4z",
} as const;
export function WorkbenchIcon({ name }: { name: keyof typeof paths }) {
  return <svg className="workbench-icon" viewBox="0 0 24 24" aria-hidden="true" fill={name === "play" || name === "stop" ? "currentColor" : "none"} stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round"><path d={paths[name]} /></svg>;
}
