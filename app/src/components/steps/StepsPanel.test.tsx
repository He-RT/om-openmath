import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { expect, test } from "vitest";
import { StepsPanel } from "./StepsPanel";
import { MockKernel } from "../../test/mockKernel";
import { messages } from "../../i18n";
import type { StepsView } from "../../kernel/generated/StepsView";
import type { KernelConfig } from "../../kernel/generated/KernelConfig";
const steps: StepsView = {
  root: [
    {
      id: "S1",
      rule_id: "factor",
      title_key: "step.factor",
      level: "Major",
      params: { factors: "x-1" },
      before_latex: ["x^2-1"],
      after_latex: ["(x-1)(x+1)"],
      children: [
        {
          id: "S1.1",
          rule_id: "verify",
          title_key: "step.verify",
          level: "Minor",
          params: { solution: "x=1" },
          before_latex: ["x=1"],
          after_latex: ["1=1"],
          children: [],
        },
      ],
    },
  ],
};
const config = {
  llm: {
    enabled: true,
    explain: "test",
    send_context: false,
    profiles: [
      {
        name: "test",
        kind: "openai_chat",
        base_url: "https://example.invalid/v1",
        model: "fixture",
        api_key: "***",
        api_key_env: null,
        extra_headers: {},
        extra_body: {},
      },
    ],
  },
} as KernelConfig;
function mount(
  kernel = new MockKernel(() => ({
    type: "llm_started",
    request_id: "unused",
    http: null,
  })),
  cfg: KernelConfig | null = config,
) {
  return {
    kernel,
    ...render(
      <StepsPanel
        steps={steps}
        cellId="c"
        outIndex={2}
        current
        config={cfg}
        kernel={kernel}
        t={messages("en")}
        language="en"
      />,
    ),
  };
}
test("actual step IDs, localized titles and expressions are shown; minor details collapse and expand", () => {
  mount(undefined, null);
  expect(screen.getByText("Factor the polynomial.")).toBeTruthy();
  expect(screen.getByLabelText("x^2-1")).toBeTruthy();
  expect(screen.queryByLabelText("1=1")).toBeNull();
  fireEvent.click(screen.getByRole("checkbox", { name: "Show all details" }));
  expect(screen.getByLabelText("1=1")).toBeTruthy();
  expect(document.querySelectorAll(".step-node")).toHaveLength(2);
  expect(
    (screen.getByRole("button", { name: "Explain all" }) as HTMLButtonElement)
      .disabled,
  ).toBe(true);
  expect(screen.getByText("Configure AI in settings")).toBeTruthy();
});

test("empty records and missing browser credentials cannot start model requests", () => {
  const kernel = new MockKernel();
  const r = render(
    <StepsPanel
      steps={{ root: [] }}
      cellId="empty"
      outIndex={1}
      current
      config={config}
      kernel={kernel}
      t={messages("en")}
      language="en"
    />,
  );
  expect(
    (screen.getByRole("button", { name: "Explain all" }) as HTMLButtonElement)
      .disabled,
  ).toBe(true);
  expect(screen.getByText("No recorded steps for this cell.")).toBeTruthy();
  r.rerender(
    <StepsPanel
      steps={steps}
      cellId="empty"
      outIndex={1}
      current
      config={{
        ...config,
        llm: {
          ...config.llm,
          profiles: config.llm.profiles.map((p) => ({ ...p, api_key: null })),
        },
      }}
      kernel={kernel}
      t={messages("en")}
      language="en"
    />,
  );
  expect(
    (screen.getByRole("button", { name: "Explain all" }) as HTMLButtonElement)
      .disabled,
  ).toBe(true);
  expect(kernel.requests).toHaveLength(0);
});
test("streamed explanation matches its actual request and references expand/focus the supplied step only", async () => {
  let requestId = "";
  const kernel = new MockKernel((r) => {
    if (r.type === "llm_explain") {
      requestId = r.request_id;
      return { type: "llm_started", request_id: r.request_id, http: null };
    }
    return { type: "ok" };
  });
  mount(kernel);
  fireEvent.click(screen.getByRole("button", { name: "Explain all" }));
  await screen.findByRole("dialog");
  expect(kernel.requests).toHaveLength(0);
  fireEvent.click(screen.getByRole("button", { name: "Send to AI" }));
  await waitFor(() => expect(requestId).not.toBe(""));
  expect(kernel.requests[0]).toEqual({
    type: "llm_explain",
    request_id: requestId,
    cell_id: "c",
    out_index: 2,
    step_id: null,
  });
  act(() => {
    kernel.emit({ type: "llm_delta", request_id: "other", text: "unrelated" });
    kernel.emit({
      type: "llm_delta",
      request_id: requestId,
      text: "**Reasoning** [S1.1] $x^2$ `code [S1]` unknown [S99]",
    });
  });
  expect(screen.queryByText("unrelated")).toBeNull();
  expect(screen.getByText("Reasoning")).toBeTruthy();
  expect(document.querySelector(".step-explanation .katex")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Go to S1.1" }));
  expect(screen.getByLabelText("1=1")).toBeTruthy();
  expect(
    document
      .querySelector('[data-step-id="S1.1"]')
      ?.classList.contains("step-highlight"),
  ).toBe(true);
  expect(screen.queryByRole("button", { name: "Go to S99" })).toBeNull();
  expect(screen.queryByRole("button", { name: "Go to S1" })).toBeNull();
  act(() => kernel.emit({ type: "llm_done", request_id: requestId }));
  expect(
    screen.queryByRole("button", { name: "Cancel explanation" }),
  ).toBeNull();
});
test("cancel/unmount removes active jobs and ignores late events; synchronous events before start response are kept", async () => {
  let id = "";
  const kernel = new MockKernel((r) => {
    if (r.type === "llm_explain") {
      id = r.request_id;
      kernel.emit({ type: "llm_delta", request_id: id, text: "early" });
      return { type: "llm_started", request_id: id, http: null };
    }
    return { type: "ok" };
  });
  const view = mount(kernel);
  fireEvent.click(screen.getByRole("button", { name: "Why? S1" }));
  fireEvent.click(
    screen.getByRole("checkbox", {
      name: "Remember for this session and destination",
    }),
  );
  fireEvent.click(screen.getByRole("button", { name: "Send to AI" }));
  await screen.findByText("early");
  fireEvent.click(screen.getByRole("button", { name: "Cancel explanation" }));
  await waitFor(() =>
    expect(
      kernel.requests.some(
        (r) => r.type === "llm_cancel" && r.request_id === id,
      ),
    ).toBe(true),
  );
  act(() => kernel.emit({ type: "llm_delta", request_id: id, text: " late" }));
  expect(screen.queryByText("early late")).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "Why? S1" }));
  await waitFor(() =>
    expect(
      kernel.requests.filter((r) => r.type === "llm_explain"),
    ).toHaveLength(2),
  );
  view.unmount();
  await waitFor(() =>
    expect(kernel.requests.filter((r) => r.type === "llm_cancel")).toHaveLength(
      2,
    ),
  );
});

test("cancellation before start acknowledgement cancels the late-started job and source changes revoke a pending privacy prompt", async () => {
  let finish!: (r: import("../../kernel/client").Response) => void;
  let id = "";
  const kernel = new MockKernel((r) => {
    if (r.type === "llm_explain") {
      id = r.request_id;
      return new Promise((resolve) => {
        finish = resolve;
      });
    }
    return { type: "ok" };
  });
  const view = mount(kernel);
  fireEvent.click(screen.getByRole("button", { name: "Explain all" }));
  fireEvent.click(screen.getByRole("button", { name: "Send to AI" }));
  await waitFor(() => expect(id).not.toBe(""));
  fireEvent.click(screen.getByRole("button", { name: "Cancel explanation" }));
  await act(async () =>
    finish({ type: "llm_started", request_id: id, http: null }),
  );
  expect(kernel.requests.filter((r) => r.type === "llm_cancel")).toHaveLength(
    2,
  );
  fireEvent.click(screen.getByRole("button", { name: "Explain all" }));
  expect(screen.getByRole("dialog")).toBeTruthy();
  view.rerender(
    <StepsPanel
      steps={steps}
      cellId="c"
      outIndex={2}
      current={false}
      config={config}
      kernel={kernel}
      t={messages("en")}
      language="en"
    />,
  );
  expect(screen.queryByRole("dialog")).toBeNull();
});
test("actual errors remain attached to their step and stale results cannot send explanations", async () => {
  const kernel = new MockKernel((r) =>
    r.type === "llm_explain"
      ? { type: "error", message: "Provider not configured" }
      : { type: "ok" },
  );
  const view = mount(kernel);
  fireEvent.click(screen.getByRole("button", { name: "Why? S1" }));
  fireEvent.click(screen.getByRole("button", { name: "Send to AI" }));
  expect((await screen.findByRole("alert")).textContent).toContain(
    "Provider not configured",
  );
  view.rerender(
    <StepsPanel
      steps={steps}
      cellId="c"
      outIndex={2}
      current={false}
      config={config}
      kernel={kernel}
      t={messages("en")}
      language="en"
    />,
  );
  expect(
    (screen.getByRole("button", { name: "Explain all" }) as HTMLButtonElement)
      .disabled,
  ).toBe(true);
});
