import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { useLlmTask } from "./useLlmTask";
import { SuggestionCard } from "./SuggestionCard";
import { expandReferences } from "../../state/assistant";
import { observeKernel } from "../../kernel/observedClient";
import { featureProfile } from "./profile";
import { MockKernel } from "../../test/mockKernel";
import { messages } from "../../i18n";
import type { UiCell } from "../../state/notebookStore";
const t = messages("en");
const suggestion = {
  wolfram: "Solve[x^2==4,x]",
  modern: "solve(x^2=4,x)",
  latex: "x^2=4",
  explanation: "Checked source",
};
test("native credential presence follows GetConfig masks, not a merely named environment variable", () => {
  const profile = {
    name: "p",
    kind: "openai_chat" as const,
    base_url: "https://example.invalid",
    model: "m",
    api_key: null,
    api_key_env: "OPTIONAL_KEY",
    extra_headers: {},
    temperature: 0.2,
    max_tokens: 64,
    supports_tools: false,
    supports_json_mode: false,
    timeout_ms: 1000,
  };
  const config = {
    enabled: true,
    chat: "p",
    profiles: [profile],
  } as import("../../kernel/generated/LlmConfig").LlmConfig;
  expect(featureProfile(config, "chat")).toBeNull();
  expect(
    featureProfile(
      { ...config, profiles: [{ ...profile, api_key: "***" }] },
      "chat",
    ),
  ).toBeTruthy();
});
test("checked suggestion actions require explicit clicks and preserve selected dialect", () => {
  const apply = vi.fn();
  render(
    <SuggestionCard
      suggestion={suggestion}
      dialect="Modern"
      t={t}
      onInsert={apply}
      onRun={(s) => apply(s, "run")}
      onEdit={(s) => apply(s, "edit")}
    />,
  );
  expect(apply).not.toHaveBeenCalled();
  expect(screen.getByText("solve(x^2=4,x)")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Insert as code" }));
  expect(apply).toHaveBeenLastCalledWith("solve(x^2=4,x)");
  fireEvent.click(screen.getByRole("button", { name: "Run" }));
  expect(apply).toHaveBeenLastCalledWith("solve(x^2=4,x)", "run");
});
test("cell references expand actual source and only current InputForm results", () => {
  const cells = [
    {
      id: "c",
      source: "a=2",
      kind: "Math",
      dialect: "Modern",
      status: "Done",
      revision: 0,
      defines: ["a"],
      uses: [],
      output: {
        items: [
          {
            type: "expr",
            out_index: 1,
            input_form: "2",
            modern_form: "2",
            latex: "2",
          },
        ],
        messages: [],
        timing_ms: 1,
      },
    },
  ] as UiCell[];
  const expanded = expandReferences("Explain @cell1", cells);
  expect(expanded).toContain("a=2");
  expect(expanded).toContain('"outputs":["2"]');
  expect(
    expandReferences("@cell1", [{ ...cells[0]!, status: "Stale" }]),
  ).toContain('"outputs":[]');
  expect(() => expandReferences("@cell9", cells)).toThrow(
    "Unknown cell reference",
  );
});
function Harness({
  kernel,
  scope = "a",
}: {
  kernel: import("../../kernel/client").KernelClient;
  scope?: string;
}) {
  const task = useLlmTask(kernel, scope, "Failed");
  return (
    <>
      <button
        onClick={() =>
          task.start({ type: "llm_translate", text: "question", cell_id: null })
        }
      >
        Start
      </button>
      <button onClick={task.cancel}>Cancel</button>
      <span>{task.value?.text}</span>
      {task.value?.suggestions.map((s, i) => (
        <span key={i}>{s.wolfram}</span>
      ))}
    </>
  );
}
test("subscription precedes actual start; independent IDs filter unknown/late events and cancellation", async () => {
  let id = "";
  const kernel = new MockKernel((r) => {
    if (r.type === "llm_translate") {
      id = r.request_id;
      kernel.emit({ type: "llm_delta", request_id: id, text: "early" });
      return { type: "llm_started", request_id: id, http: null };
    }
    return { type: "ok" };
  });
  render(<Harness kernel={kernel} />);
  fireEvent.click(screen.getByText("Start"));
  await screen.findByText("early");
  act(() =>
    kernel.emit({ type: "llm_suggestion", request_id: id, suggestion }),
  );
  expect(screen.getByText(suggestion.wolfram)).toBeTruthy();
  fireEvent.click(screen.getByText("Cancel"));
  act(() => kernel.emit({ type: "llm_delta", request_id: id, text: "late" }));
  expect(screen.queryByText("earlylate")).toBeNull();
  expect(
    kernel.requests.some((r) => r.type === "llm_cancel" && r.request_id === id),
  ).toBe(true);
});
test("late start acknowledgement after scope disposal cancels provider again and never installs data", async () => {
  let resolve!: (r: import("../../kernel/client").Response) => void;
  let id = "";
  const kernel = new MockKernel((r) => {
    if (r.type === "llm_translate") {
      id = r.request_id;
      return new Promise((done) => (resolve = done));
    }
    return { type: "ok" };
  });
  const view = render(<Harness kernel={kernel} />);
  fireEvent.click(screen.getByText("Start"));
  await waitFor(() => expect(id).not.toBe(""));
  view.unmount();
  await act(async () =>
    resolve({ type: "llm_started", request_id: id, http: null }),
  );
  expect(kernel.requests.filter((r) => r.type === "llm_cancel")).toHaveLength(
    2,
  );
});
test("global observer tracks all start kinds, actual errors and genuine cancellation without recording HTTP", async () => {
  const raw = new MockKernel((r) =>
    "request_id" in r
      ? { type: "llm_started", request_id: r.request_id, http: null }
      : { type: "ok" },
  );
  const observed = observeKernel(raw);
  await observed.client.request({
    type: "llm_complete",
    request_id: "complete",
    prefix: "x^2",
    suffix: "",
    dialect: "Modern",
  });
  expect(observed.store.getState().active).toBe(1);
  raw.emit({
    type: "llm_error",
    request_id: "complete",
    message: "Provider rejected",
  });
  expect(observed.store.getState().error).toBe("Provider rejected");
  await observed.client.request({
    type: "llm_translate",
    request_id: "new",
    text: "q",
    cell_id: null,
  });
  await observed.client.request({ type: "llm_cancel", request_id: "new" });
  raw.emit({ type: "llm_error", request_id: "new", message: "cancelled" });
  expect(observed.store.getState().error).toBeNull();
  observed.detach();
});
