import { createStore } from "zustand/vanilla";
import type { KernelClient, Request, Event } from "../kernel/client";
import type { KernelConfig } from "../kernel/generated/KernelConfig";
import type { CellInput } from "../kernel/generated/CellInput";
import type { CellStatus } from "../kernel/generated/CellStatus";
import type { CellOutput } from "../kernel/generated/CellOutput";
import type { NotebookState } from "../kernel/generated/NotebookState";
import type { NotebookFile } from "../kernel/generated/NotebookFile";
import { locale } from "../i18n";
import { observeKernel } from "../kernel/observedClient";
import { mergeConfig } from "../kernel/mirror";
import type { ChatTurnView } from "./assistant";
import { projectAcceptedSource, sameSource, sameNotebookSource } from './sourceTransaction';
import type {MathWork,MathResult} from '../agent/hostTypes';
export interface UiCell extends CellInput {
  revision: number;
  status: CellStatus;
  output?: CellOutput;
  outputId?: string;
  exec_count?: number;
  defines: string[];
  uses: string[];
  resultSource?: { source: string; dialect: CellInput["dialect"] };
  previousOutput?: CellOutput | undefined;
  stopped?: boolean;
  stopRequested?: boolean;
  textEditing?: boolean;
}
export interface UiState {
  assistantTurns: ChatTurnView[];
  documentGeneration: number;
  title: string;
  cells: UiCell[];
  active: string | null;
  focus: number;
  config: KernelConfig | null;
  busy: boolean;
  stopping: boolean;
  initializing: boolean;
  error: string | null;
  notice: string | null;
  dirty: boolean;
  panel: "variables" | "steps" | "assistant" | null;
  stepSelection: { cellId: string; outIndex: number } | null;
}
const initial: UiState = {
  assistantTurns: [],
  documentGeneration: 0,
  title: "",
  cells: [],
  active: null,
  focus: 0,
  config: null,
  busy: false,
  stopping: false,
  initializing: true,
  error: null,
  notice: null,
  dirty: false,
  panel: typeof window.matchMedia === "function" && window.matchMedia("(width < 760px)").matches ? null : "assistant",
  stepSelection: null,
};
const uiCell = (cell: CellInput): UiCell => ({
  ...cell,
  revision: 0,
  status: cell.kind === "Text" ? "Done" : "Stale",
  defines: [],
  uses: [],
});
export class NotebookController {
  readonly kernel: KernelClient;
  readonly ai: ReturnType<typeof observeKernel>;
  readonly store = createStore<UiState>(() => ({ ...initial }));
  private writes = Promise.resolve();
  private off: () => void;
  private closed = false;
  private epoch = 0;
  private operations = 0;
  private restarting: Promise<void> | undefined;
  private updating = false;
  private sourceReceipts=new Map<string,{generation:number;before:NotebookFile;after:NotebookFile;size:number}>();
  private receiptBytes=0;
  private revisions = new Map<string, number>();
  private pendingOutputs = new Map<
    string,
    { output: CellOutput; revision: number; outputId?:string }
  >();
  private removed: { cell: UiCell; index: number } | undefined;
  constructor(kernel: KernelClient) {
    this.ai = observeKernel(kernel);
    this.kernel = this.ai.client;
    this.off = this.kernel.onEvent((event) => this.event(event));
  }
  async initialize() {
    const epoch = this.epoch;
    try {
      await this.kernel.ready;
      await this.kernel.request({
        type: "set_system_language",
        language: locale("auto"),
      });
      const config = await this.kernel.request({ type: "get_config" });
      if (config.type !== "config")
        throw new Error("Kernel configuration unavailable");
      if (this.closed || epoch !== this.epoch) return;
      this.store.setState({ config: config.config });
      const snapshot = await this.kernel.request({ type: "get_notebook_state" });
      if (this.closed || epoch !== this.epoch) return;
      if (snapshot.type === "notebook_state") {
        // First-load state is authoritative only while the local source is still untouched.
        // Dirty source may have been edited after this snapshot was captured by the worker.
        this.metadata(snapshot.state, !this.store.getState().dirty);
      }
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
  async agentMath(work:MathWork,admit:()=>boolean,onStarted?:()=>void):Promise<MathResult>{
    const epoch=this.epoch;
    await this.flush();
    if(this.closed||epoch!==this.epoch||!admit()||!sameNotebookSource(this.file(),work.expected))throw new Error('STALE_SNAPSHOT');
    if(this.store.getState().busy)throw new Error('EDITING_BUSY');
    if(!this.kernel.agentMath)throw new Error('NOT_SUPPORTED');
    const running=work.command.type==='run'||work.command.type==='scratch';
    const revisions=new Map(this.store.getState().cells.map(cell=>[cell.id,cell.revision]));
    if(running){this.operations++;this.store.setState({busy:true});onStarted?.();}
    try{
      const result=await this.kernel.agentMath(work);
      if(this.closed||epoch!==this.epoch)throw new Error('UNKNOWN_OUTCOME');
      if(result.type==='ran'){
        result.outputId=crypto.randomUUID();
        const revision=revisions.get(result.cell_id);
        if(result.output&&revision!==undefined&&this.valid(result.cell_id,revision))this.pendingOutputs.set(result.cell_id,{output:result.output,revision,outputId:result.outputId});
        this.metadata(result.state,false);
        this.pendingOutputs.delete(result.cell_id);
      }
      return result;
    }finally{if(running&&epoch===this.epoch&&!this.closed){this.operations=Math.max(0,this.operations-1);this.store.setState({busy:this.operations>0});}}
  }
  /** Agent source changes share the manual write queue and never evaluate as a side effect. */
  sourceWasAccepted(operationId:string,before:NotebookFile,after:NotebookFile){const receipt=this.sourceReceipts.get(operationId);return !!receipt&&receipt.generation===this.store.getState().documentGeneration&&sameNotebookSource(receipt.before,before)&&sameNotebookSource(receipt.after,after);}
  async commitSource(expected: NotebookFile, desired: NotebookFile, admit: () => boolean, operationId?:string) {
    const previousReceipt=operationId?this.sourceReceipts.get(operationId):undefined;
    if(previousReceipt&&previousReceipt.generation===this.store.getState().documentGeneration){if(!sameNotebookSource(previousReceipt.before,expected)||!sameNotebookSource(previousReceipt.after,desired))throw new Error('PREVIEW_MISMATCH');return;}
    const epoch = this.epoch;
    let ownsBusy = false, synchronized: Promise<void> | undefined;
    const operation = this.writes.then(async () => {
      if (this.closed || epoch !== this.epoch || !admit() || !sameNotebookSource(this.file(), expected)) throw new Error('STALE_SNAPSHOT');
      if (this.store.getState().busy) throw new Error('EDITING_BUSY');
      const commit = this.kernel.commitSourceFile;
      if (!commit) throw new Error('NOT_SUPPORTED');
      ownsBusy = true; this.store.setState({ busy: true });
      const response = await commit.call(this.kernel, expected, desired);
      if (epoch !== this.epoch || this.closed) throw new Error('UNKNOWN_OUTCOME');
      if (response.type === 'error') throw new Error(response.message);
      if (response.type !== 'notebook_state') throw new Error('UNKNOWN_OUTCOME');
      if (!sameNotebookSource(response.state.file, desired)) throw new Error('UNKNOWN_OUTCOME');
      if(operationId){
        const measure=(file:NotebookFile)=>256+2*file.title.length+file.cells.reduce((n,cell)=>n+128+2*(cell.id.length+cell.source.length),0);
        const size=measure(expected)+measure(desired),old=this.sourceReceipts.get(operationId);if(old)this.receiptBytes-=old.size;
        this.sourceReceipts.set(operationId,{generation:this.store.getState().documentGeneration,before:structuredClone(expected),after:structuredClone(desired),size});this.receiptBytes+=size;
        while(this.sourceReceipts.size>64||this.receiptBytes>32*1024*1024){const [id,receipt]=this.sourceReceipts.entries().next().value!;this.sourceReceipts.delete(id);this.receiptBytes-=receipt.size;}
      }
      const merged = projectAcceptedSource(expected, desired, this.file());
      this.store.setState(state => {
        const previous = new Map(state.cells.map(cell => [cell.id, cell]));
        const cells = merged.cells.map(source => {
          const old = previous.get(source.id);
          if (!old) return uiCell(source);
          return sameSource(old, source) ? old : { ...old, ...source, revision: old.revision + 1, status: source.kind === 'Text' ? 'Done' as const : 'Stale' as const, stopped: false };
        });
        return { title: merged.title, cells, active: cells.some(cell => cell.id === state.active) ? state.active : cells[0]?.id ?? null, dirty: true };
      });
      // Edits queued while the source ACK was pending used the old visible order. Let them
      // finish, then reconcile the source projection once; edits after this point use merged UI.
      synchronized = this.writes.then(async () => {
        if (this.closed || epoch !== this.epoch) throw new Error('UNKNOWN_OUTCOME');
        const actual = await this.kernel.request({ type: 'get_notebook_state' });
        if (actual.type !== 'notebook_state') throw new Error('UNKNOWN_OUTCOME');
        const latest = this.file();
        if (!sameNotebookSource(actual.state.file, latest)) {
          const synced = await commit.call(this.kernel, actual.state.file, latest);
          if (synced.type !== 'notebook_state') throw new Error('UNKNOWN_OUTCOME');
        }
        await this.refresh(false);
      });
      this.writes = synchronized.catch(() => {});
    });
    this.writes = operation.catch(() => {});
    try { await operation; await synchronized; }
    finally { if (ownsBusy && epoch === this.epoch && !this.closed) this.store.setState({ busy: this.operations > 0 }); }
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
    if(kind==="Text") cell.textEditing=true;
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
    void this.write({ type: "upsert_cell", cell: { id, kind, source, dialect } });
    if (index < cells.length - 1)
      void this.write({ type: "move_cell", cell_id: id, to_index: index });
    return id;
  }
  select(id: string, focus = false) {
    if (!focus && this.store.getState().active === id) return;
    this.store.setState((state) => ({
      active: id,
      focus: state.focus + (focus ? 1 : 0),
    }));
  }
  textMode(id:string,editing:boolean) {this.store.setState(state=>({cells:state.cells.map(cell=>cell.id===id ? {...cell,textEditing:editing}:cell)}));}
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
      stopped: false,
      stopRequested: false,
      ...(change.kind==="Text" ? {textEditing:true}:{}),
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
    const {id,kind,source,dialect}=previous.cell;
    void this.write({ type: "upsert_cell", cell: {id,kind,source,dialect} });
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
  async configureLlm(llm: KernelConfig["llm"]): Promise<KernelConfig> {
    const old = this.store.getState().config;
    if (!old) throw new Error("Kernel configuration unavailable");
    const submitted = { ...old, llm };
    await this.flush();
    const result = await this.kernel.request({
      type: "set_config",
      config: submitted,
    });
    if (result.type === "error") throw new Error(result.message);
    const effective = await this.kernel.request({ type: "get_config" });
    if (effective.type !== "config")
      throw new Error("Kernel configuration unavailable");
    const config =
      this.kernel.kind === "wasm"
        ? mergeConfig(effective.config, mergeConfig(submitted, old))
        : effective.config;
    this.store.setState({ config });
    return config;
  }
  /** Explicit stale-result action: refresh required definitions using the kernel's graph order. */
  async update(id: string) {
    if (this.updating || this.store.getState().busy) return;
    const epoch = this.epoch;
    const revisions = new Map(this.store.getState().cells.map(cell => [cell.id, cell.revision]));
    this.updating = true;
    try {
      await this.flush();
      const response = await this.kernel.request({ type: 'get_notebook_state' });
      if (epoch !== this.epoch || response.type !== 'notebook_state') return;
      const snapshot = response.state, required = new Set<string>();
      const visit = (cellId: string) => {
        if (required.has(cellId)) return;
        required.add(cellId);
        const cell = snapshot.cells.find(cell => cell.id === cellId);
        for (const name of cell?.uses ?? [])
          for (const owner of snapshot.cells.filter(owner => owner.id !== cellId && owner.defines.includes(name))) visit(owner.id);
      };
      visit(id);
      const unchanged = () => epoch === this.epoch && [...required].every(cellId => {
        const cell = this.store.getState().cells.find(cell => cell.id === cellId);
        const source = snapshot.file.cells.find(cell => cell.id === cellId);
        return cell && source && cell.revision === revisions.get(cellId) && cell.source === source.source && cell.dialect === source.dialect;
      });
      if (!unchanged()) return;
      if (snapshot.cycles.some(cellId => required.has(cellId))) throw new Error('Cannot update a result with cyclic definitions.');
      this.metadata(snapshot, false);
      const queue = [...snapshot.definition_order.filter(cellId => required.has(cellId) && cellId !== id), id];
      for (const cellId of queue) {
        if (!unchanged() || this.store.getState().busy) return;
        const cell = this.store.getState().cells.find(cell => cell.id === cellId);
        if (cellId !== id && cell?.status === 'Done') continue;
        await this.run(cellId);
        if (!unchanged() || this.store.getState().cells.find(cell => cell.id === cellId)?.status !== 'Done') return;
      }
    } catch (error) { if (epoch === this.epoch) this.fail(error); }
    finally { this.updating = false; }
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
        c.id === id ? { ...c, status: "Running", stopped: false } : c,
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
    this.store.setState(state => ({ busy: true, error: null, cells: state.cells.map(cell => ({ ...cell, stopped: false })) }));
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
    const interrupted = new Map(this.store.getState().cells.filter(cell => cell.status === "Running" || cell.status === "Queued").map(cell => [cell.id, cell.revision]));
    let stopConfirmed=false;
    const epoch = ++this.epoch;
    this.operations = 0;
    this.revisions.clear();
    this.pendingOutputs.clear();
    this.writes = Promise.resolve();
    this.store.setState((state) => ({
      busy: true,
      stopping: true,
      cells: state.cells.map((c) =>
        c.status === "Running" || c.status === "Queued"
          ? { ...c, status: "Stale", stopRequested: true }
          : c,
      ),
    }));
    this.restarting = (async () => {
      await this.kernel.interrupt();
      stopConfirmed=true;
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
        if (epoch === this.epoch) this.store.setState(state => ({ busy: false, stopping: false, cells: state.cells.map(cell => ({...cell,stopRequested:false,...(stopConfirmed && interrupted.get(cell.id) === cell.revision ? {stopped:true}:{})})) }));
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
            ...(output ? { output, outputId:pending?.outputId??crypto.randomUUID(), resultSource: { source: source.source, dialect: source.dialect }, previousOutput: this.lastValid(cell, output) } : {}),
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
        c.id === id ? { ...c, output, outputId:crypto.randomUUID(), status, resultSource: { source: c.source, dialect: c.dialect }, previousOutput: this.lastValid(c, output) } : c,
      ),
    }));
  }
  private lastValid(cell: UiCell, output: CellOutput): CellOutput | undefined {
    if (!output.items.some(item => item.type === "error") && !output.messages.some(message => message.level === "Error")) return undefined;
    if (cell.output && !cell.output.items.some(item => item.type === "error") && !cell.output.messages.some(message => message.level === "Error")) return cell.output;
    return cell.previousOutput;
  }
  private event(event: Event) {
    if (this.closed) return;
    if (event.type === "kernel_restarted") {
      this.store.setState((state) => ({
        notice: event.message,
        cells: state.cells.map((c) => {
          // Old bytes remain a visibly stale presentation. No retained handle or inspector
          // is considered fresh after a worker restart.
          return { ...c, status: c.kind === "Text" ? "Done" : "Stale" };
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
      assistantTurns: [],
      documentGeneration: this.store.getState().documentGeneration + 1,
      title: file.title,
      active: file.cells[0]?.id ?? null,
      dirty: false,
      busy: false,
      stopping: false,
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
    this.ai.detach();
  }
}
