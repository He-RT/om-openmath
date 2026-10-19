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
  reset: "M4 10a8 8 0 1 1 1 8 M4 4v6h6",
  expand: "M9 4H4v5 M15 4h5v5 M4 15v5h5 M20 15v5h-5",
  send: "M12 20V4 M5 11l7-7 7 7",
  history: "M4 9a8 8 0 1 1 0 7 M4 3v6h6 M12 8v5l3 2",
  new: "M13 4H4v16h16v-9 M9 15l1-5L19 1l4 4-9 9z",
  book: "M12 5v15 M12 5C9 3 5 3 3 4v14c3-1 6-1 9 2 3-3 6-3 9-2V4c-3-1-7-1-9 1z",
  search: "M16 16l5 5 M18 10a8 8 0 1 1-16 0 8 8 0 0 1 16 0z",
  eye: "M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12z M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0z",
  pencil: "M4 20l1-5L17 3l4 4L9 19z M14 6l4 4",
  cube: "M12 2l9 5v10l-9 5-9-5V7z M3 7l9 5 9-5 M12 12v10 M7 4.8l9 5",
  undo: "M8 5l-5 5 5 5 M3 10h11a6 6 0 0 1 0 12",
} as const;
export function WorkbenchIcon({ name }: { name: keyof typeof paths }) {
  return <svg className="workbench-icon" viewBox="0 0 24 24" aria-hidden="true" fill={name === "play" || name === "stop" ? "currentColor" : "none"} stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round"><path d={paths[name]} /></svg>;
}
