import type {NotebookFile} from '../kernel/generated/NotebookFile';
import type {CellInput} from '../kernel/generated/CellInput';
import {sameSource} from '../state/sourceTransaction';
export type CellSelector={cell_id:string}|{client_key:string};
export type PatchOperation=
  |{type:'insert_cell';client_key:string;after:CellSelector|null;kind:'Math'|'Text';dialect:CellInput['dialect'];source:string}
  |{type:'update_cell';target:{cell_id:string};expected_source_hash:string;source:string;kind?:'Math'|'Text';dialect?:CellInput['dialect']}
  |{type:'replace_text';target:{cell_id:string};expected_source_hash:string;match:string;replacement:string}
  |{type:'delete_cell';target:{cell_id:string};expected_source_hash:string}
  |{type:'move_cell';target:CellSelector;after:CellSelector|null}
  |{type:'rename_notebook';title:string};
export async function sha256(source:string):Promise<string>{
  const bytes=new Uint8Array(await crypto.subtle.digest('SHA-256',new TextEncoder().encode(source)));
  return [...bytes].map(byte=>byte.toString(16).padStart(2,'0')).join('');
}
export interface PatchPlan {before:NotebookFile;after:NotebookFile;assignedCellIds:Record<string,string>;changedCellIds:string[];invalidatedCellIds?:string[];planHash:string}
/** Arguments are closed-schema checked before this function. Hashes always refer to the original UTF-8 source. */
export async function buildPatch(before:NotebookFile,operations:PatchOperation[],exposed:Set<string>,assign:()=>string=()=>crypto.randomUUID(),readable:Set<string>=exposed):Promise<PatchPlan>{
  if(operations.length<1||operations.length>64)throw new Error('INVALID_PATCH');
  operations=structuredClone(operations);
  const frozen=structuredClone(before),after=structuredClone(before),original=new Map(frozen.cells.map(cell=>[cell.id,cell]));
  const hashes=new Map(await Promise.all(frozen.cells.map(async cell=>[cell.id,await sha256(cell.source)] as const)));
  const assignedCellIds:Record<string,string>=Object.create(null) as Record<string,string>,changed=new Set<string>(),deleted=new Set<string>();
  const resolve=(selector:CellSelector)=>{
    const id='cell_id'in selector?selector.cell_id:assignedCellIds[selector.client_key];
    if(!id||'cell_id'in selector&&!original.has(id)||!after.cells.some(cell=>cell.id===id))throw new Error('INVALID_PATCH');
    return id;
  };
  const check=(id:string,hash:string,complete=true)=>{
    if(!(complete?exposed:readable).has(id))throw new Error('INCOMPLETE_SOURCE');
    if(!original.has(id)||hashes.get(id)!==hash||deleted.has(id))throw new Error('STALE_SNAPSHOT');
    const cell=after.cells.find(cell=>cell.id===id);if(!cell)throw new Error('INVALID_PATCH');return cell;
  };
  const position=(selector:CellSelector|null)=>selector===null?0:after.cells.findIndex(cell=>cell.id===resolve(selector))+1;
  for(const operation of operations){
    switch(operation.type){
      case 'insert_cell':{
        if(Object.hasOwn(assignedCellIds,operation.client_key))throw new Error('INVALID_PATCH');
        const index=position(operation.after),id=assign();
        if(!id||original.has(id)||after.cells.some(cell=>cell.id===id))throw new Error('INVALID_PATCH');
        assignedCellIds[operation.client_key]=id;
        after.cells.splice(index,0,{id,source:operation.source,kind:operation.kind,dialect:operation.dialect});changed.add(id);break;
      }
      case 'update_cell':{
        const id=operation.target.cell_id,cell=check(id,operation.expected_source_hash);
        cell.source=operation.source;if(operation.kind)cell.kind=operation.kind;if(operation.dialect)cell.dialect=operation.dialect;
        changed.add(id);break;
      }
      case 'replace_text':{
        const id=operation.target.cell_id,cell=check(id,operation.expected_source_hash,false),index=cell.source.indexOf(operation.match);
        if(!operation.match||index<0||cell.source.indexOf(operation.match,index+1)>=0)throw new Error('NONUNIQUE_REPLACEMENT');
        cell.source=cell.source.slice(0,index)+operation.replacement+cell.source.slice(index+operation.match.length);changed.add(id);break;
      }
      case 'delete_cell':{
        const id=operation.target.cell_id;check(id,operation.expected_source_hash,false);
        after.cells=after.cells.filter(cell=>cell.id!==id);deleted.add(id);changed.add(id);break;
      }
      case 'move_cell':{
        const id=resolve(operation.target),anchor=operation.after===null?null:resolve(operation.after);
        if(id===anchor)throw new Error('INVALID_PATCH');
        const index=after.cells.findIndex(cell=>cell.id===id),cell=after.cells.splice(index,1)[0]!;
        after.cells.splice(anchor===null?0:after.cells.findIndex(cell=>cell.id===anchor)+1,0,cell);changed.add(id);break;
      }
      case 'rename_notebook':after.title=operation.title;break;
    }
  }
  if(after.cells.length>10000||new TextEncoder().encode(JSON.stringify(after)).byteLength>2*1024*1024)throw new Error('LIMIT_EXCEEDED');
  const changedCellIds=[...changed].filter(id=>{
    const old=original.get(id),cell=after.cells.find(cell=>cell.id===id);
    return !old||!cell||!sameSource(old,cell)||frozen.cells.findIndex(cell=>cell.id===id)!==after.cells.findIndex(cell=>cell.id===id);
  });
  return {before:frozen,after,assignedCellIds,changedCellIds,planHash:await sha256(JSON.stringify({before:frozen,after,operations}))};
}
/** Inverse source transaction. Unrelated later changes survive; a touched value/order conflict aborts the whole inverse. */
export function invertSourceTransaction(current:NotebookFile,before:NotebookFile,after:NotebookFile):NotebookFile{
  const output=structuredClone(current),old=new Map(before.cells.map(cell=>[cell.id,cell])),newer=new Map(after.cells.map(cell=>[cell.id,cell]));
  const common=new Set(before.cells.filter(cell=>newer.has(cell.id)).map(cell=>cell.id));
  const oldOrder=before.cells.filter(cell=>common.has(cell.id)).map(cell=>cell.id),afterOrder=after.cells.filter(cell=>common.has(cell.id)).map(cell=>cell.id);
  const moved=JSON.stringify(oldOrder)!==JSON.stringify(afterOrder);
  if(moved&&JSON.stringify(output.cells.filter(cell=>common.has(cell.id)).map(cell=>cell.id))!==JSON.stringify(afterOrder))throw new Error('UNDO_CONFLICT');
  for(const id of new Set([...old.keys(),...newer.keys()])){
    const was=old.get(id),accepted=newer.get(id),now=output.cells.find(cell=>cell.id===id);
    if(was&&accepted&&sameSource(was,accepted))continue;
    if(accepted&&(!now||!sameSource(now,accepted))||!accepted&&now)throw new Error('UNDO_CONFLICT');
    if(was&&accepted)output.cells[output.cells.findIndex(cell=>cell.id===id)]=structuredClone(was);
    else if(!was)output.cells=output.cells.filter(cell=>cell.id!==id);
  }
  if(moved){let index=0;const cells=new Map(output.cells.map(cell=>[cell.id,cell]));output.cells=output.cells.map(cell=>common.has(cell.id)?cells.get(oldOrder[index++]!)!:cell);}
  for(const [index,cell]of before.cells.entries())if(!newer.has(cell.id)){
    const previous=before.cells.slice(0,index).reverse().find(cell=>output.cells.some(item=>item.id===cell.id));
    const next=before.cells.slice(index+1).find(cell=>output.cells.some(item=>item.id===cell.id));
    output.cells.splice(previous?output.cells.findIndex(cell=>cell.id===previous.id)+1:next?output.cells.findIndex(cell=>cell.id===next.id):0,0,structuredClone(cell));
  }
  if(before.title!==after.title){if(output.title!==after.title)throw new Error('UNDO_CONFLICT');output.title=before.title;}
  return output;
}
