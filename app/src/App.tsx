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
import { Preferences } from "./components/Preferences";
import { CommandPalette } from "./components/CommandPalette";
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
  const [preferences, setPreferences] = useState(false);
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
    if (controller.kernel.kind === "wasm") {
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
  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if (!event.metaKey && !event.ctrlKey) return;
      const key = event.key.toLowerCase();
      if (key === "k") {
        event.preventDefault();
        setCommands((value) => !value);
      }
      if (key === "s") {
        event.preventDefault();
        save();
      }
      if (key === ".") {
        event.preventDefault();
        void controller.interrupt();
      }
      if (key === "/" && state.active) {
        event.preventDefault();
        const cell = state.cells.find((c) => c.id === state.active);
        if (cell)
          controller.edit(cell.id, {
            kind: cell.kind === "Ask" ? "Math" : "Ask",
          });
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
    <div className={`app-shell ${state.panel ? "with-panel" : ""}`}>
      <TopBar
        state={state}
        controller={controller}
        t={t}
        onSave={save}
        onOpen={open}
        onNew={newNotebook}
        onCommands={() => setCommands(true)}
        onSettings={() => setPreferences(true)}
      />
      <div className="workspace-layout">
        <Notebook
          state={state}
          controller={controller}
          t={t}
          language={language}
        />
        <Inspector state={state} controller={controller} t={t} />
      </div>
      {!state.panel && (
        <button
          className="show-inspector"
          onClick={() => controller.store.setState({ panel: "docs" })}
        >
          {t.inspect} ↗
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
            run: () => {
              if (state.active) void controller.run(state.active);
            },
          },
          { label: t.runAll, run: () => void controller.runAll() },
          {
            label: t.stop,
            shortcut: "⌘.",
            run: () => void controller.interrupt(),
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
          { label: t.settings, run: () => setPreferences(true) },
          {
            label: t.variables,
            run: () => controller.store.setState({ panel: "variables" }),
          },
          {
            label: t.docs,
            run: () => controller.store.setState({ panel: "docs" }),
          },
        ]}
      />
      {preferences && state.config && (
        <Preferences
          config={state.config}
          controller={controller}
          t={t}
          theme={theme}
          onTheme={(theme) => {
            setTheme(theme);
            try {
              localStorage.setItem("openmath-theme", theme);
            } catch {
              /* Theme remains in memory. */
            }
          }}
          onClose={() => setPreferences(false)}
        />
      )}
    </div>
  );
}
