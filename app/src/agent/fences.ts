import type {ToolSnapshot} from './toolPort';
/** A source-stable notebook can still have newer accepted definitions/results. */
export function sameMathState(a:ToolSnapshot,b:ToolSnapshot){
  const facts=(snapshot:ToolSnapshot)=>snapshot.cells.map(cell=>({id:cell.id,status:cell.status,exec_count:cell.exec_count,outputId:cell.outputId}));
  return JSON.stringify(a.general)===JSON.stringify(b.general)&&JSON.stringify(facts(a))===JSON.stringify(facts(b));
}
