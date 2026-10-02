import type { Message } from "../../kernel/generated/Message";
import type { Locale, Messages as Text } from "../../i18n";
import { messageText } from "../../i18n/messages";
export function Messages({
  items,
  t,
  language,
}: {
  items: Message[];
  t: Text;
  language: Locale;
}) {
  if (!items.length) return null;
  return (
    <details className="messages" open={items.some((m) => m.level === "Error")}>
      <summary>
        {items.length} {t.messages}
      </summary>
      {items.map((m, i) => (
        <div className={`message ${m.level.toLowerCase()}`} key={i}>
          <code>
            {m.symbol}::{m.tag}
          </code>{" "}
          — {messageText(m, language)}
        </div>
      ))}
    </details>
  );
}
