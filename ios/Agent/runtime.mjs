import {Agent} from '@earendil-works/pi-agent-core';
import {createAssistantMessageEventStream} from './pi-host.mjs';

// JavaScriptCore is an interpreter host, without Node, browser UI, fetch or filesystem IO.
if (!globalThis.AbortController) {
  globalThis.AbortController = class {
    constructor() { const listeners=new Set(); this.signal={aborted:false,reason:null,
      addEventListener:(_name,callback)=>listeners.add(callback),removeEventListener:(_name,callback)=>listeners.delete(callback),
      throwIfAborted(){if(this.aborted)throw new Error('Stopped');}};this.listeners=listeners; }
    abort(){if(this.signal.aborted)return;this.signal.aborted=true;this.signal.reason=new Error('Stopped');for(const callback of this.listeners)callback();}
  };
}
const pending=new Map();let sequence=0,agent;
const host=(op,payload)=>new Promise((resolve,reject)=>{const id=String(++sequence);pending.set(id,{resolve,reject});nativePost(JSON.stringify({id,op,payload}));});
globalThis.mobileResolve=(id,json,error)=>{const request=pending.get(id);if(!request)return;pending.delete(id);if(error)request.reject(new Error(error));else request.resolve(JSON.parse(json));};
globalThis.mobileStop=()=>agent?.abort();
const uuid=()=>nativeUUID();
const usage=()=>({input:0,output:0,cacheRead:0,cacheWrite:0,totalTokens:0,cost:{input:0,output:0,cacheRead:0,cacheWrite:0,total:0}});

globalThis.mobileStart=async json=>{
  const task=JSON.parse(json),history=task.history.slice(),results=task.results.slice();let rounds=0,calls=0;
  const canonical=(role,parts,turn=null)=>({record_type:'canonical_agent_message',message_version:1,message_id:uuid(),role,task_id:task.id,agent_turn_id:turn,parts});
  const user=canonical('user',task.parts);
  await host('record',{record:{canonical:user,replay:null},presentation:task.text});history.push({canonical:user,replay:null});
  const declarations=task.tools, completed=new Set(), proposed=new Map();
  const recordTool=async(id,reply)=>{
    const message=canonical('tool',[{kind:'tool_result',part_id:uuid(),tool_call_id:id,operation_ref:reply.operation_ref,result_ref:reply.request_ref,outcome:reply.status}],proposed.get(id)??null);
    const text=JSON.stringify(reply);results.push({message_id:message.message_id,tool_call_id:id,text,is_error:!reply.ok});
    const record={canonical:message,replay:null};await host('record',{record,result:results.at(-1)});history.push(record);completed.add(id);
  };
  const tools=declarations.map(tool=>({...tool,label:tool.name,execute:async(id,args)=>{
    if(++calls>80)throw new Error('Tool budget reached');
    const reply=await host('tool',{name:tool.name,args,call:id});
    await recordTool(id,reply);
    const text=JSON.stringify(reply);
    return {content:[{type:'text',text}],details:{reply},isError:!reply.ok,terminate:reply.status==='cancelled'||reply.status==='unknown'};
  }}));
  const model={id:task.model,name:task.model,api:'openai-completions',provider:'openmath-mobile',baseUrl:'',reasoning:false,input:['text'],contextWindow:task.contextWindow,maxTokens:task.maxTokens,cost:{input:0,output:0,cacheRead:0,cacheWrite:0}};
  const streamFn=()=>{
    const stream=createAssistantMessageEventStream();
    void(async()=>{
      const partial={role:'assistant',api:model.api,provider:model.provider,model:model.id,content:[],usage:usage(),stopReason:'stop',timestamp:Date.now()};
      try {
        if(++rounds>32)throw new Error('Model budget reached');
        const message=uuid(),turn=uuid();
        const result=await host('model',{message,turn,request:{binding:task.binding,api_model:task.model,messages:history,results,images:task.images.map(image=>({...image,message_id:image.message_id==='current'?user.message_id:image.message_id})),tools:declarations,max_output_tokens:task.maxTokens,temperature:null,stream:true}});
        if(result.finish==='incomplete')throw new Error('Incomplete model response');
        for(const part of result.message.parts)if(part.kind==='tool_call')proposed.set(part.tool_call_id,turn);
        const record={canonical:result.message,replay:result.replay??null};await host('record',{record,usage:result.usage});history.push(record);
        const content=result.message.parts.map(part=>part.kind==='text'?{type:'text',text:part.text}:{type:'toolCall',id:part.tool_call_id,name:part.name,arguments:JSON.parse(part.arguments_json)});
        const reply={...partial,content,stopReason:result.finish==='tool_calls'?'toolUse':'stop'};
        stream.push({type:'done',reason:reply.stopReason,message:reply});
      } catch(error) {stream.push({type:'error',reason:'error',error:{...partial,stopReason:'error',errorMessage:error.message}});}
    })();return stream;
  };
  agent=new Agent({initialState:{model,tools},toolExecution:'sequential',streamFn,maxRetryDelayMs:0,getApiKey:()=>undefined});
  agent.subscribe(async event=>{
    if(event.type==='tool_execution_end'&&!completed.has(event.toolCallId)) {
      const reply=await host('rejected',{name:event.toolName,call:event.toolCallId,message:event.result.content?.filter(part=>part.type==='text').map(part=>part.text).join('\n')??'Tool invocation rejected'});
      await recordTool(event.toolCallId,reply);
    }
  });
  try {
    await agent.prompt(task.text);
    const last=agent.state.messages.at(-1);
    await host('settled',{reason:last?.stopReason==='error'?'failed':last?.stopReason==='aborted'?'stopped':'ended',error:last?.errorMessage??null,rounds,calls});
  } catch(error) {await host('settled',{reason:'failed',error:error.message,rounds,calls});}
  finally {agent=undefined;}
};

const startWithFailure=globalThis.mobileStart;
globalThis.mobileStart=json=>{void startWithFailure(json).catch(error=>host('settled',{reason:'failed',error:error.message,rounds:0,calls:0}));};
