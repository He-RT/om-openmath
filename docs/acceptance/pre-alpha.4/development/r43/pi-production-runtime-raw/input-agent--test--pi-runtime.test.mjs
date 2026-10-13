// Real pinned Pi loop with explicit synthetic native-port outcomes. No model network or CAS effects.
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {Type} from 'typebox';
import {createAssistantMessageEventStream} from '@earendil-works/pi-ai';
import {AgentRuntime} from '../src/runtime.ts';
const model={id:'synthetic',name:'synthetic',api:'openai-completions',provider:'openmath-fixture',baseUrl:'',reasoning:false,input:['text'],cost:{input:0,output:0,cacheRead:0,cacheWrite:0},contextWindow:4096,maxTokens:256};
const user={record_type:'canonical_agent_message',message_version:1,message_id:'user',role:'user',agent_turn_id:'turn',task_id:'task',parts:[{kind:'text',part_id:'text',text:'合成框架顺序🙂'}]};
function output(content,stopReason){return {role:'assistant',api:model.api,provider:model.provider,model:model.id,content,stopReason,timestamp:1,usage:{input:0,output:0,cacheRead:0,cacheWrite:0,totalTokens:0,cost:{input:0,output:0,cacheRead:0,cacheWrite:0,total:0}}};}
function canonical(message){return {...user,role:'assistant',message_id:`assistant-${message.timestamp}`,parts:message.content.map((p,i)=>p.type==='toolCall'?{kind:'tool_call',part_id:`part-${i}`,tool_call_id:p.id,name:p.name,arguments_json:JSON.stringify(p.arguments)}:{kind:'text',part_id:`part-${i}`,text:p.text})};}
test('pinned Pi hooks await canonical ACK and tools execute sequentially before next model request',async()=>{
 const trace=[];let round=0,active=0;let release,arrived;
 const ack=new Promise(r=>release=r),waiting=new Promise(r=>arrived=r);
 const tools=['read_notebook','search_functions'].map(name=>({name,label:name,description:'synthetic port',parameters:Type.Object({}),execute:async(id)=>{active++;assert.equal(active,1);trace.push(`execute:${id}`);await new Promise(r=>setTimeout(r,5));active--;return {content:[{type:'text',text:'synthetic unavailable'}],details:{fixture:true},isError:true};}}));
 const port={canonical,prepare:async()=>trace.push('prepare'),stream:()=>{trace.push('stream');const stream=createAssistantMessageEventStream();const message=round++===0?output([{type:'toolCall',id:'call-1',name:'read_notebook',arguments:{}},{type:'toolCall',id:'call-2',name:'search_functions',arguments:{}}],'toolUse'):output([{type:'text',text:'循环停止，仅合成结果'}],'stop');message.timestamp=round;stream.push({type:'done',reason:message.stopReason,message});return stream;},persist:async message=>{trace.push(`persist:${message.message_id}`);if(message.message_id==='assistant-1'){arrived();await ack;}trace.push(`ack:${message.message_id}`);},beforeTool:async c=>trace.push(`before:${c.toolCall.id}`),afterTool:async c=>trace.push(`after:${c.toolCall.id}`)};
 const runtime=new AgentRuntime('task',model,tools,port),running=runtime.prompt(user);
 await waiting;assert.equal(trace.filter(v=>v.startsWith('execute')).length,0);release();await running;
 assert.deepEqual(trace,['prepare','stream','persist:assistant-1','ack:assistant-1','before:call-1','execute:call-1','after:call-1','before:call-2','execute:call-2','after:call-2','prepare','stream','persist:assistant-2','ack:assistant-2']);
});
test('cancel while canonical ACK is pending prevents every tool effect',async()=>{
 let arrive,release,executions=0;
 const waiting=new Promise(r=>arrive=r),ack=new Promise(r=>release=r);
 const streamFn=()=>{const stream=createAssistantMessageEventStream();const message=output([{type:'toolCall',id:'call-1',name:'read_notebook',arguments:{}}],'toolUse');stream.push({type:'done',reason:'toolUse',message});return stream;};
 const runtime=new AgentRuntime('task',model,[{name:'read_notebook',label:'read',description:'synthetic',parameters:Type.Object({}),execute:async()=>{executions++;return {content:[],details:{}};}}],{stream:streamFn,canonical,prepare:async()=>{},persist:async()=>{arrive();await ack;},beforeTool:async()=>{},afterTool:async()=>{}});
 const running=runtime.prompt(user);await waiting;runtime.abort();release();await running;assert.equal(executions,0);
});
test('shell plugins and mismatched canonical user tasks are refused',async()=>{
 const port={stream:()=>{},canonical,prepare:async()=>{},persist:async()=>{},beforeTool:async()=>{},afterTool:async()=>{}};
 assert.throws(()=>new AgentRuntime('task',model,[{name:'shell'}],port),/INVALID_AGENT_ADAPTER/);
 const runtime=new AgentRuntime('task',model,[],port);assert.throws(()=>runtime.prompt({...user,task_id:'old-task'}),/UNSUPPORTED_AGENT_INPUT/);
});
