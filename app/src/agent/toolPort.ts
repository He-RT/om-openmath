import type {NotebookController} from '../state/notebookStore';
import type {NotebookFile} from '../kernel/generated/NotebookFile';
import type {CellOutput} from '../kernel/generated/CellOutput';
import type {CellState} from '../kernel/generated/CellState';
import type {GeneralConfig} from '../kernel/generated/GeneralConfig';
import type {FunctionCatalog} from '../kernel/generated/FunctionCatalog';
import type {HoverInfo} from '../kernel/generated/HoverInfo';
import type {MathCommand,MathResult} from './hostTypes';
import {sameNotebookSource} from '../state/sourceTransaction';
import {sameMathState} from './fences';
export interface ToolSnapshot {
  document:{id:string;generation:number;revision:number};file:NotebookFile;general:GeneralConfig;
  cells:(CellState&{revision:number;output?:CellOutput;outputId?:string;resultSource?:{source:string;dialect:string}})[];
  composing:string[];busy:boolean;
}
export interface ToolPort {
  snapshot():Promise<ToolSnapshot>;
  catalog():Promise<FunctionCatalog>;
  variables():Promise<[string,HoverInfo][]>;
  math(snapshot:ToolSnapshot,command:MathCommand):Promise<MathResult>;
  commit(snapshot:ToolSnapshot,desired:NotebookFile,operationId?:string):Promise<void>;
  sourceWasAccepted?(operationId:string,before:NotebookFile,after:NotebookFile):Promise<boolean>;
}
/** Main-thread source owner. The Agent worker receives this closed RPC surface only. */
export class NotebookToolPort implements ToolPort {
  private revision=0;
  private last:NotebookFile;
  private off:()=>void;
  private stopped=false;
  private ownsMath=false;
  private cancelled=false;
  constructor(private controller:NotebookController,private documentId:string,private generation:number,private mayWrite:boolean,private composing:()=>string[]=()=>[]){
    this.last=controller.file();
    this.off=controller.store.subscribe(()=>{const file=controller.file();if(!sameNotebookSource(this.last,file)){this.revision++;this.last=file;}});
  }
  private current(){
    if(this.stopped||this.controller.store.getState().documentGeneration!==this.generation)throw new Error('STALE_SNAPSHOT');
  }
  async snapshot():Promise<ToolSnapshot>{
    this.current();await this.controller.flush();await this.controller.refresh(false);this.current();
    const state=this.controller.store.getState();if(!state.config)throw new Error('NOT_AVAILABLE');
    return structuredClone({document:{id:this.documentId,generation:this.generation,revision:this.revision},file:this.controller.file(),general:state.config.general,
      cells:state.cells.map(({id,status,defines,uses,revision,exec_count,output,outputId,resultSource})=>({id,status,defines,uses,revision,...(exec_count===undefined?{}:{exec_count}),...(output?{output}:{}),...(outputId?{outputId}:{}),...(resultSource?{resultSource}:{})})),composing:this.composing(),busy:state.busy});
  }
  private admit(snapshot:ToolSnapshot){
    this.current();
    if(this.cancelled)return false;
    const state=this.controller.store.getState();
    return snapshot.document.id===this.documentId&&snapshot.document.generation===this.generation&&snapshot.document.revision===this.revision
      &&sameNotebookSource(this.controller.file(),snapshot.file)&&JSON.stringify(state.config?.general)===JSON.stringify(snapshot.general)
      &&sameMathState(snapshot,{...snapshot,cells:state.cells})&&this.composing().length===0;
  }
  async catalog(){this.current();const response=await this.controller.kernel.request({type:'get_function_catalog'});this.current();if(response.type!=='function_catalog')throw new Error('NOT_AVAILABLE');return response.catalog;}
  async variables(){this.current();const response=await this.controller.kernel.request({type:'get_variables'});this.current();if(response.type!=='variables')throw new Error('NOT_AVAILABLE');return response.items;}
  async math(snapshot:ToolSnapshot,command:MathCommand){
    if(command.type==='run'&&!this.mayWrite)throw new Error('PERMISSION_DENIED');
    const compute=command.type==='run'||command.type==='scratch';
    // A scratch clone is bounded by its own five-second deadline. Terminating the
    // parent worker to stop it would replay definitions and could change random state.
    try{return await this.controller.agentMath({expected:snapshot.file,general:snapshot.general,command},()=>!this.stopped&&this.admit(snapshot),()=>{if(command.type==='run')this.ownsMath=true;});}
    finally{if(compute)this.ownsMath=false;}
  }
  async cancelOwnedWork(){this.cancelled=true;if(this.ownsMath&&this.controller.store.getState().documentGeneration===this.generation&&this.controller.store.getState().busy)await this.controller.interrupt();}
  async commit(snapshot:ToolSnapshot,desired:NotebookFile,operationId?:string){
    if(!this.mayWrite)throw new Error('PERMISSION_DENIED');
    await this.controller.commitSource(snapshot.file,desired,()=>!this.stopped&&this.admit(snapshot),operationId);
  }
  async sourceWasAccepted(operationId:string,before:NotebookFile,after:NotebookFile){this.current();return this.controller.sourceWasAccepted(operationId,before,after);}
  dispose(){this.stopped=true;this.off();}
}
