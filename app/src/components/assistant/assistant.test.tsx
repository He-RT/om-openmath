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

test('composer guards IME and clears only the accepted snapshot, retaining new typing and failed inputs',async()=>{
  vi.stubGlobal('ResizeObserver',class{observe(){}disconnect(){}});
  const {Composer}=await import('./Composer');
  const profile={name:'p',kind:'openai_chat',base_url:'https://configured.invalid/v1',model:'configured-model',api_key:'***',api_key_env:null,extra_headers:{},temperature:0.2,max_tokens:512,supports_tools:true,supports_json_mode:false,timeout_ms:1000} as import('../../kernel/generated/ProfileConfig').ProfileConfig;
  let accept!:(value:boolean)=>void;
  const send=vi.fn(()=>new Promise<boolean>(resolve=>{accept=resolve;}));
  render(<Composer models={[profile]} model="p" onModel={()=>{}} mode="execute" onMode={()=>{}} cells={[]} t={t} running={false} stopping={false} disabled={false} onSend={send} onStop={()=>{}} onSettings={()=>{}} onContext={()=>{}}/>);
  const input=screen.getByRole('textbox',{name:t.chatInput});
  fireEvent.change(input,{target:{value:'first'}});
  fireEvent.compositionStart(input);fireEvent.keyDown(input,{key:'Enter'});expect(send).not.toHaveBeenCalled();
  fireEvent.compositionEnd(input);fireEvent.keyDown(input,{key:'Enter',keyCode:229});expect(send).not.toHaveBeenCalled();
  fireEvent.keyDown(input,{key:'Enter',shiftKey:true});expect(send).not.toHaveBeenCalled();
  fireEvent.keyDown(input,{key:'Enter'});expect(send).toHaveBeenCalledTimes(1);expect((input as HTMLTextAreaElement).value).toBe('first');
  fireEvent.change(input,{target:{value:'next draft'}});await act(async()=>accept(true));expect((input as HTMLTextAreaElement).value).toBe('next draft');
  fireEvent.keyDown(input,{key:'Enter'});await act(async()=>accept(false));expect((input as HTMLTextAreaElement).value).toBe('next draft');
  fireEvent.keyDown(input,{key:'Enter'});await act(async()=>accept(true));expect((input as HTMLTextAreaElement).value).toBe('');
});

test('agent projection pairs original call IDs in order and uses receipts rather than prose',async()=>{
  const {projectConversation}=await import('../../agent/conversation');
  const {actionStatus}=await import('./ActionLog');
  const canonical=(role:'user'|'assistant',id:string,parts:import('../../../../agent/src/generated/protocol').AgentCanonicalMessage['parts'])=>({canonical:{record_type:'canonical_agent_message' as const,message_version:1 as const,message_id:id,role,task_id:'task',agent_turn_id:null,parts},replay:null});
  const payloads:import('../../agent/store').AgentEventPayload[]=[{kind:'message',record:canonical('user','user',[{kind:'text',part_id:'t',text:'Do it'}]),modelId:'actual-model',mode:'execute'},{kind:'delta',taskId:'task',messageId:'assistant',turnId:'round',text:'Claimed complete'},{kind:'message',record:canonical('assistant','assistant',[{kind:'text',part_id:'t',text:'Claimed complete'},{kind:'tool_call',part_id:'tool',tool_call_id:'original-call',name:'preview_source',arguments_json:'{}'}])},{kind:'tool',name:'preview_source',callId:'original-call',operationRef:null,argumentsJson:'{}',resultJson:JSON.stringify({ok:false,status:'failed',error:{code:'INVALID_ARGUMENT',message:'Missing input'}})},{kind:'settled',taskId:'task',mode:'execute',reason:'ended',requests:1,tools:1,durationMs:200,error:null}];
  const events=payloads.map((payload,index)=>({id:String(index),documentId:'d',sessionId:'s',sequence:index+1,time:index,payload}));
  const turn=projectConversation(events)[0]!;expect(turn.modelId).toBe('actual-model');expect(turn.parts).toHaveLength(2);expect(turn.parts[0]).toMatchObject({kind:'text',text:'Claimed complete',id:'assistant:text'});
  const actions=turn.parts.flatMap(part=>part.kind==='actions'?part.actions:[]);expect(actions[0]?.id).toBe('original-call');expect(actionStatus(actions[0]!)).toBe('failed');expect(turn.settled?.reason).toBe('ended');
  const {TurnOutcome}=await import('./TurnOutcome');render(<TurnOutcome turn={turn} cells={[]} dirty={true} t={t} onUndo={async()=>{}} onLocate={()=>{}}/>);
  expect(screen.getByLabelText('Turn result').textContent).not.toContain('Claimed complete');expect(screen.getByLabelText('Turn result').textContent).not.toContain('Added');
});

test('source diff preserves changed lines and bounded context without dropping whitespace',async()=>{
  const {sourceDiff}=await import('./SourceDiff');expect(sourceDiff('a\n  old\nz','a\n  new\nz')).toEqual([{kind:'context',line:'a'},{kind:'remove',line:'  old'},{kind:'add',line:'  new'},{kind:'context',line:'z'}]);expect(sourceDiff(undefined,'new')).toEqual([{kind:'add',line:'new'}]);
});

test('unknown source changes become facts only when the same operation receives a confirmed transaction',async()=>{
  const {projectConversation,turnTransactions}=await import('../../agent/conversation');const {actionStatus}=await import('./ActionLog');
  const file={version:1 as const,title:'',cells:[]};const transaction={id:'tx',documentId:'d',sessionId:'s',taskId:'task',before:file,after:file,changedCellIds:[],createdAt:1,undoneBy:null};
  const message=(role:'user'|'assistant',id:string,parts:import('../../agent/modelTypes').AgentCanonicalMessage['parts'])=>({kind:'message' as const,record:{canonical:{record_type:'canonical_agent_message' as const,message_version:1 as const,role,message_id:id,task_id:'task',agent_turn_id:null,parts},replay:null}});
  const payloads:import('../../agent/store').AgentEventPayload[]=[message('user','u',[{kind:'text',part_id:'t',text:'edit'}]),message('assistant','a',[{kind:'tool_call',part_id:'c',tool_call_id:'write',name:'apply_notebook_patch',arguments_json:'{}'}]),{kind:'tool',callId:'write',name:'apply_notebook_patch',argumentsJson:'{}',operationRef:'op',resultJson:JSON.stringify({ok:false,status:'unknown',operation_ref:'op',data:null})},message('assistant','b',[{kind:'tool_call',part_id:'c',tool_call_id:'check',name:'get_operation_status',arguments_json:'{"operation_ref":"op"}'}]),{kind:'tool',callId:'check',name:'get_operation_status',argumentsJson:'{}',operationRef:null,resultJson:JSON.stringify({ok:true,status:'completed',data:{effect_committed:true,transaction_ref:'tx'}})}];
  const events=payloads.map((payload,index)=>({id:String(index),documentId:'d',sessionId:'s',sequence:index+1,time:index,payload}));
  expect(turnTransactions(projectConversation(events.slice(0,3))[0]!)).toHaveLength(0);
  const turn=projectConversation(events,new Map([['tx',transaction]]))[0]!,action=turn.parts.flatMap(part=>part.kind==='actions'?part.actions:[])[0]!;expect(actionStatus(action)).toBe('ok');expect(turnTransactions(turn)).toEqual([transaction]);expect(action.reply?.status).toBe('unknown');
});
