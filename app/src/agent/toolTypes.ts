import type {AgentToolName} from '../../../agent/src/generated/protocol';
export type ToolName=Exclude<AgentToolName,'read_attachment'|'prepare_attachment'>;
export const toolNames:ToolName[]=['read_notebook','search_functions','get_function_docs','preview_source','apply_notebook_patch','run_cells','inspect_result','evaluate_scratch','undo_transaction','get_operation_status'];
export type ToolErrorCode='INVALID_ARGUMENT'|'NOT_AVAILABLE'|'PERMISSION_DENIED'|'INVALID_REFERENCE'|'STALE_SNAPSHOT'|'EDITING_BUSY'|'INVALID_SOURCE'|'INVALID_PATCH'|'PREVIEW_MISMATCH'|'DEPENDENCY_NOT_READY'|'EXECUTION_FAILED'|'STALE_RESULT'|'CONTEXT_NOT_READY'|'LIMIT_EXCEEDED'|'TIMEOUT'|'CANCELLED'|'NOT_SUPPORTED'|'PREPARATION_REQUIRED'|'MODEL_INCOMPATIBLE'|'UNDO_CONFLICT'|'UNKNOWN_OUTCOME';
export interface ToolDiagnostic {code:string;message:string;severity:'error'|'warning'|'hint';cell_id:string|null;start_utf8:number|null;end_utf8:number|null}
export interface ToolReply {
  ok:boolean;status:'completed'|'partial'|'failed'|'cancelled'|'unknown';request_ref:string;operation_ref:string|null;
  document:{id:string;generation:number;revision:number}|null;data:Record<string,unknown>|null;
  error:{code:ToolErrorCode;message:string;recovery:'none'|'refresh_snapshot'|'query_operation'|'reprepare'|'reduce_request'|'choose_supported_model'|'retry_read';details:Record<string,unknown>}|null;
  pagination:{truncated:boolean;next_cursor:string|null};
}
