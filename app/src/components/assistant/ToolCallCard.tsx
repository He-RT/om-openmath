import type { ToolResultView } from "../../state/assistant";
import type { Messages } from "../../i18n";
export function ToolCallCard({
  tool,
  t,
}: {
  tool: ToolResultView;
  t: Messages;
}) {
  return (
    <details className="tool-call-card">
      <summary>
        ⚒ {tool.name} · {t.casTool}
      </summary>
      <pre>{tool.arguments}</pre>
      <span>→</span>
      <pre>{tool.result}</pre>
    </details>
  );
}
