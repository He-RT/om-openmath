import type {ModelBinding,AgentCanonicalMessage,ModelResult} from './modelTypes';
import type {ToolReply,ToolName} from './toolTypes';
import type {UserPresentation} from './attachments';
export interface BrowserAgentModel {apiModel:string;displayName:string;supportsTools:boolean;supportsImages?:boolean;temperature?:number;extraBody?:Record<string,unknown>;binding:ModelBinding;url:string;headers:[string,string][];maxOutputTokens:number;timeoutMs:number;contextWindow:number|null}
export interface BrowserAgentTask {
  taskId:string;sessionId:string;documentId:string;generation:number;user:AgentCanonicalMessage;mode:'execute'|'discuss';version:string;model:BrowserAgentModel;
  maxModelRequests?:number;maxToolCalls?:number;maxDurationMs?:number;
  presentation?:UserPresentation;
}
export type AgentRuntimeEvent=
  |{type:'accepted';taskId:string;message:AgentCanonicalMessage}
  |{type:'assistant';taskId:string;message:AgentCanonicalMessage;usage:ModelResult['usage']}
  |{type:'delta';taskId:string;messageId:string;turnId:string;text:string}
  |{type:'tool';taskId:string;callId:string;name:ToolName;reply:ToolReply}
  |{type:'settled';taskId:string;reason:'ended'|'stopped'|'failed'|'budget';requests:number|null;tools:number|null;durationMs:number;error:string|null};
