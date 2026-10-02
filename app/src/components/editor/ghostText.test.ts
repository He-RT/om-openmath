import { afterEach, expect, test, vi } from "vitest";
import { EditorState } from "@codemirror/state";
import {
  ghostText,
  setGhost,
  partialLength,
  GhostSession,
  eligiblePosition,
  acceptGhost,
  type GhostCandidate,
} from "./ghostText";
import { MockKernel } from "../../test/mockKernel";
import type { ProfileConfig } from "../../kernel/generated/ProfileConfig";
afterEach(() => vi.useRealTimers());
const profile = {
  name: "fim",
  base_url: "https://fixture.invalid",
  model: "fixture",
  kind: "openai_fim",
  api_key: "***",
  extra_headers: {},
} as ProfileConfig;
function candidate(source = "abc", pos = source.length): GhostCandidate {
  return { source, pos, dialect: "Modern", scope: profile, profile };
}
test("StateField keeps typed prefixes, clears unrelated edits/cursor moves and uses UTF16 source positions", () => {
  let state = EditorState.create({
    doc: "α+1",
    selection: { anchor: 3 },
    extensions: [ghostText],
  });
  state = state.update({
    effects: setGhost.of({ pos: 3, text: " + sqrt(🙂)" }),
  }).state;
  state = state.update({
    changes: { from: 3, insert: " + " },
    selection: { anchor: 6 },
  }).state;
  expect(state.field(ghostText)).toEqual({ pos: 6, text: "sqrt(🙂)" });
  state = state.update({ selection: { anchor: 5 } }).state;
  expect(state.field(ghostText)).toBeNull();
  state = state.update({
    selection: { anchor: 6 },
    effects: setGhost.of({ pos: 6, text: "sqrt(🙂)" }),
  }).state;
  state = state.update({ changes: { from: 0, insert: "x" } }).state;
  expect(state.field(ghostText)).toBeNull();
  expect(partialLength("sin(α) + 2")).toBe(3);
  expect(partialLength(" + αβ")).toBe(3);
});

test("eligible cursor conditions include line length, focus, local popup, busy state and actual selection", () => {
  const state = EditorState.create({
    doc: "x^2",
    selection: { anchor: 3 },
    extensions: [ghostText],
  });
  expect(eligiblePosition(state, true, true, false, false)).toBe(true);
  for (const [focus, configured, busy, local] of [
    [false, true, false, false],
    [true, false, false, false],
    [true, true, true, false],
    [true, true, false, true],
  ])
    expect(eligiblePosition(state, focus!, configured!, busy!, local!)).toBe(
      false,
    );
  for (const state of [
    EditorState.create({ doc: "x", selection: { anchor: 1 } }),
    EditorState.create({ doc: "x^2", selection: { anchor: 1 } }),
    EditorState.create({ doc: "x^2", selection: { anchor: 1, head: 3 } }),
  ])
    expect(eligiblePosition(state, true, true, false, false)).toBe(false);
});

test("explicit full and partial acceptance preserve actual source, positions and residual text", () => {
  let state = EditorState.create({
    doc: "x^2",
    selection: { anchor: 3 },
    extensions: [ghostText],
  });
  state = state.update({
    effects: setGhost.of({ pos: 3, text: " + sin(α)" }),
  }).state;
  const view = {
    get state() {
      return state;
    },
    dispatch: (spec: import("@codemirror/state").TransactionSpec) => {
      state = state.update(spec).state;
    },
  } as import("@codemirror/view").EditorView;
  expect(acceptGhost(view, true)).toBe(true);
  expect(state.doc.toString()).toBe("x^2 + ");
  expect(state.field(ghostText)).toEqual({ pos: 6, text: "sin(α)" });
  expect(acceptGhost(view)).toBe(true);
  expect(state.doc.toString()).toBe("x^2 + sin(α)");
  expect(state.field(ghostText)).toBeNull();
  expect(state.selection.main.head).toBe(state.doc.length);
  expect(acceptGhost(view)).toBe(false);
});
test("350ms idle coalesces input and exact request snapshots; finalized delta is painted only for the current source", async () => {
  vi.useFakeTimers();
  let current: GhostCandidate | null = candidate("sqrt(x)+");
  const paint = vi.fn();
  const kernel = new MockKernel((r) =>
    r.type === "llm_complete"
      ? { type: "llm_started", request_id: r.request_id, http: null }
      : { type: "ok" },
  );
  const session = new GhostSession(
    kernel,
    () => current,
    paint,
    async () => true,
    () => {},
  );
  session.changed(false);
  await vi.advanceTimersByTimeAsync(200);
  current = candidate("sqrt(x)+1");
  session.changed(false);
  await vi.advanceTimersByTimeAsync(349);
  expect(kernel.requests).toHaveLength(0);
  await vi.advanceTimersByTimeAsync(1);
  const r = kernel.requests[0];
  if (r?.type !== "llm_complete") throw new Error("missing job");
  expect(r).toMatchObject({
    prefix: "sqrt(x)+1",
    suffix: "",
    dialect: "Modern",
  });
  kernel.emit({ type: "llm_delta", request_id: r.request_id, text: " + 2" });
  expect(paint).not.toHaveBeenCalledWith({ pos: 9, text: " + 2" });
  kernel.emit({ type: "llm_done", request_id: r.request_id });
  expect(paint).toHaveBeenLastCalledWith({ pos: 9, text: " + 2" });
  session.destroy();
});
test("blocked contexts do not request; moving/editing cancels a pending job and ignores old model events", async () => {
  vi.useFakeTimers();
  let current: GhostCandidate | null = null;
  const paint = vi.fn();
  const kernel = new MockKernel((r) =>
    r.type === "llm_complete"
      ? { type: "llm_started", request_id: r.request_id, http: null }
      : { type: "ok" },
  );
  const session = new GhostSession(
    kernel,
    () => current,
    paint,
    async () => true,
    () => {},
  );
  session.changed(false);
  await vi.advanceTimersByTimeAsync(350);
  expect(kernel.requests).toHaveLength(0);
  current = candidate();
  session.changed(false);
  await vi.advanceTimersByTimeAsync(350);
  const request = kernel.requests[0];
  if (request?.type !== "llm_complete") throw new Error();
  current = null;
  session.revalidate();
  await vi.advanceTimersByTimeAsync(0);
  expect(kernel.requests[1]).toEqual({
    type: "llm_cancel",
    request_id: request.request_id,
  });
  kernel.emit({
    type: "llm_delta",
    request_id: request.request_id,
    text: "bad",
  });
  kernel.emit({ type: "llm_done", request_id: request.request_id });
  expect(paint).not.toHaveBeenCalledWith({ pos: 3, text: "bad" });
  session.destroy();
});
test("cancel-before-start acknowledgement and disposal stop late-started transport; cancelled privacy never sends", async () => {
  vi.useFakeTimers();
  let resolve!: (r: import("../../kernel/client").Response) => void;
  const kernel = new MockKernel((r) =>
    r.type === "llm_complete"
      ? new Promise((done) => (resolve = done))
      : { type: "ok" },
  );
  const session = new GhostSession(
    kernel,
    () => candidate(),
    vi.fn(),
    async () => true,
    () => {},
  );
  session.changed(false);
  await vi.advanceTimersByTimeAsync(350);
  const r = kernel.requests[0];
  if (r?.type !== "llm_complete") throw new Error();
  session.destroy();
  resolve({ type: "llm_started", request_id: r.request_id, http: null });
  await vi.advanceTimersByTimeAsync(0);
  expect(kernel.requests.filter((r) => r.type === "llm_cancel")).toHaveLength(
    2,
  );
  const blocked = new MockKernel();
  const denied = new GhostSession(
    blocked,
    () => candidate(),
    vi.fn(),
    async () => false,
    () => {},
  );
  denied.changed(false);
  await vi.advanceTimersByTimeAsync(350);
  expect(blocked.requests).toHaveLength(0);
  denied.destroy();
});
