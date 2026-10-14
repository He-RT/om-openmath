import { test } from 'node:test';
import assert from 'node:assert/strict';
import { FrameDecoder, encodeFrame, decodeFrame, decodeReadonlyResult, strictJson, schemaHash, MAX_FRAME_BYTES } from '../src/protocol.ts';
const sample = {
  protocol_version: 1, connection_id: 'connection-1', connection_generation: 1,
  runtime_instance_id: 'runtime-1', agent_session_id: 'session-1', document_id: 'document-1',
  document_generation: 1, task_id: 'task-1', task_generation: 1, message_id: 'message-1', sequence: 1, reply_to: null,
  type: 'model_event', payload: { agent_turn_id: 'turn-1', request_id: 'request-1', byte_offset: 0, text: '中文🙂é', phase: 'delta', canonical_message: null },
};
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
