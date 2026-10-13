// Actual Rust codec -> pinned Pi. Synthetic provider bytes/native tool results; no model network.
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {writeFileSync, mkdirSync} from 'node:fs';
import {join} from 'node:path';
import {Type} from 'typebox';
import {createAssistantMessageEventStream} from '@earendil-works/pi-ai';
import {AgentRuntime} from '../src/runtime.ts';

const fixture=process.env.OPENMATH_SINGLE_TURN_FIXTURE;
assert.ok(fixture,'Run scripts/test_single_turn_pi.py to build the real Rust fixture first');
const user={record_type:'canonical_agent_message',message_version:1,message_id:'user',role:'user',agent_turn_id:'input-turn',task_id:'task',parts:[{kind:'text',part_id:'text',text:'读取当前笔记本🙂'}]};
const model={id:'synthetic',name:'synthetic',api:'openai-completions',provider:'openmath-native',baseUrl:'',reasoning:false,input:['text'],cost:{input:0,output:0,cacheRead:0,cacheWrite:0},contextWindow:4096,maxTokens:256};
const argumentsJSON='{ "view" : "outline" }';
const tools=[{name:'read_notebook',description:'Native fixture unavailable',parameters:{type:'object',properties:{view:{type:'string'}},additionalProperties:false}}];
function chat(round,incomplete){
 const chunk=(delta,reason=null)=>`data: ${JSON.stringify({id:`response-${round}`,model:'actual-provider-alias',choices:[{index:0,delta,finish_reason:reason}]})}\n\n`;
 return (round===0?chunk({role:'assistant',content:'中文🙂'})+chunk({tool_calls:[{index:0,id:'original-call',type:'function',function:{name:'read_notebook',arguments:incomplete?'{ "view" : ':argumentsJSON}}]},incomplete?'length':'tool_calls'):chunk({role:'assistant',content:'工具不可用，未读取。'},'stop'))+'data: [DONE]\n\n';
}
function anthropic(round,incomplete){
 const e=(type,fields={})=>`event: ${type}\ndata: ${JSON.stringify({type,...fields})}\n\n`;
 let s=e('message_start',{message:{id:`response-${round}`,model:'actual-provider-alias',role:'assistant',content:[],usage:{input_tokens:7,output_tokens:1}}});
 s+=e('content_block_start',{index:0,content_block:{type:'text',text:''}})+e('content_block_delta',{index:0,delta:{type:'text_delta',text:round===0?'中文🙂':'工具不可用，未读取。'}})+e('content_block_stop',{index:0});
 if(round===0)s+=e('content_block_start',{index:1,content_block:{type:'tool_use',id:'original-call',name:'read_notebook',input:{}}})+e('content_block_delta',{index:1,delta:{type:'input_json_delta',partial_json:incomplete?'{ "view" : ':argumentsJSON}})+e('content_block_stop',{index:1});
 return s+e('message_delta',{delta:{stop_reason:incomplete?'max_tokens':round===0?'tool_use':'end_turn'},usage:{output_tokens:9}})+e('message_stop');
}
for(const protocol of ['chat_completions','anthropic_messages']) {
 for(const incomplete of [false,true]) {
  test(`${protocol} real single-turn codec drives Pi; incomplete=${incomplete}`,async()=>{
   const trace=[],proof=[],canonical=new WeakMap(),messages=[{canonical:user,replay:null}],results=[];
   let round=0,executions=0;
   const binding={provider_ref:'provider-1',route_ref:'route-1',model_ref:'model-1',adapter_version:'single-turn-1',protocol};
   const port={
    prepare:async()=>{trace.push('prepare');},
    stream:()=>{
     const n=round++;
     const input={request:{binding,api_model:'configured-alias',messages,results,tools,max_output_tokens:128,temperature:null,stream:true},message_id:`canonical-${n}`,task_id:'task',turn_id:`turn-${n}`,status:200,response:(protocol==='chat_completions'?chat:anthropic)(n,incomplete&&n===0)};
     const executed=spawnSync(fixture,[],{input:JSON.stringify(input),encoding:'utf8',maxBuffer:4*1024*1024,timeout:10000});
     assert.equal(executed.status,0,executed.stderr);
     const actual=JSON.parse(executed.stdout),r=actual.result;proof.push({input:structuredClone(input),actual});trace.push(`codec:${r.finish}`);
     assert.equal(r.replay?.response_id??null,incomplete?null:`response-${n}`);
     if(n===1) {
      if(protocol==='chat_completions')assert.deepEqual(actual.body.messages.at(-1),{role:'tool',tool_call_id:'original-call',content:'Native fixture unavailable; no tool effect.'});
      else assert.equal(actual.body.messages.at(-1).content[0].tool_use_id,'original-call');
     }
     // Pi requires numeric carrier usage. Unknown native usage stays null in proof, never billing zero.
     const message={role:'assistant',api:model.api,provider:model.provider,model:model.id,content:r.message.parts.map(p=>p.kind==='tool_call'?{type:'toolCall',id:p.tool_call_id,name:p.name,arguments:JSON.parse(p.arguments_json)}:{type:'text',text:p.text}),stopReason:r.finish==='tool_calls'?'toolUse':r.finish==='complete'?'stop':'error',timestamp:n+1,usage:{input:r.usage.input??0,output:r.usage.output??0,cacheRead:r.usage.cache_read??0,cacheWrite:r.usage.cache_write??0,totalTokens:r.usage.total??0,cost:{input:0,output:0,cacheRead:0,cacheWrite:0,total:0}}};
     canonical.set(message,r);
     const stream=createAssistantMessageEventStream();
     if(r.finish==='incomplete')stream.push({type:'error',reason:'error',error:message});
     else stream.push({type:'done',reason:message.stopReason,message});
     return stream;
    },
    canonical:message=>canonical.get(message).message,
    persist:async message=>{
     trace.push(`ack:${message.message_id}`);
     const decoded=proof.find(p=>p.actual.result.message.message_id===message.message_id).actual.result;
     messages.push({canonical:message,replay:decoded.replay});
     if(message.parts.some(p=>p.kind==='tool_call'))assert.equal(message.parts.find(p=>p.kind==='tool_call').arguments_json,argumentsJSON);
    },
    beforeTool:async()=>trace.push('beforeTool'),
    afterTool:async context=>{
     trace.push('afterTool');
     const id=context.toolCall.id;
     const result={...user,message_id:'tool-result',role:'tool',agent_turn_id:'turn-0',parts:[{kind:'tool_result',part_id:'result',tool_call_id:id,outcome:'unavailable',operation_ref:null,result_ref:null}]};
     messages.push({canonical:result,replay:null});results.push({message_id:result.message_id,tool_call_id:id,text:'Native fixture unavailable; no tool effect.',is_error:true});
    },
   };
   const runtime=new AgentRuntime('task',model,[{name:'read_notebook',label:'read',description:'native fixture',parameters:Type.Object({view:Type.String()}),execute:async()=>{trace.push('nativeTool');executions++;return {content:[{type:'text',text:'Native fixture unavailable; no tool effect.'}],details:{fixture:true},isError:true};}}],port);
   await runtime.prompt(user);
   assert.equal(executions,incomplete?0:1);assert.equal(round,incomplete?1:2);assert.equal(runtime.failed,incomplete);
   assert.deepEqual(trace,incomplete?['prepare','codec:incomplete']:['prepare','codec:tool_calls','ack:canonical-0','beforeTool','nativeTool','afterTool','prepare','codec:complete','ack:canonical-1']);
   if(process.env.OPENMATH_SINGLE_TURN_EVIDENCE){mkdirSync(process.env.OPENMATH_SINGLE_TURN_EVIDENCE,{recursive:true});writeFileSync(join(process.env.OPENMATH_SINGLE_TURN_EVIDENCE,`${protocol}-${incomplete?'incomplete':'complete'}.json`),JSON.stringify({scope:'development synthetic provider bytes/native unavailable tool; actual Rust codec and pinned Pi',trace,proof},null,2)+'\n');}
  });
 }
}
