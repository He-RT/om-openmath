import type { NotebookFile } from "../kernel/generated/NotebookFile";
export function decodeNotebook(text: string): NotebookFile {
  const file = JSON.parse(text) as Partial<NotebookFile>;
  if (
    file.version !== 1 ||
    typeof file.title !== "string" ||
    !Array.isArray(file.cells)
  )
    throw new Error("Invalid notebook file");
  const ids = new Set<string>();
  const cells = file.cells.map((c) => {
    if (
      !c ||
      typeof c.id !== "string" ||
      !c.id ||
      ids.has(c.id) ||
      typeof c.source !== "string" ||
      !["Math", "Text", "Ask"].includes(c.kind) ||
      !["Modern", "Wolfram", "Auto"].includes(c.dialect)
    )
      throw new Error("Invalid notebook cell");
    ids.add(c.id);
    return { id: c.id, kind: c.kind, source: c.source, dialect: c.dialect };
  });
  return { version: 1, title: file.title, cells };
}
export async function saveNotebook(
  file: NotebookFile,
  kind: "wasm" | "tauri",
): Promise<boolean> {
  const name = `${(file.title || "OpenMath").replace(/[/\\:*?"<>|]/g, "-")}.omnb`;
  const text = JSON.stringify(
    {
      version: 1,
      title: file.title,
      cells: file.cells.map(({ id, kind, source, dialect }) => ({
        id,
        kind,
        source,
        dialect,
      })),
    },
    null,
    2,
  );
  if (kind === "tauri") {
    const { save } = await import("@tauri-apps/plugin-dialog");
    const { writeTextFile } = await import("@tauri-apps/plugin-fs");
    const path = await save({
      defaultPath: name,
      filters: [{ name: "OpenMath", extensions: ["omnb"] }],
    });
    if (!path) return false;
    await writeTextFile(path, text);
    return true;
  }
  const url = URL.createObjectURL(
    new Blob([text], { type: "application/json" }),
  );
  const link = document.createElement("a");
  link.href = url;
  link.download = name;
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 0);
  return true;
}
export async function openNativeNotebook(): Promise<NotebookFile | null> {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const { readTextFile } = await import("@tauri-apps/plugin-fs");
  const path = await open({
    multiple: false,
    filters: [{ name: "OpenMath", extensions: ["omnb"] }],
  });
  return typeof path === "string"
    ? decodeNotebook(await readTextFile(path))
    : null;
}
