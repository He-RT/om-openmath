import { createStore } from "zustand/vanilla";
import type { KernelClient, Request, Event } from "../kernel/client";
import type { KernelConfig } from "../kernel/generated/KernelConfig";
import type { CellInput } from "../kernel/generated/CellInput";
import type { CellStatus } from "../kernel/generated/CellStatus";
import type { CellOutput } from "../kernel/generated/CellOutput";
import type { NotebookState } from "../kernel/generated/NotebookState";
import type { NotebookFile } from "../kernel/generated/NotebookFile";
import { locale } from "../i18n";
export interface UiCell extends CellInput {
  revision: number;
  status: CellStatus;
  output?: CellOutput;
  exec_count?: number;
  defines: string[];
  uses: string[];
}
export interface UiState {
  title: string;
  cells: UiCell[];
  active: string | null;
  focus: number;
  config: KernelConfig | null;
  busy: boolean;
  initializing: boolean;
  error: string | null;
  notice: string | null;
  dirty: boolean;
  panel: "variables" | "docs" | "steps" | "assistant" | null;
}
const initial: UiState = {
  title: "",
  cells: [],
  active: null,
  focus: 0,
  config: null,
  busy: false,
  initializing: true,
  error: null,
  notice: null,
  dirty: false,
  panel: "docs",
};
const uiCell = (cell: CellInput): UiCell => ({
  ...cell,
  revision: 0,
  status: cell.kind === "Text" ? "Done" : "Stale",
  defines: [],
  uses: [],
});
export class NotebookController {
  readonly store = createStore<UiState>(() => ({ ...initial }));
  private writes = Promise.resolve();
  private off: () => void;
  private closed = false;
  private epoch = 0;
  private operations = 0;
  private restarting: Promise<void> | undefined;
  private revisions = new Map<string, number>();
  private pendingOutputs = new Map<
    string,
    { output: CellOutput; revision: number }
  >();
  private removed: { cell: UiCell; index: number } | undefined;
  constructor(readonly kernel: KernelClient) {
    this.off = kernel.onEvent((event) => this.event(event));
  }
  async initialize() {
    try {
      await this.kernel.ready;
      await this.kernel.request({
        type: "set_system_language",
        language: locale("auto"),
      });
      const config = await this.kernel.request({ type: "get_config" });
      if (config.type !== "config")
        throw new Error("Kernel configuration unavailable");
      this.store.setState({ config: config.config });
      await this.refresh();
      this.store.setState({ initializing: false });
    } catch (error) {
      this.fail(error);
      this.store.setState({ initializing: false });
    }
  }
  private fail(error: unknown) {
    if (!this.closed)
      this.store.setState({
        error: error instanceof Error ? error.message : "Kernel request failed",
      });
  }
  private write(request: Request) {
    const epoch = this.epoch;
    this.writes = this.writes
      .then(async () => {
        if (this.closed || epoch !== this.epoch) return;
        const response = await this.kernel.request(request);
        if (response.type === "error") throw new Error(response.message);
      })
      .catch((error) => {
        if (epoch === this.epoch) this.fail(error);
      });
    return this.writes;
  }
  flush() {
    return this.writes;
  }
  add(kind: CellInput["kind"] = "Math", source = "", after?: string) {
    const state = this.store.getState();
    const id = crypto.randomUUID();
    const dialect =
      state.config?.general.dialect === "wolfram"
        ? "Wolfram"
        : state.config?.general.dialect === "modern"
          ? "Modern"
          : "Auto";
    const cell = uiCell({ id, kind, source, dialect });
    let index = after
      ? state.cells.findIndex((c) => c.id === after) + 1
      : state.cells.length;
    if (index < 0) index = state.cells.length;
    const cells = [...state.cells];
    cells.splice(index, 0, cell);
    this.store.setState({
      cells,
      active: id,
      focus: state.focus + 1,
      dirty: true,
    });
    void this.write({ type: "upsert_cell", cell });
    if (index < cells.length - 1)
      void this.write({ type: "move_cell", cell_id: id, to_index: index });
    return id;
  }
  select(id: string, focus = false) {
    this.store.setState((state) => ({
      active: id,
      focus: state.focus + (focus ? 1 : 0),
    }));
  }
  edit(
    id: string,
    change: Partial<Pick<CellInput, "source" | "kind" | "dialect">>,
  ) {
    const state = this.store.getState();
    const cell = state.cells.find((c) => c.id === id);
    if (!cell) return;
    const next = {
      ...cell,
      ...change,
      revision: cell.revision + 1,
      status: change.kind === "Text" ? ("Done" as const) : ("Stale" as const),
    };
    this.store.setState({
      cells: state.cells.map((c) => (c.id === id ? next : c)),
      dirty: true,
    });
    void this.write({
      type: "upsert_cell",
      cell: {
        id: next.id,
        kind: next.kind,
        source: next.source,
        dialect: next.dialect,
      },
    });
  }
  remove(id: string) {
    const state = this.store.getState();
    const index = state.cells.findIndex((c) => c.id === id);
    const cell = state.cells[index];
    if (!cell) return;
    this.removed = { cell, index };
    const cells = state.cells.filter((c) => c.id !== id);
    this.store.setState({
      cells,
      active: cells[Math.min(index, cells.length - 1)]?.id ?? null,
      dirty: true,
      notice: "delete",
    });
    void this.write({ type: "delete_cell", cell_id: id });
  }
  undo() {
    const previous = this.removed;
    if (!previous) return;
    this.removed = undefined;
    const state = this.store.getState();
    const cells = [...state.cells];
    const { output: _, exec_count: __, ...rest } = previous.cell;
    void _;
    void __;
    cells.splice(Math.min(previous.index, cells.length), 0, {
      ...rest,
      status: "Stale",
    });
    this.store.setState({
      cells,
      active: previous.cell.id,
      notice: null,
      dirty: true,
    });
    void this.write({ type: "upsert_cell", cell: previous.cell });
    void this.write({
      type: "move_cell",
      cell_id: previous.cell.id,
      to_index: Math.min(previous.index, cells.length - 1),
    });
  }
  move(id: string, offset: number) {
    const state = this.store.getState();
    const index = state.cells.findIndex((c) => c.id === id);
    const to = index + offset;
    if (index < 0 || to < 0 || to >= state.cells.length) return;
    const cells = [...state.cells];
    const [cell] = cells.splice(index, 1);
    if (!cell) return;
    cells.splice(to, 0, cell);
    this.store.setState({ cells, dirty: true });
    void this.write({ type: "move_cell", cell_id: id, to_index: to });
  }
  rename(title: string) {
    this.store.setState({ title, dirty: true });
    void this.write({ type: "rename_notebook", title });
  }
  async configure(change: Partial<KernelConfig["general"]>) {
    const old = this.store.getState().config;
    if (!old) return;
    const config = { ...old, general: { ...old.general, ...change } };
    await this.flush();
    const response = await this.kernel.request({ type: "set_config", config });
    if (response.type === "error") {
      this.fail(new Error(response.message));
      return;
    }
    this.store.setState({ config });
  }
  private valid(id: string, revision: number) {
    return (
      this.store.getState().cells.find((c) => c.id === id)?.revision ===
      revision
    );
  }
  async run(id: string, mode: "stay" | "next" | "insert" = "stay") {
    const cell = this.store.getState().cells.find((c) => c.id === id);
    if (!cell || cell.kind !== "Math" || this.store.getState().busy) return;
    const epoch = this.epoch;
    await this.flush();
    if (
      !this.valid(id, cell.revision) ||
      epoch !== this.epoch ||
      this.store.getState().busy
    )
      return;
    this.operations++;
    this.pendingOutputs.clear();
    this.revisions.clear();
    for (const c of this.store.getState().cells)
      this.revisions.set(c.id, c.revision);
    this.store.setState((state) => ({
      busy: true,
      error: null,
      cells: state.cells.map((c) =>
        c.id === id ? { ...c, status: "Running" } : c,
      ),
    }));
    try {
      const response = await this.kernel.request({
        type: "evaluate",
        cell_id: id,
        source: cell.source,
        dialect: cell.dialect,
      });
      if (epoch !== this.epoch) return;
      if (response.type === "error") throw new Error(response.message);
      if (response.type === "evaluated" && this.valid(id, cell.revision))
        this.pendingOutputs.set(id, {
          output: response.output,
          revision: cell.revision,
        });
      await this.flush();
      await this.refresh(false);
      if (this.valid(id, cell.revision) && mode !== "stay") {
        const state = this.store.getState();
        const index = state.cells.findIndex((c) => c.id === id);
        const next = state.cells[index + 1];
        if (mode === "insert" || !next) this.add("Math", "", id);
        else this.select(next.id, true);
      }
    } catch (error) {
      if (epoch === this.epoch) this.fail(error);
    } finally {
      if (epoch === this.epoch) {
        this.operations = Math.max(0, this.operations - 1);
        this.revisions.clear();
        this.pendingOutputs.clear();
        this.store.setState({ busy: this.operations > 0 });
      }
    }
  }
  async runAll() {
    if (this.store.getState().busy) return;
    const epoch = this.epoch;
    await this.flush();
    if (epoch !== this.epoch || this.store.getState().busy) return;
    this.pendingOutputs.clear();
    for (const cell of this.store.getState().cells)
      this.revisions.set(cell.id, cell.revision);
    this.operations++;
    this.store.setState({ busy: true, error: null });
    try {
      const response = await this.kernel.request({ type: "run_all" });
      if (epoch === this.epoch && response.type === "error")
        throw new Error(response.message);
      if (epoch === this.epoch) {
        await this.flush();
        await this.refresh(false);
      }
    } catch (error) {
      if (epoch === this.epoch) this.fail(error);
    } finally {
      if (epoch === this.epoch) {
        this.operations = Math.max(0, this.operations - 1);
        this.revisions.clear();
        this.pendingOutputs.clear();
        this.store.setState({ busy: false });
      }
    }
  }
  interrupt(): Promise<void> {
    if (this.restarting) return this.restarting;
    const epoch = ++this.epoch;
    this.operations = 0;
    this.revisions.clear();
    this.pendingOutputs.clear();
    this.writes = Promise.resolve();
    this.store.setState((state) => ({
      busy: true,
      cells: state.cells.map((c) =>
        c.status === "Running" || c.status === "Queued"
          ? { ...c, status: "Stale" }
          : c,
      ),
    }));
    this.restarting = (async () => {
      await this.kernel.interrupt();
      if (this.closed) return;
      await this.kernel.request({
        type: "set_system_language",
        language: locale("auto"),
      });
      const actual = await this.kernel.request({ type: "get_notebook_state" });
      const latest = this.file();
      if (
        actual.type === "notebook_state" &&
        JSON.stringify(actual.state.file) !== JSON.stringify(latest)
      ) {
        if (this.kernel.kind === "wasm") {
          const loaded = await this.kernel.request({
            type: "load_notebook",
            file: latest,
          });
          if (loaded.type === "error") throw new Error(loaded.message);
          await this.kernel.request({ type: "restore_definitions" });
        } else {
          const desired = new Set(latest.cells.map((c) => c.id));
          for (const c of actual.state.file.cells)
            if (!desired.has(c.id))
              await this.kernel.request({ type: "delete_cell", cell_id: c.id });
          for (const c of latest.cells)
            await this.kernel.request({ type: "upsert_cell", cell: c });
          for (let i = 0; i < latest.cells.length; i++) {
            const c = latest.cells[i];
            if (c)
              await this.kernel.request({
                type: "move_cell",
                cell_id: c.id,
                to_index: i,
              });
          }
          await this.kernel.request({
            type: "rename_notebook",
            title: latest.title,
          });
        }
      }
      await this.flush();
      await this.refresh(false);
    })()
      .catch((error) => {
        if (epoch === this.epoch) this.fail(error);
      })
      .finally(() => {
        this.restarting = undefined;
        if (epoch === this.epoch) this.store.setState({ busy: false });
      });
    return this.restarting;
  }
  async refresh(replace = true) {
    const response = await this.kernel.request({ type: "get_notebook_state" });
    if (response.type === "notebook_state")
      this.metadata(response.state, replace);
  }
  private metadata(state: NotebookState, replace: boolean) {
    this.store.setState((old) => ({
      title: replace ? state.file.title : old.title,
      cells: (replace ? state.file.cells.map(uiCell) : old.cells).map(
        (cell) => {
          const meta = state.cells.find((c) => c.id === cell.id);
          const source = state.file.cells.find((c) => c.id === cell.id);
          if (
            !meta ||
            !source ||
            source.source !== cell.source ||
            source.dialect !== cell.dialect ||
            source.kind !== cell.kind
          )
            return cell;
          const pending = this.pendingOutputs.get(cell.id);
          const output =
            pending &&
            pending.revision === cell.revision &&
            (meta.status === "Done" || meta.status === "Error")
              ? pending.output
              : undefined;
          const { exec_count: _, ...rest } = cell;
          void _;
          return {
            ...rest,
            status: meta.status,
            defines: meta.defines,
            uses: meta.uses,
            ...(meta.exec_count !== undefined
              ? { exec_count: meta.exec_count }
              : {}),
            ...(output ? { output } : {}),
          };
        },
      ),
    }));
  }
  private output(id: string, output: CellOutput) {
    const status =
      output.messages.some((m) => m.level === "Error") ||
      output.items.some((i) => i.type === "error")
        ? "Error"
        : "Done";
    this.store.setState((state) => ({
      cells: state.cells.map((c) =>
        c.id === id ? { ...c, output, status } : c,
      ),
    }));
  }
  private event(event: Event) {
    if (this.closed) return;
    if (event.type === "kernel_restarted") {
      this.store.setState((state) => ({
        notice: event.message,
        cells: state.cells.map((c) => {
          const { output: _, exec_count: __, ...rest } = c;
          void _;
          void __;
          return { ...rest, status: c.kind === "Text" ? "Done" : "Stale" };
        }),
      }));
      return;
    }
    if (event.type !== "cell_output" && event.type !== "cell_status") return;
    const revision = this.revisions.get(event.cell_id);
    if (revision !== undefined && !this.valid(event.cell_id, revision)) return;
    if (event.type === "cell_output") {
      if (this.operations > 0 && revision !== undefined)
        this.pendingOutputs.set(event.cell_id, {
          output: event.output,
          revision,
        });
      else this.output(event.cell_id, event.output);
    } else if (
      this.operations === 0 ||
      event.status === "Queued" ||
      event.status === "Running" ||
      event.status === "Stale"
    ) {
      this.store.setState((state) => ({
        cells: state.cells.map((c) =>
          c.id === event.cell_id ? { ...c, status: event.status } : c,
        ),
      }));
    }
  }
  file(): NotebookFile {
    const state = this.store.getState();
    return {
      version: 1,
      title: state.title,
      cells: state.cells.map(({ id, kind, source, dialect }) => ({
        id,
        kind,
        source,
        dialect,
      })),
    };
  }
  async load(file: NotebookFile) {
    if (this.store.getState().busy) await this.interrupt();
    this.epoch++;
    this.operations = 0;
    this.revisions.clear();
    this.pendingOutputs.clear();
    await this.flush();
    const response = await this.kernel.request({ type: "load_notebook", file });
    if (response.type === "error") throw new Error(response.message);
    this.store.setState({
      cells: file.cells.map(uiCell),
      title: file.title,
      active: file.cells[0]?.id ?? null,
      dirty: false,
      busy: false,
      notice: null,
    });
    await this.refresh();
  }
  markSaved(notice: string, expected?: NotebookFile) {
    this.store.setState({
      dirty: expected
        ? JSON.stringify(this.file()) !== JSON.stringify(expected)
        : false,
      notice,
    });
  }
  dispose() {
    this.closed = true;
    this.off();
  }
}
