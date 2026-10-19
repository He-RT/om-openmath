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
for (const mode of ['success','failure','edit'] as const) test(`explicit update follows the kernel dependency order and stops on ${mode}`, async()=>{
  const statuses=new Map<string,import('../kernel/generated/CellStatus').CellStatus>();
  const kernel=new MockKernel(request=>{
    if(request.type==='get_notebook_state')return {type:'notebook_state',state:{
      file:controller.file(),cells:controller.store.getState().cells.map(cell=>({id:cell.id,status:statuses.get(cell.id)??'Stale',defines:cell.defines,uses:cell.uses})),
      definition_order:['a','b','unrelated'],cycles:[],
    }};
    if(request.type==='evaluate'){
      statuses.set(request.cell_id,mode==='failure'?'Error':'Done');
      if(mode==='edit')controller.edit('target',{source:'b+2'});
      return {type:'evaluated',cell_id:request.cell_id,output:{items:[],messages:mode==='failure'?[{level:'Error',symbol:'test',tag:'test',text:'definition failed'}]:[],timing_ms:0},reran:[]};
    }
    return {type:'ok'};
  });
  const controller:NotebookController=new NotebookController(kernel);
  const rows=[['target','b+1',[],['b']],['b','let b=a+1',['b'],['a']],['unrelated','let other=99',['other'],[]],['a','let a=5',['a'],[]]] as const;
  controller.store.setState({cells:rows.map(([id,source,defines,uses])=>({id,source,kind:'Math',dialect:'Modern',status:'Stale',revision:0,defines:[...defines],uses:[...uses]}))});
  await controller.update('target');
  expect(kernel.requests.filter(request=>request.type==='evaluate').map(request=>request.cell_id)).toEqual(mode==='success'?['a','b','target']:['a']);
  expect(controller.store.getState().busy).toBe(false);
  if(mode!=='success')expect(controller.store.getState().cells.find(cell=>cell.id==='target')?.status).toBe('Stale');
  controller.dispose();
});
test("failed output keeps the last valid result and restart preserves only its stale presentation",async()=>{
  let input: import("../kernel/generated/CellInput").CellInput | null=null,failed=false;
  const good: import("../kernel/generated/CellOutput").CellOutput={items:[{type:"expr",out_index:1,input_form:"4",modern_form:"4",latex:"4",nature:"exact",unchanged:false}],messages:[],timing_ms:1};
  const bad: import("../kernel/generated/CellOutput").CellOutput={items:[{type:"error",message:"incomplete",span:{start:0,end:1}}],messages:[],timing_ms:2};
  const kernel=new MockKernel(request=>{
    if(request.type==="get_config") return {type:"config",config};
    if(request.type==="upsert_cell") {input=request.cell;return {type:"ok"};}
    if(request.type==="evaluate") return {type:"evaluated",cell_id:request.cell_id,output:failed?bad:good,reran:[]};
    if(request.type==="get_notebook_state") return {type:"notebook_state",state:{file:{version:1,title:"",cells:input?[input]:[]},cells:input?[{id:input.id,status:failed?"Error":"Done",defines:[],uses:[],exec_count:1}]:[],definition_order:[],cycles:[]}};
    return {type:"ok"};
  });
  const controller=new NotebookController(kernel);await controller.initialize();
  const id=controller.add("Math","2+2");await controller.run(id);
  expect(controller.store.getState().cells[0]?.resultSource?.source).toBe("2+2");
  failed=true;controller.edit(id,{source:"("});await controller.run(id);
  const broken=controller.store.getState().cells[0]!;
  expect(broken.status).toBe("Error");expect(broken.previousOutput).toEqual(good);expect(broken.output).toEqual(bad);
  expect(controller.file().cells[0]).toEqual({id,kind:"Math",source:"(",dialect:"Modern"});
  kernel.emit({type:"kernel_restarted",message:"worker restarted"});
  expect(controller.store.getState().cells[0]?.status).toBe("Stale");expect(controller.store.getState().cells[0]?.previousOutput).toEqual(good);
  controller.dispose();
});
test("an unconfirmed interruption never claims a cell has stopped",async()=>{
  const kernel=new MockKernel(()=>({type:"ok"})),controller=new NotebookController(kernel);
  const id=controller.add("Math","2+2");await controller.flush();
  controller.store.setState(state=>({busy:true,cells:state.cells.map(cell=>({...cell,status:"Running"}))}));
  kernel.interrupt=async()=>{throw new Error("interrupt unavailable");};
  await controller.interrupt();
  expect(controller.store.getState().cells.find(cell=>cell.id===id)?.stopped).not.toBe(true);
  expect(controller.store.getState().stopping).toBe(false);expect(controller.store.getState().error).toContain("interrupt unavailable");
  controller.dispose();
});
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

test("a late startup snapshot cannot replace source edited while initialization is awaiting readback", async () => {
  let frozen: import("../kernel/generated/NotebookFile").NotebookFile = { version: 1, title: "", cells: [] };
  let release: ((response: Response) => void) | undefined;
  let requested!: () => void;
  const snapshotStarted = new Promise<void>((resolve) => { requested = resolve; });
  const kernel = new MockKernel((request) => {
    if (request.type === "get_config") return { type: "config", config };
    if (request.type === "upsert_cell") {
      const index = frozen.cells.findIndex((cell) => cell.id === request.cell.id);
      frozen = { ...frozen, cells: index < 0 ? [...frozen.cells, request.cell] : frozen.cells.map((cell, i) => i === index ? request.cell : cell) };
      return { type: "ok" };
    }
    if (request.type === "get_notebook_state") {
      const snapshot = structuredClone(frozen);
      requested();
      return new Promise<Response>((resolve) => {
        release = () => resolve({ type: "notebook_state", state: { file: snapshot, cells: snapshot.cells.map((cell) => ({ id: cell.id, status: "Stale", defines: [], uses: [] })), definition_order: [], cycles: [] } });
      });
    }
    return { type: "ok" };
  });
  const controller = new NotebookController(kernel);
  const id = controller.add("Math", "1+1");
  await controller.flush();
  const startup = controller.initialize();
  await snapshotStarted;
  controller.edit(id, { source: "let f(x)=\n x+1;f(4)" });
  await controller.flush();
  release?.({ type: "ok" });
  await startup;
  expect(controller.store.getState().cells[0]?.source).toBe("let f(x)=\n x+1;f(4)");
  expect(controller.store.getState().dirty).toBe(true);
  controller.dispose();
});

for(const kind of ['run','scratch'] as const)test(`accepted agent ${kind} reports owned work and clears it after a failure`,async()=>{
  let release!:()=>void,start!:()=>void;const gate=new Promise<void>(resolve=>{release=resolve;}),started=new Promise<void>(resolve=>{start=resolve;});
  const kernel:import('../kernel/client').KernelClient=new MockKernel(()=>({type:'ok'}));kernel.agentMath=async()=>{await gate;throw new Error('synthetic terminal');};
  const controller=new NotebookController(kernel);controller.store.setState({config});const id=controller.add('Math','2+2');await controller.flush();const before=controller.file();
  const command:import('../agent/hostTypes').MathCommand=kind==='run'?{type:'run',cell_id:id}:{type:'scratch',source:'2+2',dialect:'Modern',use_notebook_definitions:false};
  const result=controller.agentMath({expected:before,general:config.general,command},()=>true,start);await started;
  expect(controller.store.getState().agentWork).toEqual(kind==='run'?{kind:'run',cellId:id}:{kind:'scratch'});expect(controller.store.getState().cells[0]?.status).toBe(kind==='run'?'Running':'Stale');
  release();await expect(result).rejects.toThrow('synthetic terminal');expect(controller.store.getState().agentWork).toBeNull();expect(controller.store.getState().busy).toBe(false);expect(controller.file()).toEqual(before);expect(controller.store.getState().cells[0]?.status).toBe('Stale');controller.dispose();
});
