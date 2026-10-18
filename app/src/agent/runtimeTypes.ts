import type {ModelBinding,AgentCanonicalMessage,ModelResult} from './modelTypes';
import type {ToolReply,ToolName} from './toolTypes';
export interface BrowserAgentModel {apiModel:string;displayName:string;supportsTools:boolean;binding:ModelBinding;url:string;headers:[string,string][];maxOutputTokens:number;timeoutMs:number;contextWindow:number|null}
export interface BrowserAgentTask {
  taskId:string;sessionId:string;documentId:string;generation:number;user:AgentCanonicalMessage;mode:'execute'|'discuss';version:string;model:BrowserAgentModel;
  maxModelRequests?:number;maxToolCalls?:number;maxDurationMs?:number;
}
export type AgentRuntimeEvent=
  |{type:'accepted';taskId:string;message:AgentCanonicalMessage}
  |{type:'assistant';taskId:string;message:AgentCanonicalMessage;usage:ModelResult['usage']}
  |{type:'tool';taskId:string;callId:string;name:ToolName;reply:ToolReply}
  |{type:'settled';taskId:string;reason:'ended'|'stopped'|'failed'|'budget';requests:number|null;tools:number|null;durationMs:number;error:string|null};
