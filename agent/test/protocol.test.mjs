import { test } from 'node:test';
import assert from 'node:assert/strict';
import { FrameDecoder, encodeFrame, decodeFrame, decodeReadonlyResult, decodeToolResult, strictJson, schemaHash, MAX_FRAME_BYTES } from '../src/protocol.ts';
import { notebookPatchResult } from '../src/tools/notebook.ts';
const sample = {
  protocol_version: 1, connection_id: 'connection-1', connection_generation: 1,
  runtime_instance_id: 'runtime-1', agent_session_id: 'session-1', document_id: 'document-1',
  document_generation: 1, task_id: 'task-1', task_generation: 1, message_id: 'message-1', sequence: 1, reply_to: null,
  type: 'model_event', payload: { agent_turn_id: 'turn-1', request_id: 'request-1', byte_offset: 0, text: '中文🙂é', phase: 'delta', canonical_message: null },
};
test('inspect_result keeps original history/partial data closed and separate from render claims', () => {
  const result={ok:true,status:'completed',request_ref:'native-inspect',operation_ref:null,document:{id:'doc',generation:1,revision:2},data:{origin:'notebook',is_current:false,part:'summary',payload:{status:'Error',binding:{freshness:'stale'},successful_statements:1},render_state:'not_applicable',media_ref:null},error:null,pagination:{truncated:false,next_cursor:null}};
  const encode=value=>Buffer.from(JSON.stringify(value));
  assert.equal(decodeReadonlyResult('inspect_result',encode(result)).ok,true);
  for(const payload of [{},[],"原文🙂",1,true,null])assert.equal(decodeReadonlyResult('inspect_result',encode({...result,data:{...result.data,payload}})).ok,true);
  for(const bad of [{...result,data:{...result.data,source:'execute'}},{...result,operation_ref:'invented-operation'},{...result,data:{...result.data,origin:'main_kernel'}},{...result,status:'partial'}])assert.throws(()=>decodeToolResult('inspect_result',encode(bad)),/IPC_INVALID_TOOL_RESULT/);
  const failed={...result,ok:false,status:'failed',data:null,error:{code:'INVALID_REFERENCE',message:'expired',recovery:'retry_read',details:{}}};
  assert.equal(decodeReadonlyResult('inspect_result',encode(failed)).status,'failed');
});
test('scratch data retains isolated result and real errors without a notebook modification claim', () => {
  const data={result_ref:'isolated-result',evaluation_status:'done',output_summary:{origin:'scratch',acceptance:'isolated'},diagnostics:[],notebook_modified:false};
  const result={ok:true,status:'completed',request_ref:'scratch-request',operation_ref:null,document:{id:'doc',generation:1,revision:0},data,error:null,pagination:{truncated:false,next_cursor:null}};
  const encode=value=>Buffer.from(JSON.stringify(value));
  assert.equal(decodeReadonlyResult('evaluate_scratch',encode(result)).status,'completed');
  for(const bad of [{...result,data:{...data,notebook_modified:true}},{...result,data:{...data,result_ref:null}},{...result,operation_ref:'fabricated-main-operation'}])assert.throws(()=>decodeReadonlyResult('evaluate_scratch',encode(bad)),/IPC_INVALID_TOOL_RESULT/);
  const partial={...result,ok:false,status:'failed',data:{...data,evaluation_status:'error'},error:{code:'EXECUTION_FAILED',message:'Real scratch error after successful prefix',recovery:'none',details:{}}};
  assert.equal(decodeReadonlyResult('evaluate_scratch',encode(partial)).status,'failed');
});
test('actual preview results preserve source-only and complete patch reference rules', () => {
  const encode=value=>Buffer.from(JSON.stringify(value));
  const data={preview_kind:'source',valid:true,preview_ref:null,plan_hash:'a'.repeat(64),diagnostics:[],affected_cell_ids:[],assigned_cell_ids:{}};
  const result={ok:true,status:'completed',request_ref:'request',operation_ref:null,document:{id:'doc',generation:1,revision:0},data,error:null,pagination:{truncated:false,next_cursor:null}};
  assert.equal(decodeReadonlyResult('preview_source',encode(result)).status,'completed');
  const patch=structuredClone(result);patch.data.preview_kind='patch';
  assert.throws(()=>decodeReadonlyResult('preview_source',encode(patch)),/IPC_INVALID_TOOL_RESULT/);
  patch.data.preview_ref='actual-preview';
  assert.equal(decodeReadonlyResult('preview_source',encode(patch)).status,'completed');
  const fake=structuredClone(result);fake.data.preview_ref='fake-source-preview';
  assert.throws(()=>decodeReadonlyResult('preview_source',encode(fake)),/IPC_INVALID_TOOL_RESULT/);
  assert.throws(()=>decodeReadonlyResult('apply_notebook_patch',encode(result)),/IPC_TOOL_RESULT_LIMIT/);
});
test('patch result preserves actual operation/saved=false and uncertain writes cannot continue a loop', () => {
  const result={ok:true,status:'completed',request_ref:'request',operation_ref:'original-operation',document:{id:'doc',generation:1,revision:1},data:{transaction_ref:'original-transaction',snapshot_ref:'new-snapshot',changed_cell_ids:['a'],invalidated_cell_ids:['a'],plan_hash:'a'.repeat(64),saved:false},error:null,pagination:{truncated:false,next_cursor:null}};
  const payload=body=>({agent_turn_id:'turn',tool_call_id:'call',operation_ref:body.operation_ref,result_ref:'native-result',outcome:body.status,result_json:JSON.stringify(body)});
  assert.equal(notebookPatchResult(payload(result)).details.operationRef,'original-operation');
  for(const bad of [{...result,data:{...result.data,saved:true}},{...result,data:{...result.data,extra:true}}])assert.throws(()=>decodeToolResult('apply_notebook_patch',Buffer.from(JSON.stringify(bad))),/IPC_INVALID_TOOL_RESULT/);
  const unknown={...result,ok:false,status:'unknown',data:null,error:{code:'UNKNOWN_OUTCOME',message:'unknown',recovery:'query_operation',details:{}}};
  assert.equal(notebookPatchResult(payload(unknown)).terminate,true);
  assert.equal(notebookPatchResult(payload(unknown)).isError,true);
  assert.throws(()=>notebookPatchResult({...payload(unknown),operation_ref:'new-operation'}),/IPC_TOOL_RESULT_MISMATCH/);
  assert.throws(()=>decodeToolResult('apply_notebook_patch',Buffer.from(JSON.stringify({...unknown,operation_ref:null}))),/IPC_INVALID_TOOL_RESULT/);
});
test('all actual encoded UTF8 split points and multiple complete frames roundtrip', () => {
  const line = encodeFrame(sample);
  for (let cut = 0; cut <= line.length; cut++) {
    const decoder = new FrameDecoder();
    const frames = [...decoder.push(line.subarray(0, cut)), ...decoder.push(line.subarray(cut))];
    assert.equal(frames.length, 1); assert.deepEqual(JSON.parse(JSON.stringify(frames[0])), sample); decoder.finish();
  }
  assert.equal(schemaHash.length, 64);
  const two = new Uint8Array(line.length * 2); two.set(line); two.set(line, line.length);
  assert.equal(new FrameDecoder().push(two).length, 2);
});
test('unknown credentials/URLs/Pi state, missing required null and unsafe counters are rejected', () => {
  for (const name of ['api_key', 'authorization', 'base_url', 'pi_state']) {
    const value = structuredClone(sample); value.payload[name] = 'private-value';
    assert.throws(() => decodeFrame(new TextEncoder().encode(JSON.stringify(value))), /IPC_INVALID_SHAPE/);
  }
  const missing = structuredClone(sample); delete missing.reply_to;
  assert.throws(() => decodeFrame(new TextEncoder().encode(JSON.stringify(missing))), /IPC_INVALID_SHAPE/);
  const bad = structuredClone(sample); bad.sequence = 9007199254740992;
  assert.throws(() => decodeFrame(new TextEncoder().encode(JSON.stringify(bad))), /IPC_INVALID_SHAPE/);
});
test('duplicate members, invalid Unicode/UTF8, limits and incomplete EOF poison real connections', () => {
  for (const text of ['{"a":1,"a":2}', '{"a":{"x":1,"\\u0078":2}}', '"\\ud800"', '1e9999', '{} junk', '['.repeat(65) + '0' + ']'.repeat(65)]) {
    assert.throws(() => strictJson(new TextEncoder().encode(text)), /IPC_INVALID_JSON/);
  }
  assert.throws(() => strictJson(new Uint8Array([255])), /IPC_INVALID_UTF8/);
  assert.throws(() => strictJson(new Uint8Array(MAX_FRAME_BYTES + 1)), /IPC_FRAME_SIZE/);
  const decoder = new FrameDecoder(); decoder.push(new TextEncoder().encode('{"a"'));
  assert.throws(() => decoder.finish(), /IPC_INCOMPLETE_EOF/);
  assert.throws(() => decoder.push(new TextEncoder().encode('}\n')), /IPC_CONNECTION_FAILED/);
});

test('run result keeps the original operation and accepts partial facts without becoming readonly',()=>{
 const encode=v=>Buffer.from(JSON.stringify(v));
 const result={ok:true,status:'completed',request_ref:'run',operation_ref:'run',document:{id:'doc',generation:1,revision:0},data:{run_ref:'run',run_outcome:'completed',cells:[{cell_id:'a',source_hash:'a'.repeat(64),status:'done',result_ref:'actual-result',diagnostics:[]}],executed_cell_ids:['a']},error:null,pagination:{truncated:false,next_cursor:null}};
 assert.equal(decodeToolResult('run_cells',encode(result)).operation_ref,'run');
 assert.throws(()=>decodeReadonlyResult('run_cells',encode(result)),/IPC_TOOL_RESULT_LIMIT/);
 const partial={...result,ok:false,status:'partial',data:{...result.data,run_outcome:'partial'},error:{code:'EXECUTION_FAILED',message:'Original accepted prefix retained',recovery:'none',details:{}}};
 assert.equal(decodeToolResult('run_cells',encode(partial)).status,'partial');
 const unknown={...partial,status:'unknown',error:{code:'UNKNOWN_OUTCOME',message:'Read original operation',recovery:'query_operation',details:{}}};
 assert.equal(decodeToolResult('run_cells',encode(unknown)).status,'unknown');
 for(const bad of [{...result,operation_ref:'different-run'},{...result,data:{...result.data,source:'execute'}},{...partial,ok:true},{...unknown,operation_ref:null}])assert.throws(()=>decodeToolResult('run_cells',encode(bad)),/IPC_INVALID_TOOL_RESULT/);
});

test('operation query may complete while its target remains unknown; it grants no effect',()=>{
 const result={ok:true,status:'completed',request_ref:'query',operation_ref:null,document:{id:'doc',generation:2,revision:7},data:{operation_state:'unknown',outcome_known:false,effect_committed:null,transaction_ref:null,run_ref:null,result_refs:[],recovery_hint:'Original receipt is unavailable; never replay'},error:null,pagination:{truncated:false,next_cursor:null}};
 assert.equal(decodeReadonlyResult('get_operation_status',Buffer.from(JSON.stringify(result))).status,'completed');
 for(const value of [{...result,operation_ref:'fake-side-effect'},{...result,data:{...result.data,execute:true}},{...result,data:{...result.data,operation_state:'not_found'}}])assert.throws(()=>decodeToolResult('get_operation_status',Buffer.from(JSON.stringify(value))),/IPC_INVALID_TOOL_RESULT/);
});

test('undo result carries the original inverse operation and saved=false without readonly authority',()=>{
 const value={ok:true,status:'completed',request_ref:'original-undo-call',operation_ref:'original-undo-operation',document:{id:'doc',generation:1,revision:3},data:{undo_transaction_ref:'new-inverse-transaction',snapshot_ref:'actual-current-source',changed_cell_ids:['a'],invalidated_cell_ids:['a'],saved:false},error:null,pagination:{truncated:false,next_cursor:null}};
 const encode=x=>Buffer.from(JSON.stringify(x));
 assert.equal(decodeToolResult('undo_transaction',encode(value)).operation_ref,'original-undo-operation');
 assert.throws(()=>decodeReadonlyResult('undo_transaction',encode(value)),/IPC_TOOL_RESULT_LIMIT/);
 const unknown={...value,ok:false,status:'unknown',data:null,error:{code:'UNKNOWN_OUTCOME',message:'Query original undo operation',recovery:'query_operation',details:{}}};
 assert.equal(decodeToolResult('undo_transaction',encode(unknown)).status,'unknown');
 for(const changed of [{...value,data:{...value.data,saved:true}},{...value,data:{...value.data,source:'run code'}},{...unknown,operation_ref:null}])assert.throws(()=>decodeToolResult('undo_transaction',encode(changed)),/IPC_INVALID_TOOL_RESULT/);
});
