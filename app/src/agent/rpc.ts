import type {ToolPort,ToolSnapshot} from './toolPort';
import type {FunctionCatalog} from '../kernel/generated/FunctionCatalog';
import type {HoverInfo} from '../kernel/generated/HoverInfo';
import type {MathCommand,MathResult} from './hostTypes';
import type {NotebookFile} from '../kernel/generated/NotebookFile';
export type OwnerRequest={type:'snapshot'}|{type:'catalog'}|{type:'variables'}|{type:'math';snapshot:ToolSnapshot;command:MathCommand}|{type:'commit';snapshot:ToolSnapshot;desired:NotebookFile;operationId?:string}|{type:'confirm_source';operationId:string;before:NotebookFile;after:NotebookFile};
type OwnerPacket={id:number;request:OwnerRequest};
type OwnerResponse={id:number;ok:true;value:unknown}|{id:number;ok:false;error:string};
/** Dedicated MessagePort; neither provider text nor model tool arguments can choose RPC methods. */
export class WorkerToolPort implements ToolPort{
  private serial=0;private pending=new Map<number,{resolve:(value:unknown)=>void;reject:(error:Error)=>void}>();private closed=false;
  constructor(private port:MessagePort){
    port.onmessage=(event:MessageEvent<OwnerResponse>)=>{const value=event.data,pending=this.pending.get(value.id);if(!pending)return;this.pending.delete(value.id);if(value.ok)pending.resolve(value.value);else pending.reject(new Error(value.error));};
    port.start();
  }
  private request<T>(request:OwnerRequest):Promise<T>{
    if(this.closed)return Promise.reject(new Error('INVALID_REFERENCE'));
    const id=++this.serial;
    return new Promise<T>((resolve,reject)=>{this.pending.set(id,{resolve:value=>resolve(value as T),reject});this.port.postMessage({id,request} satisfies OwnerPacket);});
  }
  snapshot(){return this.request<ToolSnapshot>({type:'snapshot'});}
  catalog(){return this.request<FunctionCatalog>({type:'catalog'});}
  variables(){return this.request<[string,HoverInfo][]>({type:'variables'});}
  math(snapshot:ToolSnapshot,command:MathCommand){return this.request<MathResult>({type:'math',snapshot,command});}
  commit(snapshot:ToolSnapshot,desired:NotebookFile,operationId?:string){return this.request<void>({type:'commit',snapshot,desired,...(operationId?{operationId}:{})});}
  sourceWasAccepted(operationId:string,before:NotebookFile,after:NotebookFile){return this.request<boolean>({type:'confirm_source',operationId,before,after});}
  close(){this.closed=true;this.port.close();for(const pending of this.pending.values())pending.reject(new Error('UNKNOWN_OUTCOME'));this.pending.clear();}
}
export function bindToolPort(port:MessagePort,owner:ToolPort){
  let closed=false;
  port.onmessage=(event:MessageEvent<OwnerPacket>)=>{
    const {id,request}=event.data;
    void(async()=>{
      try{
        let value:unknown;
        switch(request.type){case 'snapshot':value=await owner.snapshot();break;case 'catalog':value=await owner.catalog();break;case 'variables':value=await owner.variables();break;case 'math':value=await owner.math(request.snapshot,request.command);break;case 'commit':value=await owner.commit(request.snapshot,request.desired,request.operationId);break;case 'confirm_source':value=await owner.sourceWasAccepted?.(request.operationId,request.before,request.after)??false;break;default:throw new Error('INVALID_ARGUMENT');}
        if(!closed)port.postMessage({id,ok:true,value} satisfies OwnerResponse);
      }catch(error){if(!closed)port.postMessage({id,ok:false,error:error instanceof Error?error.message:'NOT_AVAILABLE'} satisfies OwnerResponse);}
    })();
  };
  port.start();return()=>{closed=true;port.close();};
}
