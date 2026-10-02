import { expect, test } from "vitest";
import { NotebookController } from "./notebookStore";
import { MockKernel } from "../test/mockKernel";
import type { KernelConfig } from "../kernel/generated/KernelConfig";
import type { Request, Response } from "../kernel/client";
const config: KernelConfig = {
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
};
test("current source edits serialize before runs and obsolete results stay stale", async () => {
  const order: Request[] = [];
  let finish: ((response: Response) => void) | undefined;
  const kernel = new MockKernel(async (request) => {
    order.push(request);
    if (request.type === "get_config") return { type: "config", config };
    if (request.type === "get_notebook_state")
      return {
        type: "notebook_state",
        state: {
          file: { version: 1, title: "", cells: [] },
          cells: [],
          definition_order: [],
          cycles: [],
        },
      };
    if (request.type === "evaluate")
      return new Promise<Response>((resolve) => {
        finish = resolve;
      });
    return { type: "ok" };
  });
  const controller = new NotebookController(kernel);
  await controller.initialize();
  const id = controller.add("Math", "1+1");
  await controller.flush();
  const running = controller.run(id);
  await new Promise((resolve) => setTimeout(resolve, 0));
  controller.edit(id, { source: "2+2" });
  finish?.({
    type: "evaluated",
    cell_id: id,
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
    reran: [],
  });
  await running;
  expect(controller.store.getState().cells[0]?.source).toBe("2+2");
  expect(controller.store.getState().cells[0]?.status).toBe("Stale");
  expect(controller.store.getState().cells[0]?.output).toBeUndefined();
  expect(order.findIndex((r) => r.type === "upsert_cell")).toBeLessThan(
    order.findIndex((r) => r.type === "evaluate"),
  );
  controller.dispose();
});
test("saving an older source snapshot does not mark newly edited source clean", async () => {
  const kernel = new MockKernel(() => ({ type: "ok" }));
  const controller = new NotebookController(kernel);
  const id = controller.add("Text", "before");
  const file = controller.file();
  controller.edit(id, { source: "after" });
  controller.markSaved("saved old snapshot", file);
  expect(controller.store.getState().dirty).toBe(true);
  controller.markSaved("saved latest", controller.file());
  expect(controller.store.getState().dirty).toBe(false);
  controller.dispose();
});
test("restart reconciles the latest frontend source even when an earlier serialized edit is pending", async () => {
  let release: ((response: Response) => void) | undefined;
  let file = {
    version: 1,
    title: "",
    cells: [],
  } as import("../kernel/generated/NotebookFile").NotebookFile;
  const kernel = new MockKernel((request) => {
    if (request.type === "get_config") return { type: "config", config };
    if (
      request.type === "get_notebook_state" ||
      request.type === "restore_definitions"
    )
      return {
        type: "notebook_state",
        state: {
          file,
          cells: file.cells.map((c) => ({
            id: c.id,
            status: "Stale",
            defines: [],
            uses: [],
          })),
          definition_order: [],
          cycles: [],
        },
      };
    if (request.type === "upsert_cell")
      return new Promise<Response>((resolve) => {
        release = resolve;
      });
    if (request.type === "load_notebook") {
      file = request.file;
      return { type: "ok" };
    }
    return { type: "ok" };
  });
  const controller = new NotebookController(kernel);
  await controller.initialize();
  const id = controller.add("Math", "before");
  await new Promise((resolve) => setTimeout(resolve, 0));
  controller.edit(id, { source: "latest" });
  await controller.interrupt();
  expect(file.cells[0]?.source).toBe("latest");
  expect(controller.store.getState().cells[0]?.source).toBe("latest");
  release?.({ type: "ok" });
  controller.dispose();
});
test("a computed response is committed only after matching actual current-source metadata", async () => {
  let file = {
    version: 1,
    title: "",
    cells: [],
  } as import("../kernel/generated/NotebookFile").NotebookFile;
  const kernel = new MockKernel((request) => {
    if (request.type === "get_config") return { type: "config", config };
    if (request.type === "get_notebook_state")
      return {
        type: "notebook_state",
        state: {
          file,
          cells: file.cells.map((c) => ({
            id: c.id,
            status: "Done",
            defines: [],
            uses: [],
            exec_count: 1,
          })),
          definition_order: [],
          cycles: [],
        },
      };
    if (request.type === "upsert_cell") {
      file = { ...file, cells: [request.cell] };
      return { type: "ok" };
    }
    if (request.type === "evaluate")
      return {
        type: "evaluated",
        cell_id: request.cell_id,
        output: {
          items: [
            {
              type: "expr",
              out_index: 1,
              input_form: "4",
              modern_form: "4",
              latex: "4",
            },
          ],
          messages: [],
          timing_ms: 2,
        },
        reran: [],
      };
    return { type: "ok" };
  });
  const controller = new NotebookController(kernel);
  await controller.initialize();
  const id = controller.add("Math", "2+2");
  await controller.run(id);
  expect(controller.store.getState().cells[0]?.output?.items[0]?.type).toBe(
    "expr",
  );
  expect(controller.store.getState().cells[0]?.exec_count).toBe(1);
  controller.dispose();
});
