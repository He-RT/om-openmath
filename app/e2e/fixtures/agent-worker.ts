/// <reference lib="webworker" />
import {Agent,type AgentTool} from '@earendil-works/pi-agent-core';
import type {Api,Model,AssistantMessage} from '@earendil-works/pi-ai';
import init,{Kernel} from '../../src/kernel/wasm/om_wasm.js';
import {BrowserModelAdapter} from '../../src/agent/modelAdapter';
import type {AgentCanonicalMessage,CanonicalRecord,ModelInput,ModelResult} from '../../src/agent/modelTypes';
let agent:Agent|undefined;
const textMessage=(id:string,role:'system'|'user',text:string):AgentCanonicalMessage=>({record_type:'canonical_agent_message',message_version:1,message_id:id,role,task_id:'task-fixture',agent_turn_id:null,parts:[{kind:'text',part_id:id+'-part',text}]});
self.onmessage=event=>{
  if(event.data.type==='stop'){agent?.abort();return;}
  void run(event.data.endpoint).catch(()=>self.postMessage({type:'result',ok:false,error:'Worker fixture failed'}));
};
async function run(endpoint:string){
  await init();const kernel=new Kernel();let sequence=0;
  const request=(body:unknown)=>JSON.parse(kernel.request(JSON.stringify({id:++sequence,body}))).response.body;
  for(const [id,source]of [['a','let a=2'],['b','a+1']])request({type:'upsert_cell',cell:{id,source,kind:'Math',dialect:'Modern'}});
  const history:CanonicalRecord[]=[{canonical:textMessage('system-fixture','system','使用工具读取笔记本。'),replay:null},{canonical:textMessage('user-fixture','user','读取当前笔记本'),replay:null}];
  const results:ModelInput['results']=[],rounds:ModelResult[]=[],persisted=new Set<string>();
  let requests=0,reads=0;
  const adapter=new BrowserModelAdapter(result=>rounds.push(result));
  const persist=(message:AssistantMessage)=>{
    if(message.stopReason==='error'||message.stopReason==='aborted')return;
    const result=adapter.result(message);
    if(!persisted.has(result.message.message_id)){persisted.add(result.message.message_id);history.push({canonical:result.message,replay:result.replay});}
  };
  const tools:AgentTool[]=[{name:'read_notebook',label:'Read notebook',description:'Read current notebook',parameters:{type:'object',properties:{},additionalProperties:false} as AgentTool['parameters'],execute:async(callId)=>{
    reads++;const file=request({type:'get_notebook_state'}).state.file;
    const messageId='tool-fixture-'+reads;
    history.push({canonical:{record_type:'canonical_agent_message',message_version:1,message_id:messageId,role:'tool',task_id:'task-fixture',agent_turn_id:'turn-fixture-'+requests,parts:[{kind:'tool_result',part_id:messageId+'-part',tool_call_id:callId,operation_ref:null,result_ref:'result-fixture-'+reads,outcome:'completed'}]},replay:null});
    results.push({message_id:messageId,tool_call_id:callId,text:JSON.stringify(file),is_error:false});
    return {content:[{type:'text',text:JSON.stringify(file)}],details:{count:file.cells.length}};
  }}];
  const model:Model<Api>={id:'loopback',name:'Loopback',api:'openai-completions',provider:'fixture',baseUrl:'',reasoning:false,input:['text'],cost:{input:0,output:0,cacheRead:0,cacheWrite:0},contextWindow:8192,maxTokens:1024};
  agent=new Agent({initialState:{model,tools},toolExecution:'sequential',streamFn:adapter.stream,
    prepareRequest:async()=>{
      requests++;adapter.prepare({requestId:'request-fixture-'+requests,messageId:'assistant-fixture-'+requests,taskId:'task-fixture',turnId:'turn-fixture-'+requests,url:endpoint,headers:[['Content-Type','application/json']],timeoutMs:3000,
        input:{binding:{provider_ref:'provider-fixture',route_ref:'route-fixture',model_ref:'model-fixture',adapter_version:'single-turn-v1',protocol:'chat_completions'},api_model:'loopback',messages:history,results,tools:[{name:'read_notebook',description:'Read current notebook',parameters:{type:'object',properties:{},additionalProperties:false}}],max_output_tokens:1024,temperature:null,stream:true}});
      self.postMessage({type:'request',count:requests});
    },
    beforeToolCall:async(context)=>{persist(context.assistantMessage);},
    finishTurn:async(context)=>{persist(context.message);},
  });
  try{
    await agent.prompt('读取当前笔记本');
    const last=agent.state.messages.at(-1),reason=last?.role==='assistant'?last.stopReason:null;
    self.postMessage({type:'result',ok:reason!=='error'&&reason!=='aborted'&&!agent.state.errorMessage,requests,reads,rounds,history,reason,error:agent.state.errorMessage??null});
  }finally{kernel.free();agent=undefined;}
}
