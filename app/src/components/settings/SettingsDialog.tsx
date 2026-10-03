import { useEffect, useRef, useState } from "react";
import type { KernelConfig } from "../../kernel/generated/KernelConfig";
import type { NotebookController } from "../../state/notebookStore";
import type { Messages } from "../../i18n";
import { GeneralSettings } from "./GeneralSettings";
import { ModelSettings } from "./ModelSettings";
export function SettingsDialog({
  config,
  controller,
  t,
  theme,
  onTheme,
  onClose,
  initialTab = "general",
}: {
  config: KernelConfig;
  controller: NotebookController;
  t: Messages;
  theme: string;
  onTheme: (theme: string) => void;
  onClose: () => void;
  initialTab?: "general" | "models";
}) {
  const [tab, setTab] = useState<"general" | "models" | "features">(initialTab);
  const root = useRef<HTMLElement>(null),
    close = useRef(onClose);
  close.current = onClose;
  useEffect(() => {
    const previous = document.activeElement,
      element = root.current;
    if (!element) return;
    const controls = () =>
      [
        ...element.querySelectorAll<HTMLElement>(
          "button:not(:disabled),input:not(:disabled),select:not(:disabled),textarea:not(:disabled)",
        ),
      ].filter((node) => !node.closest("[hidden]"));
    controls()[0]?.focus();
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        close.current();
      }
      if (e.key === "Tab") {
        const items = controls(),
          first = items[0],
          last = items.at(-1);
        if (e.shiftKey && document.activeElement === first) {
          e.preventDefault();
          last?.focus();
        } else if (!e.shiftKey && document.activeElement === last) {
          e.preventDefault();
          first?.focus();
        }
      }
    };
    element.addEventListener("keydown", key);
    return () => {
      element.removeEventListener("keydown", key);
      if (previous instanceof HTMLElement) previous.focus();
    };
  }, []);
  return (
    <div
      className="modal-backdrop"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <section
        className="preferences settings-dialog"
        role="dialog"
        aria-modal="true"
        aria-label={t.settings}
        ref={root}
      >
        <header>
          <h2>{t.settings}</h2>
          <button aria-label={t.close} onClick={onClose}>
            ×
          </button>
        </header>
        <nav role="tablist">
          {(["general", "models", "features"] as const).map((value) => (
            <button
              key={value}
              role="tab"
              aria-selected={tab === value}
              onClick={() => setTab(value)}
            >
              {value === "general"
                ? t.generalSettings
                : value === "models"
                  ? t.modelSettings
                  : t.featureMappings}
            </button>
          ))}
        </nav>
        <div hidden={tab !== "general"}>
          <GeneralSettings
            config={config}
            controller={controller}
            t={t}
            theme={theme}
            onTheme={onTheme}
          />
        </div>
        <div hidden={tab === "general"}>
          <ModelSettings
            config={config}
            controller={controller}
            t={t}
            featuresOnly={tab === "features"}
          />
        </div>
      </section>
    </div>
  );
}
