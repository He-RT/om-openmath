// Actual private test peer; this is not the pinned production Pi adapter.
import fs from 'node:fs';
import {FrameDecoder,encodeFrame} from '../../../agent/src/protocol.ts';
const decoder=new FrameDecoder(),observed=[],mode=process.argv[3]??'valid';
let hello,original,ack,stage=0,toolFinished=false,modelFinished=false,settled=false;
function finish(){
 if(settled||!toolFinished||!modelFinished)return;settled=true;
 emit('settled',6,{reason:'loop_stopped'});
 fs.writeFileSync(process.argv[2],JSON.stringify({fixture:true,framework:'synthetic-private-peer',environment_keys:Object.keys(process.env),observed},null,2)+'\n');
 process.stdout.write('',()=>process.exit(0));
}
function emit(type,sequence,payload,reply_to=null){const f={...hello,type,sequence,message_id:`peer-${sequence}`,reply_to,payload};const bytes=encodeFrame(f);observed.push({direction:'peer',frame:f});process.stdout.write(bytes);return bytes;}
for await(const chunk of process.stdin){for(const frame of decoder.push(chunk)){
 observed.push({direction:'host',frame});
 if(mode==='collect'){if(observed.length===2){fs.writeFileSync(process.argv[2],JSON.stringify(observed)+'\n');process.exit(0);}continue;}
 if(frame.type==='hello'){
  hello=frame;
  if(mode!=='valid'){
   let raw;
   if(mode==='invalid_utf8')raw=Buffer.from([0xff,10]);
   if(mode==='partial_eof')raw=Buffer.from('{"type":');
   if(mode==='sequence_gap')raw=Buffer.from(JSON.stringify({...frame,type:'ready',sequence:2,reply_to:frame.message_id})+'\n');
   if(mode==='stale_scope')raw=Buffer.from(JSON.stringify({...frame,type:'ready',document_generation:frame.document_generation+1,reply_to:frame.message_id})+'\n');
   if(mode==='wrong_handshake')raw=Buffer.from(JSON.stringify({...frame,type:'ready',reply_to:frame.message_id,payload:{...frame.payload,schema_hash:'0'.repeat(64)}})+'\n');
   if(mode==='duplicate_field')raw=Buffer.from(JSON.stringify({...frame,type:'ready',reply_to:frame.message_id}).replace('{','{"protocol_version":1,')+'\n');
   if(mode==='oversize')raw=Buffer.alloc(1048577,32);
   if(mode==='empty_eof')raw=Buffer.alloc(0);
   fs.writeFileSync(process.argv[2],JSON.stringify({fixture:true,mode,input_base64:raw.toString('base64')})+'\n');
   process.stdout.write(raw,()=>process.exit(0));continue;
  }
  emit('ready',1,frame.payload,frame.message_id);
  original=emit('transcript_event',2,{event_id:'pipe-proposal',source_sequence:1,message:{record_type:'canonical_agent_message',message_version:1,message_id:'pipe-message',role:'assistant',agent_turn_id:'pipe-turn',task_id:hello.task_id,parts:[{kind:'text',part_id:'text',text:'完整原文 中文🙂 / é'},{kind:'tool_call',part_id:'call',tool_call_id:'model-call',name:'read_notebook',arguments_json:'{}'}]}});
 }else if(frame.type==='ack'&&stage===0){
  ack=frame;stage=1;process.stdout.write(original);
 }else if(frame.type==='ack'&&stage===1){
  if(JSON.stringify(ack)!==JSON.stringify(frame))throw Error('ACK replay changed');stage=2;
  emit('tool_call',3,{agent_turn_id:'pipe-turn',tool_call_id:'model-call',proposal_event_ref:'pipe-proposal'});
  emit('prepare_request',4,{agent_turn_id:'pipe-turn',request_id:'model-request'});
 }else if(frame.type==='model_request'){
  emit('model_request',5,frame.payload);
 }else if(frame.type==='tool_result'){
  toolFinished=true;finish();
 }else if(frame.type==='model_event'){
  modelFinished=frame.payload.phase!=='delta';finish();
 }
}}
