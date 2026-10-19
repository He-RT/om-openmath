import {useState} from 'react';
import type {ConversationTurn} from '../../agent/conversation';
import {turnTransactions} from '../../agent/conversation';
import type {UiCell} from '../../state/notebookStore';
import type {Messages} from '../../i18n';
import type {SolutionSetView} from '../../kernel/generated/SolutionSetView';
import {agentText as text,cellLabel} from './agentText';
interface Product {type:string;triangles?:number;curves?:number;geometry_exists?:boolean;view?:SolutionSetView;input_form?:string;scratch?:boolean}
function productLabel(product:Product,t:Messages){
  if(product.type==='scene3_d'&&product.geometry_exists)return text(t,`三维场景已生成，${product.triangles??0} 个三角形`,`3D scene generated, ${product.triangles??0} triangles`);
  if(product.type==='plot'&&product.geometry_exists)return text(t,`二维图已生成，${product.curves??0} 条曲线`,`2D plot generated, ${product.curves??0} curves`);
  if(product.type==='solutions'&&product.view){const rows=product.view.solutions,verified=rows.length>0&&rows.every(row=>row.verified==='Exact'),conditional=!!product.view.region_latex||rows.some(row=>!!row.condition_latex);return text(t,`${rows.length} 个解${verified?'，已代入验证':''}${conditional?'，含成立条件':''}`,`${rows.length} solutions${verified?', verified by substitution':''}${conditional?', with conditions':''}`);}
  if(product.type==='expr'&&product.input_form)return text(t,`已核对结果：${product.input_form.slice(0,80)}${product.input_form.length>80?'…':''}`,`Inspected result: ${product.input_form.slice(0,80)}${product.input_form.length>80?'…':''}`);
  return null;
}
export function TurnOutcome({turn,cells,dirty,t,onUndo,onLocate}:{turn:ConversationTurn;cells:UiCell[];dirty:boolean;t:Messages;onUndo:(refs:string[])=>Promise<unknown>;onLocate:(id:string)=>void}){
  const [pending,setPending]=useState(false),settled=turn.settled;if(!settled)return null;
  const end=settled.reason==='ended'?t['agent.turn_ended']:settled.reason==='stopped'?t['agent.stopped']:settled.reason==='budget'?t['agent.budget']:t['agent.failed']+(settled.error??text(t,'原因未知','Unknown reason'));
  const actions=turn.parts.flatMap(part=>part.kind==='actions'?part.actions:[]),transactions=turnTransactions(turn),changed=new Set(transactions.flatMap(tx=>tx.changedCellIds));
  const runs=actions.filter(action=>action.name==='run_cells'&&action.reply?.data),ran=new Set(runs.flatMap(action=>Array.isArray(action.reply?.data?.executed_cell_ids)?action.reply.data.executed_cell_ids as string[]:[]));
  const runCells=runs.flatMap(action=>Array.isArray(action.reply?.data?.cells)?action.reply.data.cells as {status:string;diagnostics?:unknown[]}[]:[]),errors=runCells.filter(cell=>cell.status==='error').length;
  const noDiagnostics=runCells.length>0&&runCells.every(cell=>Array.isArray(cell.diagnostics)&&cell.diagnostics.length===0);
  const products=actions.flatMap(action=>{if(action.name!=='inspect_result'||action.reply?.data?.is_current!==true)return [];const payload=action.reply.data.payload as {items?:Product[]}|null;return (payload?.items??[]).map(item=>({...item,scratch:action.reply?.data?.origin==='scratch'}));});
  const stale=[...changed].filter(id=>{const cell=cells.find(cell=>cell.id===id);if(!cell||cell.kind!=='Math')return false;const edit=actions.map(action=>!!action.transaction&&!action.transaction.undoneBy&&action.transaction.changedCellIds.includes(id)).lastIndexOf(true),run=actions.map(action=>action.name==='run_cells'&&Array.isArray(action.reply?.data?.executed_cell_ids)&&action.reply.data.executed_cell_ids.includes(id)).lastIndexOf(true);return edit>=0&&run<edit;});
  const currentlyStale=[...changed].filter(id=>!stale.includes(id)&&cells.some(cell=>cell.id===id&&cell.kind==='Math'&&cell.status==='Stale'));
  const undoable=transactions.filter(tx=>!tx.undoneBy).map(tx=>tx.id),redoable=[...new Set(transactions.flatMap(tx=>tx.undoneBy?[tx.undoneBy]:[]))],first=[...changed,...ran].find(id=>cells.some(cell=>cell.id===id));
  return <>
    {turn.mode==='execute'&&<section className="agent-outcome" aria-label={text(t,'本轮结果','Turn result')}>
      <h4>{text(t,'本轮结果','Turn result')}</h4><ul>
        {transactions.map(tx=>{const added=tx.changedCellIds.filter(id=>!tx.before.cells.some(cell=>cell.id===id)&&tx.after.cells.some(cell=>cell.id===id)).length,deleted=tx.changedCellIds.filter(id=>!tx.after.cells.some(cell=>cell.id===id)).length,edited=tx.changedCellIds.length-added-deleted;return <li key={tx.id}><span aria-hidden="true">✓</span><span>{tx.undoneBy?text(t,'本轮修改已撤销','Changes from this turn were reverted'):tx.changedCellIds.length?text(t,`新增 ${added}、修改 ${edited}、删除 ${deleted} 个单元格`,`Added ${added}, edited ${edited}, deleted ${deleted} cells`):text(t,`笔记本已重命名为「${tx.after.title}」`,`Notebook renamed to “${tx.after.title}”`)}</span></li>;})}
        {ran.size>0&&<li><span aria-hidden="true">{errors?'⚠':'✓'}</span><span>{text(t,`运行 ${ran.size} 个单元格${errors?`，${errors} 个出错`:noDiagnostics?'，没有诊断':''}`,`Ran ${ran.size} cells${errors?`, ${errors} errors`:noDiagnostics?', no diagnostics':''}`)}</span></li>}
        {products.map((product,index)=>{const label=productLabel(product,t);return label?<li key={index}><span aria-hidden="true">✓</span><span>{product.scratch&&text(t,'临时计算：','Scratch calculation: ')}{label}</span></li>:null;})}
        {stale.length>0&&<li className="outcome-warning"><span aria-hidden="true">⚠</span><span>{stale.map(id=>cellLabel(t,cells,id)).join('、')} {text(t,'修改后未更新结果','changed without updated results')}</span></li>}
        {currentlyStale.length>0&&<li className="outcome-warning"><span aria-hidden="true">⚠</span><span>{currentlyStale.map(id=>cellLabel(t,cells,id)).join('、')} {text(t,'当前结果需要重新运行','current results need rerunning')}</span></li>}
        {changed.size>0&&<li className="outcome-save"><span aria-hidden="true">◷</span><span>{dirty?text(t,'尚未保存到文件（⌘S）','Not saved to a file (⌘S)'):text(t,'当前笔记本已下载或保存','Current notebook downloaded or saved')}</span></li>}
      </ul>
      <div className="outcome-actions">{undoable.length>0&&<button disabled={pending} onClick={()=>{setPending(true);void onUndo(undoable).finally(()=>setPending(false));}}>{text(t,'撤销本轮修改','Undo turn changes')}</button>}{redoable.length>0&&<button disabled={pending} onClick={()=>{setPending(true);void onUndo(redoable).finally(()=>setPending(false));}}>{text(t,'重做','Redo')}</button>}{first&&<button onClick={()=>onLocate(first)}>{text(t,'定位单元格','Locate cell')}</button>}</div>
    </section>}
    <div className="agent-end" role="status">{end} · {settled.durationMs===null?text(t,'用时未知','duration unknown'):`${Math.round(settled.durationMs/1000)} ${text(t,'秒','s')}`} · {settled.requests===null?text(t,'请求次数未知','request count unknown'):text(t,`${settled.requests} 次模型请求`,`${settled.requests} model requests`)}</div>
  </>;
}
