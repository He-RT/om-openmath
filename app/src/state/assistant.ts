import type { Suggestion } from "../kernel/generated/Suggestion";
import type { ChatMessage } from "../kernel/generated/ChatMessage";
import type { UiCell } from "./notebookStore";
export interface ToolResultView {
  name: string;
  arguments: string;
  result: string;
}
export interface LlmTaskView {
  id: string;
  text: string;
  suggestions: Suggestion[];
  tools: ToolResultView[];
  status: "streaming" | "done" | "cancelled" | "error";
  error?: string;
}
export interface ChatTurnView {
  id: string;
  input: string;
  sent: string;
  result?: LlmTaskView;
}
export function expandReferences(text: string, cells: UiCell[]) {
  return text.replace(
    /(^|[\s(])@cell([1-9]\d*)\b/g,
    (_all, prefix: string, number: string) => {
      const index = Number(number) - 1,
        cell = cells[index];
      if (!cell) throw new Error("Unknown cell reference");
      const current = cell.status === "Done" || cell.status === "Error";
      const outputs = current
        ? (cell.output?.items.flatMap((item) =>
            item.type === "expr" || item.type === "solutions"
              ? [item.input_form]
              : [],
          ) ?? [])
        : [];
      return `${prefix}\n[Notebook reference]\n${JSON.stringify({ cell: index + 1, source: cell.source, dialect: cell.dialect, status: cell.status, output_current: current, outputs })}\n`;
    },
  );
}
export function conversationHistory(turns: ChatTurnView[]): ChatMessage[] {
  const messages: ChatMessage[] = [];
  for (const turn of turns) {
    if (turn.result?.status !== "done") continue;
    messages.push({
      role: "user",
      content: turn.sent,
      tool_calls: [],
      tool_call_id: null,
    });
    const result = turn.result;
    const content = [
      result.text,
      result.tools.length
        ? `Recorded read-only CAS events:\n${JSON.stringify(result.tools)}`
        : "",
      result.suggestions.length
        ? `CAS-parsed source proposals (not executed):\n${JSON.stringify(result.suggestions.map((s) => s.wolfram))}`
        : "",
    ]
      .filter(Boolean)
      .join("\n\n");
    if (content)
      messages.push({
        role: "assistant",
        content,
        tool_calls: [],
        tool_call_id: null,
      });
  }
  return messages;
}
