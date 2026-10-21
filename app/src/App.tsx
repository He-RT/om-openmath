import { useEffect, useRef, useState } from "react";
import { useStore } from "zustand";
import type { KernelClient } from "./kernel/client";
import { useKernel } from "./kernel/provider";
import { NotebookController } from "./state/notebookStore";
import {
  decodeNotebook,
  saveNotebook,
  openNativeNotebook,
} from "./state/files";
import { locale, messages } from "./i18n";
import { TopBar } from "./components/TopBar";
import { Notebook } from "./components/Notebook";
import { Inspector } from "./components/Inspector";
import { Outline } from "./components/Outline";
import { FunctionCatalog } from "./components/FunctionCatalog";
import { WorkbenchIcon } from "./components/WorkbenchIcon";
import { useMediaQuery } from "./components/useMediaQuery";
import { Preferences } from "./components/Preferences";
import { CommandPalette } from "./components/CommandPalette";
import { exportNotebook, type ExportFormat } from "./state/export";
import { useDesktopMenu } from "./state/desktopMenu";
import { hasNativeFiles } from './platform';
export default function App({ kernel }: { kernel?: KernelClient }) {
  const provided = useKernel();
  const client = kernel ?? provided.client;
  const [controller, setController] = useState<NotebookController | null>(null);
  useEffect(() => {
    if (!client) return;
    const next = new NotebookController(client);
    setController(next);
    void next.initialize();
    return () => next.dispose();
  }, [client]);
  const t = messages(locale());
  if (!controller)
    return (
      <div className="kernel-loading">
        <strong>OpenMath</strong>
        <p role="status">{provided.error ? t.kernelError : t.loading}</p>
      </div>
    );
  return <Workspace controller={controller} />;
}
function Workspace({ controller }: { controller: NotebookController }) {
  const state = useStore(controller.store);
  const language = locale(state.config?.general.language);
  const t = messages(language);
  const [commands, setCommands] = useState(false);
  const [catalog, setCatalog] = useState(false);
  const wide = useMediaQuery("(min-width: 1100px)"), phone = useMediaQuery("(width < 760px)");
  const [outlinePinned, setOutlinePinned] = useState(true), [outlineOverlay, setOutlineOverlay] = useState(false);
  const outlineVisible = wide ? outlinePinned : outlineOverlay;
  const lastPanel = useRef<NonNullable<typeof state.panel>>("assistant");
  useEffect(() => { if (state.panel) lastPanel.current = state.panel; }, [state.panel]);
  const toggleOutline = () => wide ? setOutlinePinned(value => !value) : setOutlineOverlay(value => !value);
  const toggleInspector = () => controller.store.setState({ panel: state.panel ? null : lastPanel.current });
  const openCatalog = () => { if (phone) controller.store.setState({ panel: null }); setOutlineOverlay(false); setCatalog(true); };
  const openCommands = () => { if (phone) controller.store.setState({ panel: null }); setOutlineOverlay(false); setCommands(true); };
  const openSettings = (ai = false) => { if (phone) controller.store.setState({ panel: null }); setOutlineOverlay(false); setPreferences(ai ? "models" : "general"); };
  const [preferences, setPreferences] = useState<"general" | "models" | null>(
    null,
  );
  const fileInput = useRef<HTMLInputElement>(null);
  const [theme, setTheme] = useState(() => {
    try {
      return localStorage.getItem("openmath-theme") ?? "system";
    } catch {
      return "system";
    }
  });
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);
  const save = () => {
    void controller
      .flush()
      .then(async () => {
        const file = controller.file();
        return {
          file,
          saved: await saveNotebook(file, controller.kernel.kind),
        };
      })
      .then(({ file, saved }) => {
        if (saved)
          controller.markSaved(
            controller.kernel.kind === "wasm" ? t.saved : t.fileSaved,
            file,
          );
      })
      .catch(() => controller.store.setState({ error: t.fileError }));
  };
  const open = () => {
    if (!hasNativeFiles(controller.kernel.kind)) {
      fileInput.current?.click();
      return;
    }
    void openNativeNotebook()
      .then((file) => (file ? controller.load(file) : undefined))
      .catch(() => controller.store.setState({ error: t.fileError }));
  };
  const newNotebook = () =>
    void controller
      .load({ version: 1, title: "", cells: [] })
      .catch((error) => controller.store.setState({ error: String(error) }));
  const exportFile = (format: ExportFormat) => {
    void controller.flush().then(async () => {
      const current = controller.store.getState();
      if (await exportNotebook(current.title, current.cells, format, controller.kernel.kind))
        controller.store.setState({ notice: controller.kernel.kind === "wasm" ? t.exported : t.fileSaved });
    }).catch(() => controller.store.setState({ error: t.fileError }));
  };
  const changeTheme = (next: string) => {
    setTheme(next);
    try { localStorage.setItem("openmath-theme", next); } catch { /* Keep theme in memory. */ }
  };
  useDesktopMenu(controller.kernel.kind, {
    "om-new": newNotebook, "om-open": open, "om-save": save, "om-save-as": save,
    "om-export-md": () => exportFile("markdown"), "om-export-tex": () => exportFile("latex"),
    "om-settings": () => openSettings(), "om-commands": openCommands,
    "om-docs": openCatalog,
    "om-assistant": () => controller.store.setState({ panel: "assistant" }),
    "om-panel": toggleInspector,
    "om-theme-light": () => changeTheme("light"), "om-theme-dark": () => changeTheme("dark"), "om-theme-system": () => changeTheme("system"),
  }, () => controller.store.setState({ error: t.fileError }));
  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if (!event.metaKey && !event.ctrlKey) return;
      const key = event.key.toLowerCase();
      if (key === "k") {
        event.preventDefault();
        if (commands) setCommands(false); else openCommands();
      }
      if (key === "1" && !event.altKey) { event.preventDefault(); toggleOutline(); }
      if (event.altKey && key === "0") { event.preventDefault(); toggleInspector(); }
      if (event.altKey && ["1", "2", "3"].includes(key)) { event.preventDefault(); controller.store.setState({ panel: (["assistant", "steps", "variables"] as const)[Number(key) - 1]! }); }
      if (key === "s") {
        event.preventDefault();
        save();
      }
      if (key === ".") {
        event.preventDefault();
        void controller.stopWork();
      }
      if (key === "z" && !event.defaultPrevented && !(event.target instanceof Element && event.target.closest('input,textarea,[contenteditable="true"],.cm-editor'))) {
        event.preventDefault(); controller.undoDocument(event.shiftKey);
      }
      if (event.shiftKey && state.active) {
        if (key === "arrowup" || key === "arrowdown") {
          event.preventDefault();
          controller.move(state.active, key === "arrowup" ? -1 : 1);
        }
        if (key === "d") {
          event.preventDefault();
          controller.remove(state.active);
        }
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  });
  if (state.initializing)
    return (
      <div className="kernel-loading">
        <strong>OpenMath</strong>
        <p role="status">{t.loading}</p>
      </div>
    );
  if (!state.config)
    return (
      <div className="kernel-loading">
        <strong>OpenMath</strong>
        <p role="alert">{state.error ?? t.kernelError}</p>
        <button onClick={() => void controller.initialize()}>{t.retry}</button>
      </div>
    );
  return (
    <div className={`app-shell ${state.panel ? "with-panel" : ""} ${wide && outlinePinned ? "with-outline" : ""}`}>
      <Outline state={state} controller={controller} t={t} visible={outlineVisible} overlay={!wide} onClose={() => wide ? setOutlinePinned(false) : setOutlineOverlay(false)} />
      <div className="workbench-main">
      <TopBar
        state={state}
        controller={controller}
        t={t}
        onSave={save}
        onOpen={open}
        onNew={newNotebook}
        onExport={exportFile}
        onCommands={openCommands}
        onSettings={openSettings}
        onDocs={openCatalog}
        outlineVisible={outlineVisible} inspectorVisible={!!state.panel}
        onOutline={toggleOutline} onInspector={toggleInspector}
      />
      <div className="workspace-layout">
        <Notebook
          state={state}
          controller={controller}
          t={t}
          language={language}
        />
        <Inspector
          state={state}
          controller={controller}
          t={t}
          onSettings={() => openSettings(true)}
          phone={phone}
        />
      </div>
      </div>
      {!state.panel && phone && (
        <button
          className="show-inspector"
          onClick={() => controller.store.setState({ panel: "assistant" })}
        >
          <WorkbenchIcon name="chat" />{t.assistant}
        </button>
      )}
      {state.error && (
        <div className="notice notice-error" role="alert">
          <span>{state.error}</span>
          <button
            aria-label={t.close}
            onClick={() => controller.store.setState({ error: null })}
          >
            ×
          </button>
        </div>
      )}
      {state.notice && (
        <div className="notice" role="status">
          <span>
            {state.notice === "delete" ? t.deleteNotice : state.notice}
          </span>
          {state.notice === "delete" && (
            <button onClick={() => controller.undo()}>{t.undo}</button>
          )}
          <button
            aria-label={t.close}
            onClick={() => controller.store.setState({ notice: null })}
          >
            ×
          </button>
        </div>
      )}
      <input
        hidden
        ref={fileInput}
        type="file"
        accept=".omnb,application/json"
        aria-label={t.open}
        onChange={(event) => {
          const file = event.target.files?.[0];
          if (file)
            void file
              .text()
              .then((text) => controller.load(decodeNotebook(text)))
              .catch(() => controller.store.setState({ error: t.fileError }));
          event.target.value = "";
        }}
      />
      <CommandPalette
        open={commands}
        onOpen={setCommands}
        t={t}
        commands={[
          {
            label: t.run,
            shortcut: "⌘↵",
            disabled: state.busy || !state.active,
            run: () => {
              if (state.active) void controller.run(state.active);
            },
          },
          { label: t.runAll, disabled: state.busy || state.cells.length === 0, run: () => void controller.runAll() },
          {
            label: t.stop,
            shortcut: "⌘.",
            run: () => void controller.stopWork(),
          },
          {
            label: `＋ ${t.math}`,
            run: () => {
              controller.add("Math");
            },
          },
          {
            label: `＋ ${t.text}`,
            run: () => {
              controller.add("Text");
            },
          },
          { label: t.save, shortcut: "⌘S", run: save },
          { label: t.open, run: open },
          { label: t.newNotebook, run: newNotebook },
          { label: t.exportMarkdown, run: () => exportFile("markdown") },
          { label: t.exportLatex, run: () => exportFile("latex") },
          { label: t.settings, run: () => openSettings() },
          {
            label: t.variables,
            run: () => controller.store.setState({ panel: "variables" }),
          },
          {
            label: t.functionCatalog,
            run: openCatalog,
          },
        ]}
      />
      <FunctionCatalog open={catalog} onClose={() => setCatalog(false)} controller={controller} t={t} />
      {preferences && state.config && (
        <Preferences
          config={state.config}
          controller={controller}
          t={t}
          theme={theme}
          onTheme={changeTheme}
          initialTab={preferences}
          onClose={() => setPreferences(null)}
        />
      )}
    </div>
  );
}
