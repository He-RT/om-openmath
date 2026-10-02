import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { expect, test } from "vitest";
import App from "./App";
import { MockKernel } from "./test/mockKernel";
import type { KernelConfig } from "./kernel/generated/KernelConfig";

test("identifies OpenMath while the actual kernel is starting and has no active run controls", () => {
  render(<App />);
  expect(screen.getByText("OpenMath")).toBeTruthy();
  expect(screen.getByRole("status").textContent).toContain("Starting kernel");
  expect(screen.queryByRole("button", { name: /solve|run/i })).toBeNull();
});
test("notebook controls use synchronized typed requests and keep source-only title without rebuilding live state", async () => {
  const config = {
    general: {
      language: "en",
      dialect: "modern",
      constants: "math",
      reactive: true,
      auto_run_dependents: true,
      show_steps: true,
      auto_plot: false,
      eval_timeout_ms: 30000,
    },
    llm: {
      enabled: false,
      translate: "",
      explain: "",
      complete: "",
      chat: "",
      fix: "",
      send_context: false,
      profiles: [],
    },
  } as KernelConfig;
  const kernel = new MockKernel((request) =>
    request.type === "get_config"
      ? { type: "config", config }
      : request.type === "get_notebook_state"
        ? {
            type: "notebook_state",
            state: {
              file: { version: 1, title: "", cells: [] },
              cells: [],
              definition_order: [],
              cycles: [],
            },
          }
        : { type: "ok" },
  );
  render(<App kernel={kernel} />);
  await screen.findByRole("heading", { name: "Start with an expression" });
  fireEvent.change(screen.getByLabelText("Notebook title"), {
    target: { value: "My equations" },
  });
  await waitFor(() =>
    expect(
      kernel.requests.some(
        (r) => r.type === "rename_notebook" && r.title === "My equations",
      ),
    ).toBe(true),
  );
  expect(kernel.requests.some((r) => r.type === "load_notebook")).toBe(false);
  fireEvent.click(screen.getByRole("button", { name: "＋ Text" }));
  await screen.findByRole("textbox", { name: "Text input 1" });
  fireEvent.change(screen.getByRole("textbox", { name: "Text input 1" }), {
    target: { value: "# My reasoning" },
  });
  await waitFor(() =>
    expect(
      kernel.requests.some(
        (r) => r.type === "upsert_cell" && r.cell.source === "# My reasoning",
      ),
    ).toBe(true),
  );
  expect(kernel.requests.some((r) => r.type === "evaluate")).toBe(false);
});
