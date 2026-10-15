import {randomUUID,createHash} from 'node:crypto';
import {readFileSync} from 'node:fs';
import {createAssistantMessageEventStream, type AssistantMessage,type AssistantMessageEventStream,type Api,type Model} from '@earendil-works/pi-ai';
import type {AgentTool} from '@earendil-works/pi-agent-core';
import type {TSchema} from 'typebox';
import {AgentRuntime} from './runtime.ts';
import {notebookPatchResult} from './tools/notebook.ts';
import {encodeFrame,decodeCanonical,decodeToolResult,schemaHash,strictJson} from './protocol.ts';
import type {AgentIPCFrame,AgentHelloFrame,AgentCanonicalMessage,AgentModelRequestPayload,AgentToolResultPayload,AgentToolName} from './generated/protocol.ts';

const toolSchema=JSON.parse(readFileSync(new URL('../../docs/design/agent-tools.schema.json',import.meta.url),'utf8'));
const coreVersion=JSON.parse(readFileSync(new URL('../node_modules/@earendil-works/pi-agent-core/package.json',import.meta.url),'utf8')).version;
const aiVersion=JSON.parse(readFileSync(new URL('../node_modules/@earendil-works/pi-ai/package.json',import.meta.url),'utf8')).version;
export const ADAPTER_VERSION='openmath-adapter-1';
const model:Model<Api>={id:'native-controlled',name:'Native transport carrier',api:'openai-completions',provider:'openmath-native',baseUrl:'',reasoning:false,input:['text'],cost:{input:0,output:0,cacheRead:0,cacheWrite:0},contextWindow:4096,maxTokens:256};
const usage={input:0,output:0,cacheRead:0,cacheWrite:0,totalTokens:0,cost:{input:0,output:0,cacheRead:0,cacheWrite:0,total:0}};
function argumentsSchema(name:string):TSchema {
 const expand=(value:any,depth=0):any=>{
  if(depth>64)throw new Error('TOOL_SCHEMA_DEPTH');
  if(value&&typeof value==='object'){
   if(value.$ref)return expand(toolSchema.$defs[value.$ref.slice(8)],depth+1);
   if(Array.isArray(value))return value.map(v=>expand(v,depth+1));
   return Object.fromEntries(Object.entries(value).map(([k,v])=>[k,expand(v,depth+1)]));
  }return value;
 };
 return expand(toolSchema.$defs[name+'_args']);
}
interface Pending {resolve:()=>void;reject:(error:Error)=>void;event:string}
/** A single private helper activation. Canonical data stays authoritative in the native owner. */
export class AgentBridge {
 #hello?:AgentHelloFrame;#runtime?:AgentRuntime;#sequence=0;#hostSequence=0;#sourceSequence=0;#bytes=0;#closed=false;#active=false;
 #seen=new Map<string,{hash:string,sequence:number}>();#ack=new Map<string,Pending>();
 #canonical=new WeakMap<AssistantMessage,AgentCanonicalMessage>();#events=new Map<string,string>();
 #request?:AgentModelRequestPayload;#prepared?:{resolve:(payload:AgentModelRequestPayload)=>void,reject:(error:Error)=>void};
 #stream?:AssistantMessageEventStream;#offset=0;#requestTerminal=false;#partial?:AssistantMessage;#cancelled=false;#failure=false;
 #tools=new Map<string,{resolve:(result:AgentToolResultPayload)=>void,reject:(error:Error)=>void,turn:string,name:string}>();
 #results=new Map<string,AgentToolResultPayload>();#writes=Promise.resolve();
 readonly #write:(bytes:Uint8Array)=>Promise<void>;readonly #settled:()=>void;
 constructor(write:(bytes:Uint8Array)=>Promise<void>,settled:()=>void){this.#write=write;this.#settled=settled;}
 async #send(type:AgentIPCFrame['type'],payload:unknown,reply_to:string|null=null,register?:(id:string)=>void):Promise<string>{
  if(!this.#hello||this.#closed||this.#sequence>=4096)throw new Error('IPC_CONNECTION_CLOSED');
  const h=this.#hello,message_id='helper-'+randomUUID(),sequence=++this.#sequence;
  const frame={protocol_version:1,connection_id:h.connection_id,connection_generation:h.connection_generation,runtime_instance_id:h.runtime_instance_id,agent_session_id:h.agent_session_id,document_id:h.document_id,document_generation:h.document_generation,task_id:h.task_id,task_generation:h.task_generation,message_id,sequence,reply_to,type,payload} as AgentIPCFrame;
  const bytes=encodeFrame(frame);register?.(message_id);this.#writes=this.#writes.then(()=>this.#write(bytes));await this.#writes;return message_id;
 }
 async accept(frame:AgentIPCFrame,raw:Uint8Array):Promise<void>{
  if(this.#closed)throw new Error('IPC_CONNECTION_CLOSED');
  if(!this.#hello){
   if(frame.type!=='hello'||frame.sequence!==1||frame.payload.core_version!==coreVersion||coreVersion!==aiVersion||coreVersion!=='1.0.4'||frame.payload.adapter_version!==ADAPTER_VERSION||frame.payload.schema_hash!==schemaHash)throw new Error('IPC_HANDSHAKE_MISMATCH');
   this.#hello=frame;this.#runtime=this.#createRuntime(frame.task_id);
  }else{
   const h=this.#hello;
   for(const key of ['runtime_instance_id','document_id','document_generation','agent_session_id','task_id','task_generation','connection_id','connection_generation'] as const)if(frame[key]!==h[key])throw new Error('STALE_AGENT_SCOPE');
  }
  const hash=createHash('sha256').update(raw).digest('hex'),old=this.#seen.get(frame.message_id);
  if(old){if(old.hash!==hash||old.sequence!==frame.sequence)throw new Error('IPC_ID_CONFLICT');return;}
  if(frame.sequence!==this.#hostSequence+1||this.#seen.size>=4096||this.#bytes+raw.length>8*1024*1024)throw new Error('IPC_SEQUENCE_OR_BUDGET');
  this.#seen.set(frame.message_id,{hash,sequence:frame.sequence});this.#hostSequence=frame.sequence;this.#bytes+=raw.length;
  switch(frame.type){
   case 'hello':await this.#send('ready',{core_version:coreVersion,adapter_version:ADAPTER_VERSION,schema_hash:schemaHash},frame.message_id);break;
   case 'user_input':
    if(this.#active)throw new Error('AGENT_TASK_ACTIVE');this.#validateInput(frame.payload.message);this.#active=true;
    void this.#runtime!.prompt(frame.payload.message).then(()=>this.#stop(this.#cancelled?'cancelled':(this.#failure||this.#runtime!.failed)?'failed':'loop_stopped'),()=>this.#stop(this.#cancelled?'cancelled':'failed')).catch(()=>this.#settled());break;
   case 'steer':this.#validateInput(frame.payload.message);if(!this.#active)throw new Error('AGENT_TASK_NOT_ACTIVE');this.#runtime!.steer(frame.payload.message);break;
   case 'follow_up':this.#validateInput(frame.payload.message);if(!this.#active)throw new Error('AGENT_TASK_NOT_ACTIVE');this.#runtime!.followUp(frame.payload.message);break;
   case 'ack':{
    const pending=frame.reply_to&&this.#ack.get(frame.reply_to);if(!pending||pending.event!==frame.payload.event_id||frame.payload.event_sequence===0)throw new Error('IPC_UNEXPECTED_ACK');
    this.#ack.delete(frame.reply_to!);pending.resolve();break;
   }
   case 'model_request':{
    if(!this.#request||!this.#prepared||frame.payload.request_id!==this.#request.request_id||frame.payload.agent_turn_id!==this.#request.agent_turn_id)throw new Error('IPC_REQUEST_SCOPE_MISMATCH');
    this.#request=frame.payload;const prepared=this.#prepared;this.#prepared=undefined;prepared.resolve(frame.payload);break;
   }
   case 'model_event':this.#modelEvent(frame.payload);break;
   case 'tool_result':{
    const p=frame.payload,pending=this.#tools.get(p.tool_call_id);if(!pending||pending.turn!==p.agent_turn_id)throw new Error('IPC_TOOL_RECEIPT_REQUIRED');
    if(p.outcome==='unavailable'){if(p.operation_ref!==null||p.result_ref!==null||p.result_json!==null)throw new Error('IPC_TOOL_RECEIPT_REQUIRED');}
    else {if(!p.result_ref||p.result_json===null)throw new Error('IPC_TOOL_RECEIPT_REQUIRED');const envelope=decodeToolResult(pending.name,Buffer.from(p.result_json));if(envelope.status!==p.outcome||envelope.ok!==(p.outcome==='completed')||envelope.operation_ref!==p.operation_ref||(!['apply_notebook_patch','run_cells'].includes(pending.name)&&p.operation_ref!==null))throw new Error('IPC_TOOL_RESULT_MISMATCH');}
    this.#tools.delete(frame.payload.tool_call_id);pending.resolve(frame.payload);break;
   }
   case 'cancel':this.#cancelled=true;this.#rejectPending(new Error('AGENT_CANCELLED'));this.#runtime!.abort();if(!this.#active)await this.#stop('cancelled');break;
   case 'error':this.#failure=true;this.#rejectPending(new Error('NATIVE_PORT_UNAVAILABLE'));this.#runtime!.abort();if(!this.#active)await this.#stop('failed');break;
   default:throw new Error('IPC_WRONG_DIRECTION');
  }
 }
 #validateInput(message:AgentCanonicalMessage){if(message.role!=='user'||message.task_id!==this.#hello!.task_id)throw new Error('STALE_AGENT_SCOPE');}
 #createRuntime(task:string):AgentRuntime{
  const tools:AgentTool[]=toolSchema.properties.tool.enum.map((name:AgentToolName)=>({name,label:name,description:'Native '+name,parameters:argumentsSchema(name),replay:'never',execute:async(call:string)=>{
   const result=this.#results.get(call);if(!result)throw new Error('IPC_TOOL_NOT_ACKED');this.#results.delete(call);
   if(result.outcome==='unavailable')return {content:[{type:'text',text:'Native tool unavailable'}],details:{outcome:result.outcome},isError:true,terminate:true};
   if(name==='apply_notebook_patch')return notebookPatchResult(result);
   return {content:[{type:'text',text:result.result_json!}],details:{outcome:result.outcome,resultRef:result.result_ref,operationRef:result.operation_ref},isError:result.outcome!=='completed',terminate:result.outcome==='unknown'||result.outcome==='cancelled'};
  }}));
  return new AgentRuntime(task,model,tools,{
   canonical:message=>{const canonical=this.#canonical.get(message);if(!canonical)throw new Error('IPC_MODEL_MESSAGE_MISSING');return canonical;},
   persist:async(message,signal)=>{
    signal?.throwIfAborted();const event='assistant-event-'+randomUUID();
    const pending=Promise.withResolvers<void>();void pending.promise.catch(()=>{});
    this.#events.set(message.message_id,event);
    await this.#send('transcript_event',{event_id:event,source_sequence:++this.#sourceSequence,message},null,id=>this.#ack.set(id,{resolve:pending.resolve,reject:pending.reject,event}));
    await pending.promise;signal?.throwIfAborted();
   },
   prepare:async(signal)=>{
    signal?.throwIfAborted();this.#request={agent_turn_id:'turn-'+randomUUID(),request_id:'request-'+randomUUID(),context_ref:'unprepared',model_config_ref:'unprepared'};
    this.#offset=0;this.#requestTerminal=false;this.#partial=undefined;
    const pending=Promise.withResolvers<AgentModelRequestPayload>();void pending.promise.catch(()=>{});this.#prepared={resolve:pending.resolve,reject:pending.reject};
    await this.#send('prepare_request',{agent_turn_id:this.#request.agent_turn_id,request_id:this.#request.request_id});
    await pending.promise;signal?.throwIfAborted();
   },
   stream:()=>{
    const stream=createAssistantMessageEventStream();this.#stream=stream;
    void this.#send('model_request',this.#request).catch(()=>this.#failStream('error'));
    return stream;
   },
   beforeTool:async(context,signal)=>{
    const canonical=this.#canonical.get(context.assistantMessage);if(!canonical?.agent_turn_id)throw new Error('IPC_MODEL_MESSAGE_MISSING');
    const event=this.#events.get(canonical.message_id);if(!event)throw new Error('IPC_PROPOSAL_NOT_ACKED');
    const pending=Promise.withResolvers<AgentToolResultPayload>();void pending.promise.catch(()=>{});this.#tools.set(context.toolCall.id,{resolve:pending.resolve,reject:pending.reject,turn:canonical.agent_turn_id,name:context.toolCall.name});
    await this.#send('tool_call',{agent_turn_id:canonical.agent_turn_id,tool_call_id:context.toolCall.id,proposal_event_ref:event});
    this.#results.set(context.toolCall.id,await pending.promise);signal?.throwIfAborted();
   },
   afterTool:async()=>{},
  });
 }
 #modelEvent(payload:Extract<AgentIPCFrame,{type:'model_event'}>['payload']):void{
  if(!this.#request||!this.#stream||this.#requestTerminal||payload.request_id!==this.#request.request_id||payload.agent_turn_id!==this.#request.agent_turn_id||payload.byte_offset!==this.#offset)throw new Error('IPC_MODEL_STREAM_MISMATCH');
  this.#offset+=Buffer.byteLength(payload.text,'utf8');
  if(payload.phase==='delta'){
   if(!this.#partial){
    this.#partial={role:'assistant',api:model.api,provider:model.provider,model:model.id,content:[{type:'text',text:''}],usage,stopReason:'stop',timestamp:Date.now()};
    this.#stream.push({type:'start',partial:this.#partial});this.#stream.push({type:'text_start',contentIndex:0,partial:this.#partial});
   }
   const part=this.#partial.content[0];if(part.type!=='text')throw new Error('IPC_INVALID_STREAM');part.text+=payload.text;
   this.#stream.push({type:'text_delta',contentIndex:0,delta:payload.text,partial:this.#partial});return;
  }
  this.#requestTerminal=true;
  if(payload.phase!=='finished'){this.#failure=payload.phase==='failed';this.#failStream(payload.phase==='cancelled'?'aborted':'error');return;}
  if(!payload.canonical_message)throw new Error('IPC_MODEL_MESSAGE_MISSING');
  const canonical=decodeCanonical(Buffer.from(JSON.stringify(payload.canonical_message)));
  if(canonical.role!=='assistant'||canonical.task_id!==this.#hello!.task_id||canonical.agent_turn_id!==this.#request.agent_turn_id)throw new Error('STALE_CANONICAL_SCOPE');
  const content:AssistantMessage['content']=canonical.parts.map(p=>{
   if(p.kind==='text')return {type:'text',text:p.text};
   if(p.kind==='tool_call'){const args=strictJson(Buffer.from(p.arguments_json));if(!args||typeof args!=='object'||Array.isArray(args))throw new Error('IPC_INVALID_TOOL_ARGUMENTS');return {type:'toolCall',id:p.tool_call_id,name:p.name,arguments:args};}
   throw new Error('UNSUPPORTED_MODEL_PART');
  });
  const message:AssistantMessage={role:'assistant',api:model.api,provider:model.provider,model:model.id,content,usage,stopReason:content.some(c=>c.type==='toolCall')?'toolUse':'stop',timestamp:Date.now()};
  this.#canonical.set(message,canonical);
  this.#stream.push({type:'done',reason:message.stopReason as 'stop'|'toolUse',message});
 }
 #failStream(reason:'error'|'aborted'){
  if(!this.#stream)return;
  const canonical:AgentCanonicalMessage={record_type:'canonical_agent_message',message_version:1,message_id:'assistant-error-'+randomUUID(),role:'assistant',agent_turn_id:this.#request?.agent_turn_id??null,task_id:this.#hello!.task_id,parts:[{kind:'text',part_id:'error-text',text:reason==='aborted'?'Agent cancelled':'Native model transport unavailable'}]};
  const message:AssistantMessage={role:'assistant',api:model.api,provider:model.provider,model:model.id,content:[{type:'text',text:canonical.parts[0].kind==='text'?canonical.parts[0].text:''}],usage,stopReason:reason,errorMessage:'NATIVE_PORT_UNAVAILABLE',timestamp:Date.now()};
  this.#canonical.set(message,canonical);this.#stream.push({type:'error',reason,error:message});
 }
 #rejectPending(error:Error){for(const p of this.#ack.values())p.reject(error);this.#ack.clear();this.#prepared?.reject(error);this.#prepared=undefined;for(const p of this.#tools.values())p.reject(error);this.#tools.clear();this.#failStream('aborted');}
 async #stop(reason:'loop_stopped'|'failed'|'cancelled'){
  if(this.#closed)return;await this.#send('settled',{reason});this.#closed=true;this.#settled();
 }
 async close(){if(this.#closed)return;this.#closed=true;this.#rejectPending(new Error('IPC_CONNECTION_CLOSED'));this.#runtime?.abort();await this.#writes.catch(()=>{});}
}
