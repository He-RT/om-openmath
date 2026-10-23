import fs from 'node:fs/promises';
import vm from 'node:vm';
import assert from 'node:assert/strict';
const source=await fs.readFile(new URL('../OpenMath/Resources/mobile-agent.js',import.meta.url),'utf8');
const events=[];let sequence=0,rounds=0,settled;
const finished=new Promise(resolve=>settled=resolve);
const sandbox={nativeURL:(source,base)=>{const url=new URL(source,base||undefined);return JSON.stringify({href:url.href,hash:url.hash,origin:url.origin,protocol:url.protocol,pathname:url.pathname});},nativeUUID:()=>`fixture-${++sequence}`,nativePost:json=>{
  const {id,op,payload}=JSON.parse(json);events.push({op,payload});
  let result={ok:true};
  if(op==='model') {
    rounds++;const parts=rounds===1?[{kind:'tool_call',part_id:'call-part',tool_call_id:'call-1',name:'read_notebook',arguments_json:'{}'}]:[{kind:'text',part_id:'answer-part',text:'已读取中文笔记🙂'}];
    result={message:{record_type:'canonical_agent_message',message_version:1,message_id:payload.message,role:'assistant',task_id:'task-1',agent_turn_id:payload.turn,parts},replay:null,usage:{input:null,output:null},finish:rounds===1?'tool_calls':'complete'};
  }
  if(op==='tool')result={ok:true,status:'completed',operation_ref:'operation-1',request_ref:'result-1',data:{title:'中文笔记'}};
  queueMicrotask(()=>{sandbox.mobileResolve(id,JSON.stringify(result),null);if(op==='settled')settled(payload);});
}};
vm.createContext(sandbox);vm.runInContext(source,sandbox);
sandbox.mobileStart(JSON.stringify({id:'task-1',text:'读取笔记本',parts:[{kind:'text',part_id:'input-part',text:'读取笔记本'}],history:[],results:[],images:[],binding:{},model:'synthetic-no-network',contextWindow:0,maxTokens:1024,tools:[{name:'read_notebook',description:'Readonly notebook',parameters:{type:'object',properties:{},additionalProperties:false}}]}));
const timeout=setTimeout(()=>{throw new Error('Pi fixture did not settle');},5000);
const outcome=await finished;clearTimeout(timeout);
assert.equal(outcome.reason,'ended');assert.equal(rounds,2);
assert.equal(events.filter(event=>event.op==='tool').length,1,JSON.stringify(events));
const proposal=events.findIndex(event=>event.op==='record'&&event.payload.record.canonical.role==='assistant');
const tool=events.findIndex(event=>event.op==='tool');assert(proposal<tool);
const second=events.filter(event=>event.op==='model')[1].payload.request;
assert.equal(second.results[0].tool_call_id,'call-1');
assert.equal(second.messages.at(-1).canonical.role,'tool');
assert.equal(events.filter(event=>event.op==='record').at(-1).payload.record.canonical.parts[0].text,'已读取中文笔记🙂');
console.log('Pinned mobile Pi: two real scheduler rounds, durable tool proposal before invocation, paired result, Unicode reply; no network');
