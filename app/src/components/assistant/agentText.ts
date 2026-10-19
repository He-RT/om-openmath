import type {Messages} from '../../i18n';
export const agentText=(t:Messages,zh:string,en:string)=>t.assistant==='助手'?zh:en;
export const cellLabel=(t:Messages,cells:{id:string}[],id:string)=>{const index=cells.findIndex(cell=>cell.id===id);return index>=0?t.cellNumber.replace('{n}',String(index+1)):agentText(t,'已删除的单元格','Deleted cell');};
