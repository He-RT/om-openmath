import {useState} from 'react';
import type {ConversationAction} from '../../agent/conversation';
import {actionCells} from '../../agent/conversation';
import type {UiCell} from '../../state/notebookStore';
import type {Messages} from '../../i18n';
import type {NotebookFile} from '../../kernel/generated/NotebookFile';
import type {AgentEventPayload} from '../../agent/store';
import {agentText as text,cellLabel} from './agentText';
import {SourceDiff} from './SourceDiff';
import {WorkbenchIcon} from '../WorkbenchIcon';
const toolIcons={read_notebook:'book',search_functions:'search',get_function_docs:'search',preview_source:'eye',apply_notebook_patch:'pencil',run_cells:'play',inspect_result:'cube',evaluate_scratch:'play',undo_transaction:'undo',get_operation_status:'eye'} as const;
export function actionStatus(action:ConversationAction,active=true){if(action.confirmed)return 'ok';if(!action.reply)return active?'running':'unknown';if(action.reply.status==='unknown')return 'unknown';if(action.reply.status==='cancelled')return 'cancelled';if(!action.reply.ok||action.name==='preview_source'&&action.reply.data?.valid===false||action.name==='evaluate_scratch'&&action.reply.data?.evaluation_status==='error')return 'failed';return 'ok';}
function actionVerb(action:ConversationAction,t:Messages){const names:Record<ConversationAction['name'],[string,string]>={read_notebook:['读取笔记本','Read notebook'],search_functions:['查阅函数','Search functions'],get_function_docs:['查阅函数','Read function docs'],preview_source:['预览修改','Preview source'],apply_notebook_patch:['修改笔记本','Edit notebook'],run_cells:['运行','Run cells'],inspect_result:['检查结果','Inspect result'],evaluate_scratch:['临时计算','Scratch calculation'],undo_transaction:['撤销修改','Undo changes'],get_operation_status:['核对操作','Check operation']};return text(t,...names[action.name]);}
function ActionRow({action,cells,t,onLocate,onUndo,loadPreview,running}:{action:ConversationAction;cells:UiCell[];t:Messages;onLocate:(id:string)=>void;onUndo:(refs:string[])=>Promise<unknown>;loadPreview:(id:string)=>Promise<Extract<AgentEventPayload,{kind:'preview'}>|undefined>;running:boolean}){
  const [diff,setDiff]=useState<{before:NotebookFile;after:NotebookFile;changedCellIds:string[]}|null>(null),[pending,setPending]=useState(false);
  const status=actionStatus(action,running),ids=actionCells(action),data=action.reply?.data,first=['apply_notebook_patch','run_cells','inspect_result'].includes(action.name)?ids.find(id=>cells.some(cell=>cell.id===id)):undefined;
  let args:Record<string,unknown>={};try{args=JSON.parse(action.argumentsJson) as Record<string,unknown>;}catch{/* The original invalid payload remains in details. */}
  let object=ids.map(id=>cellLabel(t,cells,id)).join('、');
  if(action.name==='read_notebook'&&Array.isArray(data?.cells))object=text(t,`${data.cells.length} 个单元格`,`${data.cells.length} cells`);
  if(action.name==='search_functions')object=String(args.query??'');
  if(action.name==='get_function_docs')object=String(args.name??args.function_id??'');
  if(action.name==='preview_source')object=data?.valid===true?text(t,'检查通过','Check passed'):data?.valid===false?text(t,'源码需要修正','Source needs correction'):'';
  if(action.name==='inspect_result')object=data?.part==='geometry'?text(t,'几何数据','Geometry data'):text(t,'实际结果','Recorded result');
  const canDiff=!!action.transaction||action.name==='preview_source'&&typeof data?.preview_ref==='string';
  return <li className="agent-action" data-action-status={status}>
    <span className="action-tool" aria-hidden="true"><WorkbenchIcon name={toolIcons[action.name]}/></span>
    <div><span className="action-verb">{actionVerb(action,t)}</span> <span className="action-object">{object}</span>
      {!action.confirmed&&action.reply?.error&&<span className="action-error">{action.reply.error.code}{action.reply.error.message!==action.reply.error.code&&` · ${action.reply.error.message}`} · {text(t,'按错误信息修正后重试','Review the error before retrying')}</span>}
      {status==='unknown'&&<span className="action-error">{text(t,'结果尚未确认，请先核对操作回执','Outcome unconfirmed; check the operation receipt first')}</span>}
      <span className="action-links">
        {first&&<button onClick={()=>onLocate(first)}>{t.locate}</button>}
        {canDiff&&<button disabled={pending} onClick={()=>{if(diff){setDiff(null);return;}const value=action.transaction??action.preview;if(value){setDiff(value);return;}setPending(true);void loadPreview(data!.preview_ref as string).then(value=>{if(value)setDiff(value);}).finally(()=>setPending(false));}}>{text(t,'查看差异','View changes')}</button>}
        {action.transaction&&action.name==='apply_notebook_patch'&&<button disabled={pending||!!action.transaction.undoneBy} onClick={()=>{setPending(true);void onUndo([action.transaction!.id]).finally(()=>setPending(false));}}>{action.transaction.undoneBy?text(t,'已撤销','Reverted'):t.undo}</button>}
      </span>
      {diff&&<div className="agent-diff">{diff.before.title!==diff.after.title&&<section><SourceDiff before={diff.before.title} after={diff.after.title} more={t.more}/></section>}{diff.changedCellIds.map(id=>{const before=diff.before.cells.find(cell=>cell.id===id),after=diff.after.cells.find(cell=>cell.id===id);return <section key={id}><strong>{cellLabel(t,cells,id)}</strong>{before?.kind!==after?.kind&&<small>{before?.kind??'∅'} → {after?.kind??'∅'}</small>}<SourceDiff before={before?.source} after={after?.source} more={text(t,'显示其余差异','Show remaining changes')}/></section>;})}</div>}
    </div>
    <span className="action-symbol" aria-hidden="true">{status==='ok'?'✓':status==='failed'?'×':status==='running'?'◌':'○'}</span><span className="sr-only">{status}</span>
  </li>;
}
export function ActionLog({actions,running,...props}:{actions:ConversationAction[];running:boolean;cells:UiCell[];t:Messages;onLocate:(id:string)=>void;onUndo:(refs:string[])=>Promise<unknown>;loadPreview:(id:string)=>Promise<Extract<AgentEventPayload,{kind:'preview'}>|undefined>}){
  const [expanded,setExpanded]=useState<boolean|null>(null),open=running||(expanded??actions.length<=6),failed=actions.filter(action=>actionStatus(action,running)==='failed');
  const visible=open?actions:failed,groups:ConversationAction[][]=[];
  for(const action of visible){const last=groups.at(-1),read=['read_notebook','search_functions','get_function_docs'].includes(action.name)&&actionStatus(action,running)==='ok';if(read&&last&&last.every(item=>['read_notebook','search_functions','get_function_docs'].includes(item.name)&&actionStatus(item,running)==='ok'))last.push(action);else groups.push([action]);}
  const edits=actions.filter(action=>action.name==='apply_notebook_patch'&&action.reply?.ok).length,runs=actions.filter(action=>action.name==='run_cells'&&action.reply?.data).length;
  return <div className="agent-actions"><button className="action-summary" aria-expanded={open} onClick={()=>setExpanded(!open)}>{open?'⌄':'›'} {text(props.t,`${actions.length} 个操作：修改 ${edits} 处，运行 ${runs} 次`,`${actions.length} actions: ${edits} edits, ${runs} runs`)}{failed.length>0&&text(props.t,`，${failed.length} 次失败`,`, ${failed.length} failed`)}</button>
    <ol>{groups.map(group=>group.length>=3?<li key={group[0]!.id}><details className="agent-read-group"><summary>{text(props.t,`读取笔记本与函数说明（${group.length} 个操作）`,`Read notebook and function docs (${group.length} actions)`)}</summary><ol>{group.map(action=><ActionRow key={action.id} action={action} running={running} {...props}/>)}</ol></details></li>:group.map(action=><ActionRow key={action.id} action={action} running={running} {...props}/>))}</ol>
  </div>;
}
