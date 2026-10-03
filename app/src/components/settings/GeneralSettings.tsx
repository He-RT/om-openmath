import type { KernelConfig } from "../../kernel/generated/KernelConfig";
import type { NotebookController } from "../../state/notebookStore";
import type { Messages } from "../../i18n";
export function GeneralSettings({
  config,
  controller,
  t,
  theme,
  onTheme,
}: {
  config: KernelConfig;
  controller: NotebookController;
  t: Messages;
  theme: string;
  onTheme: (theme: string) => void;
}) {
  return (
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
            if (Number.isSafeInteger(value) && value > 0 && value <= 3600000)
              void controller.configure({ eval_timeout_ms: value });
          }}
        />
      </label>

      <label>
        {t.dialect}
        <select
          aria-label={t.dialect}
          value={config.general.dialect}
          onChange={(e) =>
            void controller.configure({
              dialect: e.target.value as "auto" | "modern" | "wolfram",
            })
          }
        >
          <option value="auto">{t.auto}</option>
          <option value="modern">{t.modern}</option>
          <option value="wolfram">Wolfram</option>
        </select>
      </label>
      <label>
        {t.defaultSteps}
        <input
          type="checkbox"
          checked={config.general.show_steps}
          onChange={(e) =>
            void controller.configure({ show_steps: e.target.checked })
          }
        />
      </label>
      <label>
        {t.autoPlot}
        <input
          type="checkbox"
          checked={config.general.auto_plot}
          onChange={(e) =>
            void controller.configure({ auto_plot: e.target.checked })
          }
        />
      </label>
    </div>
  );
}
