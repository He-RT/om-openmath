import type {NotebookController,UiCell} from '../state/notebookStore';
import type {Messages} from '../i18n';
/** Legacy Ask source is retained verbatim. New conversations belong to the inspector. */
export function AskCell({cell,index,t}:{cell:UiCell;index:number;t:Messages;controller:NotebookController;busy:boolean}){
  return <div className="ask-cell"><pre className="legacy-ask" aria-label={`${t.question} ${index+1}`}>{cell.source}</pre><p className="muted">{t.assistant==='助手'?'旧问答单元格（只读）。继续提问请使用右侧助手。':'Legacy question (read only). Continue in the assistant.'}</p></div>;
}
