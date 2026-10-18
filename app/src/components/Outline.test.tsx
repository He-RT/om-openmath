import { render, screen, fireEvent } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { Outline, outlineTitle } from "./Outline";
import { NotebookController, type UiCell } from "../state/notebookStore";
import { MockKernel } from "../test/mockKernel";
import { messages } from "../i18n";
const cell = (source: string, kind: UiCell["kind"] = "Text"): UiCell => ({ id: "cell-one", source, kind, dialect: "Modern", status: "Stale", revision: 0, defines: [], uses: [] });
test("outline uses real Markdown headings and does not treat fenced examples as sections", () => {
  expect(outlineTitle(cell("```text\n# not a section\n```\n\n## Real **heading**"))).toEqual({ label: "Real heading", heading: true });
  expect(outlineTitle(cell("# x\n2+2", "Math"))).toEqual({ label: "# x", heading: false });
});
test("outline selection locates stable cell identity without requesting focus or changing source", () => {
  const controller = new NotebookController(new MockKernel());
  const current = cell("let a=2", "Math");
  controller.store.setState({ cells: [current], initializing: false });
  const scroll = vi.fn(), target = document.createElement("section");
  target.className = "notebook-cell"; target.dataset.cellId = current.id; target.scrollIntoView = scroll; document.body.append(target);
  const focus = controller.store.getState().focus;
  render(<Outline controller={controller} state={controller.store.getState()} t={messages("en")} visible overlay={false} onClose={vi.fn()} />);
  fireEvent.click(screen.getByRole("button", { name: /let a=2/ }));
  expect(controller.store.getState().active).toBe(current.id);
  expect(controller.store.getState().focus).toBe(focus);
  expect(controller.store.getState().cells[0]?.source).toBe(current.source);
  expect(scroll).toHaveBeenCalledWith({ block: "nearest", behavior: "instant" });
  target.remove(); controller.dispose();
});
