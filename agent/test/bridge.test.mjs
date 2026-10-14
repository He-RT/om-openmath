import {test} from 'node:test';
import assert from 'node:assert/strict';
import {AgentBridge,ADAPTER_VERSION} from '../src/bridge.ts';
import {decodeFrame,encodeFrame,decodeReadonlyResult,schemaHash} from '../src/protocol.ts';
const hello={protocol_version:1,connection_id:'connection',connection_generation:1,runtime_instance_id:'runtime',agent_session_id:'session',document_id:'document',document_generation:1,task_id:'task',task_generation:1,message_id:'hello',sequence:1,reply_to:null,type:'hello',payload:{core_version:'1.0.4',adapter_version:ADAPTER_VERSION,schema_hash:schemaHash}};
const user={record_type:'canonical_agent_message',message_version:1,message_id:'user',role:'user',agent_turn_id:'user-turn',task_id:'task',parts:[{kind:'text',part_id:'text',text:'真实Pi/合成宿主 中文🙂'}]};
function hostFrame(kind,sequence,payload,reply_to=null){return {...hello,type:kind,message_id:'host-'+sequence,sequence,reply_to,payload};}
function raw(f){return encodeFrame(f).slice(0,-1);}
test('actual Pi bridge resolves immutable model refs and canonical ACK before unavailable tool hook',async()=>{
 const observed=[];let hostSequence=1;let bridge;const done=Promise.withResolvers();
 const deliver=f=>bridge.accept(f,raw(f));
 bridge=new AgentBridge(async bytes=>{
  const frame=decodeFrame(bytes.slice(0,-1));observed.push(frame);
  const reply=(kind,payload,reply_to=null)=>deliver(hostFrame(kind,++hostSequence,payload,reply_to));
  if(frame.type==='ready')await reply('user_input',{event_ref:'input-event',message:user});
  if(frame.type==='prepare_request')await reply('model_request',{...frame.payload,context_ref:'native-context',model_config_ref:'native-model-config'});
  if(frame.type==='model_request'){
   await reply('model_event',{agent_turn_id:frame.payload.agent_turn_id,request_id:frame.payload.request_id,byte_offset:0,text:'中文🙂',phase:'delta',canonical_message:null});
   await reply('model_event',{agent_turn_id:frame.payload.agent_turn_id,request_id:frame.payload.request_id,byte_offset:Buffer.byteLength('中文🙂'),text:'',phase:'finished',canonical_message:{...user,message_id:'assistant-native',role:'assistant',agent_turn_id:frame.payload.agent_turn_id,parts:[{kind:'text',part_id:'text',text:'中文🙂'},{kind:'tool_call',part_id:'call',tool_call_id:'model-call',name:'read_notebook',arguments_json:'{ "view" : "outline" }'}]}});
  }
  if(frame.type==='transcript_event')await reply('ack',{event_id:frame.payload.event_id,event_sequence:1,receipt_hash:'a'.repeat(64)},frame.message_id);
  if(frame.type==='tool_call')await reply('tool_result',{agent_turn_id:frame.payload.agent_turn_id,tool_call_id:frame.payload.tool_call_id,operation_ref:null,result_ref:null,outcome:'unavailable',result_json:null});
 },()=>done.resolve());
 await deliver(hello);await Promise.race([done.promise,new Promise((_,reject)=>setTimeout(()=>reject(Error('bridge timeout')),3000))]);
 assert.deepEqual(observed.map(f=>f.type),['ready','prepare_request','model_request','transcript_event','tool_call','settled']);
 const canonical=observed.find(f=>f.type==='transcript_event').payload.message;
 assert.equal(canonical.parts[1].arguments_json,'{ "view" : "outline" }');assert.equal(observed.at(-1).payload.reason,'loop_stopped');
});
test('wrong connection and byte-identical hello replay cannot reset active helper scope',async()=>{
 const frames=[];const bridge=new AgentBridge(async bytes=>frames.push(decodeFrame(bytes.slice(0,-1))),()=>{});
 await bridge.accept(hello,raw(hello));await bridge.accept(hello,raw(hello));assert.equal(frames.length,1);
 const old={...hostFrame('cancel',2,{reason:'user_stop'}),connection_generation:2};await assert.rejects(bridge.accept(old,raw(old)),/STALE_AGENT_SCOPE/);await bridge.close();
});
test('actual Pi consumes a complete native readonly result and continues to a second model turn',async()=>{
 const observed=[];let bridge,sequence=1,turn=0;const done=Promise.withResolvers();
 const deliver=f=>bridge.accept(f,raw(f));
 const nativeResult={ok:true,status:'completed',request_ref:'native-operation',operation_ref:null,document:{id:'document',generation:1,revision:7},data:{snapshot_ref:'native-snapshot',is_current:true,title:'真实数据🙂',cells:[],symbols:[],recent_transaction_refs:[]},error:null,pagination:{truncated:false,next_cursor:null}};
 bridge=new AgentBridge(async bytes=>{
  const frame=decodeFrame(bytes.slice(0,-1));observed.push(frame);
  const reply=(kind,payload,reply_to=null)=>deliver(hostFrame(kind,++sequence,payload,reply_to));
  if(frame.type==='ready')await reply('user_input',{event_ref:'input',message:user});
  if(frame.type==='prepare_request'){turn++;await reply('model_request',{...frame.payload,context_ref:'context-'+turn,model_config_ref:'config'});}
  if(frame.type==='model_request')await reply('model_event',{agent_turn_id:frame.payload.agent_turn_id,request_id:frame.payload.request_id,byte_offset:0,text:'',phase:'finished',canonical_message:{...user,message_id:'assistant-'+turn,role:'assistant',agent_turn_id:frame.payload.agent_turn_id,parts:turn===1?[{kind:'tool_call',part_id:'call',tool_call_id:'read-call',name:'read_notebook',arguments_json:'{}'}]:[{kind:'text',part_id:'text',text:'done'}]}});
  if(frame.type==='transcript_event')await reply('ack',{event_id:frame.payload.event_id,event_sequence:turn,receipt_hash:'a'.repeat(64)},frame.message_id);
  if(frame.type==='tool_call')await reply('tool_result',{agent_turn_id:frame.payload.agent_turn_id,tool_call_id:'read-call',operation_ref:null,result_ref:'native-result',outcome:'completed',result_json:JSON.stringify(nativeResult)});
 },()=>done.resolve());
 await deliver(hello);let timer;try {await Promise.race([done.promise,new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error('readonly Pi timeout')),3000)})]);}finally {clearTimeout(timer);}
 assert.equal(turn,2,JSON.stringify(observed));assert.equal(observed.filter(f=>f.type==='tool_call').length,1);assert.equal(observed.filter(f=>f.type==='transcript_event').length,2);assert.equal(observed.at(-1).payload.reason,'loop_stopped');
 assert.equal(decodeReadonlyResult('read_notebook',Buffer.from(JSON.stringify(nativeResult))).ok,true);
 for(const changed of [{...nativeResult,ok:false},{...nativeResult,error:{code:'INVALID_ARGUMENT'}},{...nativeResult,extra:true},{...nativeResult,data:{...nativeResult.data,extra:true}}])assert.throws(()=>decodeReadonlyResult('read_notebook',Buffer.from(JSON.stringify(changed))),/IPC_INVALID_TOOL_RESULT/);
});
