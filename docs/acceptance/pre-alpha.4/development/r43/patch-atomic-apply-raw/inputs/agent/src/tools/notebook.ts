/** Pi receives only the native owner's durable result. It never assembles source operations. */
import type {AgentToolResultPayload} from '../generated/protocol.ts';
import {decodeToolResult} from '../protocol.ts';

export function notebookPatchResult(result:AgentToolResultPayload) {
  if(result.result_json===null||result.result_ref===null)throw new Error('IPC_TOOL_RECEIPT_REQUIRED');
  const envelope=decodeToolResult('apply_notebook_patch',Buffer.from(result.result_json));
  if(envelope.status!==result.outcome||envelope.operation_ref!==result.operation_ref)throw new Error('IPC_TOOL_RESULT_MISMATCH');
  // Unknown/cancelled effects settle this loop. A fresh turn may query the original operation;
  // this adapter never replays a write because no successful result was observed.
  return {content:[{type:'text' as const,text:result.result_json}],details:{outcome:result.outcome,resultRef:result.result_ref,operationRef:result.operation_ref},isError:result.outcome!=='completed',terminate:result.outcome==='unknown'||result.outcome==='cancelled'};
}
