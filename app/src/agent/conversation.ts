import type {AgentStoredEvent,AgentTransactionRecord} from './store';
import type {AgentRuntimeEvent} from './runtimeTypes';
import type {AgentCanonicalMessage} from './modelTypes';
import type {ToolReply,ToolName} from './toolTypes';
import type {UserPresentation} from './attachments';
export interface ConversationAction {id:string;messageId:string;name:ToolName;argumentsJson:string;reply?:ToolReply;cells?:string[];confirmed?:boolean;transaction?:AgentTransactionRecord;preview?:{before:AgentTransactionRecord['before'];after:AgentTransactionRecord['after'];changedCellIds:string[]}}
export type ConversationPart={kind:'text';id:string;messageId:string;text:string;partial?:boolean}|{kind:'actions';id:string;actions:ConversationAction[]};
export interface ConversationTurn {id:string;user:AgentCanonicalMessage;modelId:string;mode:'execute'|'discuss';parts:ConversationPart[];settled?:Omit<Extract<AgentRuntimeEvent,{type:'settled'}>,'durationMs'>&{durationMs:number|null};presentation?:UserPresentation;running:boolean}
function appendMessage(turn:ConversationTurn,message:AgentCanonicalMessage){
  const previous=turn.parts.findIndex(part=>part.kind==='text'&&part.messageId===message.message_id&&part.partial);
  if(previous>=0)turn.parts.splice(previous,1);
  for(const part of message.parts){
    if(part.kind==='text')turn.parts.push({kind:'text',id:message.message_id+':text',messageId:message.message_id,text:part.text});
    else if(part.kind==='tool_call'){
      const action:ConversationAction={id:part.tool_call_id,messageId:message.message_id,name:part.name as ToolName,argumentsJson:part.arguments_json};
      const last=turn.parts.at(-1);if(last?.kind==='actions')last.actions.push(action);else turn.parts.push({kind:'actions',id:message.message_id+':actions',actions:[action]});
    }
  }
}
/** Project original ordered events only; model prose never supplies action success or outcome facts. */
export function projectConversation(events:AgentStoredEvent[],transactions=new Map<string,AgentTransactionRecord>()):ConversationTurn[]{
  const turns:ConversationTurn[]=[],byTask=new Map<string,ConversationTurn>(),actions=new Map<string,ConversationAction>();
  const previews=new Map<string,Extract<AgentStoredEvent['payload'],{kind:'preview'}>>();
  const results=new Map<string,string>();
  const presentations=new Map<string,UserPresentation>();
  for(const {payload}of events){
    if(payload.kind==='user_input'){presentations.set(payload.taskId,payload.presentation);continue;}
    if(payload.kind==='preview'){previews.set(payload.reference,payload);continue;}
    if(payload.kind==='message'){
      const message=payload.record.canonical;if(!message.task_id)continue;
      if(message.role==='user'){
        const turn:ConversationTurn={id:message.task_id,user:message,modelId:payload.modelId??'',mode:payload.mode??'execute',parts:[],running:false};const presentation=presentations.get(turn.id);if(presentation)turn.presentation=presentation;turns.push(turn);byTask.set(turn.id,turn);
      }else if(message.role==='assistant'){
        const turn=byTask.get(message.task_id);if(!turn)continue;appendMessage(turn,message);
        for(const part of turn.parts)if(part.kind==='actions')for(const action of part.actions)actions.set(action.id,action);
      }
    }else if(payload.kind==='delta'){
      const turn=byTask.get(payload.taskId);if(!turn)continue;
      let part=turn.parts.find((part):part is Extract<ConversationPart,{kind:'text'}>=>part.kind==='text'&&part.messageId===payload.messageId);
      if(!part){part={kind:'text',id:payload.messageId+':text',messageId:payload.messageId,text:'',partial:true};turn.parts.push(part);}if(part.partial)part.text+=payload.text;
    }else if(payload.kind==='tool'&&payload.resultJson){
      const action=actions.get(payload.callId);if(!action)continue;
      try{action.reply=JSON.parse(payload.resultJson) as ToolReply;}catch{continue;}
      const data=action.reply.data,ref=typeof data?.transaction_ref==='string'?data.transaction_ref:typeof data?.undo_transaction_ref==='string'?data.undo_transaction_ref:null;
      if(Array.isArray(data?.cells))for(const cell of data.cells as {cell_id?:unknown;result_ref?:unknown}[])if(typeof cell.cell_id==='string'&&typeof cell.result_ref==='string')results.set(cell.result_ref,cell.cell_id);
      if(action.name==='inspect_result'){try{const id=results.get((JSON.parse(action.argumentsJson) as {result_ref:string}).result_ref);if(id)action.cells=[id];}catch{/* Invalid arguments are shown with their actual receipt. */}}
      const transaction=ref?transactions.get(ref):undefined;if(transaction)action.transaction=transaction;
      const preview=action.name==='preview_source'&&typeof data?.preview_ref==='string'?previews.get(data.preview_ref):undefined;if(preview)action.preview=preview;
    }else if(payload.kind==='turn'){
      const turn=byTask.get(payload.taskId);if(turn)turn.running=payload.phase==='started';
    }else if(payload.kind==='settled'){
      const turn=byTask.get(payload.taskId);if(turn){turn.running=false;turn.settled={type:'settled',...payload};}
    }
  }
  for(const action of actions.values())if(action.name==='get_operation_status'&&action.reply?.data?.effect_committed===true&&action.transaction){try{const ref=(JSON.parse(action.argumentsJson) as {operation_ref:string}).operation_ref;for(const original of actions.values())if(original.name==='apply_notebook_patch'&&original.reply?.operation_ref===ref){original.transaction=action.transaction;original.confirmed=true;}}catch{/* No inferred source success without an operation identity. */}}
  return turns;
}
export function turnTransactions(turn:ConversationTurn){return turn.parts.flatMap(part=>part.kind==='actions'?part.actions.flatMap(action=>action.transaction&&action.name==='apply_notebook_patch'?[action.transaction]:[]):[]);}
export function actionCells(action:ConversationAction):string[]{
  if(action.cells)return action.cells;
  const data=action.reply?.data;if(action.transaction)return action.transaction.changedCellIds;
  if(Array.isArray(data?.executed_cell_ids))return data.executed_cell_ids.filter((id):id is string=>typeof id==='string');
  try{const args=JSON.parse(action.argumentsJson) as {cell_ids?:string[]};return args.cell_ids??[];}catch{return [];}
}
