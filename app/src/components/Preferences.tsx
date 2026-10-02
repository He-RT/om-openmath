import type { KernelConfig } from "../kernel/generated/KernelConfig";
import type { Messages } from "../i18n";
import type { NotebookController } from "../state/notebookStore";
import { useEffect, useRef } from "react";
export function Preferences({
  config,
  controller,
  t,
  theme,
  onTheme,
  onClose,
}: {
  config: KernelConfig;
  controller: NotebookController;
  t: Messages;
  theme: string;
  onTheme: (theme: string) => void;
  onClose: () => void;
}) {
  const root = useRef<HTMLElement>(null);
  const close = useRef(onClose);
  close.current = onClose;
  useEffect(() => {
    const previous = document.activeElement;
    const element = root.current;
    if (!element) return;
    const controls = () => [
      ...element.querySelectorAll<HTMLElement>(
        "button:not(:disabled),input:not(:disabled),select:not(:disabled)",
      ),
    ];
    controls()[0]?.focus();
    const key = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        close.current();
      }
      if (event.key === "Tab") {
        const nodes = controls();
        const first = nodes[0];
        const last = nodes.at(-1);
        if (event.shiftKey && document.activeElement === first) {
          event.preventDefault();
          last?.focus();
        } else if (!event.shiftKey && document.activeElement === last) {
          event.preventDefault();
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
        ref={root}
        className="preferences"
        role="dialog"
        aria-modal
        aria-label={t.settings}
      >
        <header>
          <h2>{t.settings}</h2>
          <button aria-label={t.close} onClick={onClose}>
            ×
          </button>
        </header>
        <div className="preferences-fields">
          <label>
            {t.language}
            <select
              aria-label={t.language}
              value={config.general.language}
              onChange={(e) =>
                void controller.configure({
                  language: e.target.value as "auto" | "en" | "zh-CN",
                })
              }
            >
              <option value="auto">{t.system}</option>
              <option value="zh-CN">简体中文</option>
              <option value="en">English</option>
            </select>
          </label>
          <label>
            {t.theme}
            <select
              aria-label={t.theme}
              value={theme}
              onChange={(e) => onTheme(e.target.value)}
            >
              <option value="system">{t.system}</option>
              <option value="light">{t.light}</option>
              <option value="dark">{t.dark}</option>
            </select>
          </label>
          <label>
            {t.constants}
            <select
              value={config.general.constants}
              onChange={(e) =>
                void controller.configure({
                  constants: e.target.value as "math" | "strict",
                })
              }
            >
              <option value="math">{t.mathConstants}</option>
              <option value="strict">{t.strictConstants}</option>
            </select>
          </label>
          <label>
            {t.reactive}
            <input
              type="checkbox"
              aria-label={t.reactive}
              checked={config.general.reactive}
              onChange={(e) =>
                void controller.configure({ reactive: e.target.checked })
              }
            />
          </label>
          <label>
            {t.autoRun}
            <input
              type="checkbox"
              checked={config.general.auto_run_dependents}
              onChange={(e) =>
                void controller.configure({
                  auto_run_dependents: e.target.checked,
                })
              }
            />
          </label>
          <label>
            {t.timeout}
            <input
              type="number"
              min="1"
              max="3600000"
              value={config.general.eval_timeout_ms}
              onChange={(e) => {
                const value = Number(e.target.value);
                if (
                  Number.isSafeInteger(value) &&
                  value > 0 &&
                  value <= 3600000
                )
                  void controller.configure({ eval_timeout_ms: value });
              }}
            />
          </label>
        </div>
      </section>
    </div>
  );
}
