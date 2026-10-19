import {agent_tool_declarations,validate_agent_payload} from '../kernel/wasm/om_wasm.js';
import type {ToolPort,ToolSnapshot} from './toolPort';
import {AgentStore,type AgentOperationRecord,type AgentTransactionRecord} from './store';
import {buildPatch,invertSourceTransaction,sha256,type PatchOperation,type PatchPlan} from './patch';
import {toolNames,type ToolName,type ToolReply,type ToolDiagnostic,type ToolErrorCode} from './toolTypes';
import type {CellOutput} from '../kernel/generated/CellOutput';
import type {Dialect} from '../kernel/generated/Dialect';
import type {Diagnostic} from '../kernel/generated/Diagnostic';
import {sameNotebookSource} from '../state/sourceTransaction';
import {sameMathState} from './fences';
interface SnapshotRef {kind:'snapshot';snapshot:ToolSnapshot;exposed:Set<string>;readable?:Set<string>}
interface PreviewRef {kind:'preview';snapshot:ToolSnapshot;plan:PatchPlan;operationId:string}
interface ResultRef {kind:'result';snapshot:ToolSnapshot;origin:'notebook'|'scratch';cellId:string|null;sourceHash:string|null;execCount:number|undefined;outputId:string|undefined;output:CellOutput;status:string}
interface CursorRef {kind:'cursor';owner:string;part:string;offset:number;chunks?:string[];ids?:string[];array?:boolean;after?:string}
type Ref=SnapshotRef|PreviewRef|ResultRef|CursorRef;
interface Lease {value:Ref;expires:number;bytes:number}
const effects=new Set<ToolName>(['apply_notebook_patch','run_cells','undo_transaction']);
const blockedDiscuss=new Set<ToolName>(['preview_source',...effects]);
const codes=new Set<ToolErrorCode>(['INVALID_ARGUMENT','NOT_AVAILABLE','PERMISSION_DENIED','INVALID_REFERENCE','STALE_SNAPSHOT','EDITING_BUSY','INVALID_SOURCE','INVALID_PATCH','PREVIEW_MISMATCH','DEPENDENCY_NOT_READY','EXECUTION_FAILED','STALE_RESULT','CONTEXT_NOT_READY','LIMIT_EXCEEDED','TIMEOUT','CANCELLED','NOT_SUPPORTED','PREPARATION_REQUIRED','MODEL_INCOMPATIBLE','UNDO_CONFLICT','UNKNOWN_OUTCOME']);
const bytes=(value:unknown)=>new TextEncoder().encode(JSON.stringify(value)).byteLength;
const reference=(prefix:string)=>prefix+'-'+crypto.randomUUID();
const diag=(item:Diagnostic,cellId:string|null):ToolDiagnostic=>({code:item.code,message:item.message,severity:item.severity==='Error'?'error':item.severity==='Warning'?'warning':'hint',cell_id:cellId,start_utf8:item.span.start,end_utf8:item.span.end});
function diagnostics(output:CellOutput,cellId:string|null):ToolDiagnostic[]{
  return output.messages.slice(0,100).map(message=>({code:message.tag||'KERNEL_MESSAGE',message:message.text,severity:message.level==='Error'?'error':message.level==='Warning'?'warning':'hint',cell_id:cellId,start_utf8:null,end_utf8:null}));
}
function summary(output:CellOutput){
  return {items:output.items.map(item=>{
    if(item.type==='plot')return {type:item.type,curves:item.data?.curves.length??0,points:item.data?.curves.reduce((n,c)=>n+c.segments.reduce((m,s)=>m+s.length,0),0)??0,x_range:item.data?.x_range??item.request.x_range,y_range:item.data?.y_range??item.request.y_range,geometry_exists:!!item.data,skipped:item.data?.geometry?.skipped??0};
    if(item.type==='scene3_d')return {type:item.type,geometry_exists:!!item.data,meshes:item.data?.meshes.length??0,triangles:item.data?.meshes.reduce((n,m)=>n+m.triangles.length,0)??0,bounds:item.data?.bounds??null,sampled:item.data?.sampled??null,unavailable:item.unavailable};
    if(item.type==='solutions')return {type:item.type,out_index:item.out_index,view:item.view};
    if(item.type==='expr')return {type:item.type,out_index:item.out_index,input_form:item.input_form,nature:item.nature??null,unchanged:item.unchanged??null};
    if(item.type==='error')return item;
    return {type:item.type};
  }),messages:output.messages,timing_ms:output.timing_ms};
}
export interface ToolHostOptions {documentId:string;generation:number;sessionId:string;taskId:string;mode:'execute'|'discuss';signal:AbortSignal}
/** Worker-side closed tools. Model arguments never supply owner identity, grants, endpoint or kernel commands. */
export class ToolHost {
  private leases=new Map<string,Lease>();private retained=0;private recent:string[]=[];private closed=false;
  private serial=Promise.resolve();
  constructor(private port:ToolPort,private store:AgentStore,private options:ToolHostOptions){}
  declarations(){
    const names=toolNames.filter(name=>this.options.mode==='execute'||!blockedDiscuss.has(name));
    return JSON.parse(agent_tool_declarations(JSON.stringify(names))) as {name:ToolName;label:string;description:string;parameters:Record<string,unknown>}[];
  }
  async initialize(){const snapshot=await this.current();await this.store.createSession(this.options.documentId,this.options.sessionId,'');await this.store.putDocument({...snapshot.document,file:snapshot.file,updatedAt:Date.now()});}
  dispose(){this.closed=true;this.leases.clear();this.retained=0;}
  private check(){if(this.closed)throw new Error('INVALID_REFERENCE');if(this.options.signal.aborted)throw new Error('CANCELLED');}
  private async current(allowStopped=false){
    if(this.closed)throw new Error('INVALID_REFERENCE');if(!allowStopped)this.check();const snapshot=await this.port.snapshot();if(!allowStopped)this.check();
    if(snapshot.document.id!==this.options.documentId||snapshot.document.generation!==this.options.generation)throw new Error('STALE_SNAPSHOT');
    return snapshot;
  }
  private put(value:Ref){
    const size=bytes(value.kind==='snapshot'?value.snapshot:value);
    if(size>32*1024*1024)throw new Error('LIMIT_EXCEEDED');
    while(this.leases.size>=128||this.retained+size>32*1024*1024){const [id,lease]=this.leases.entries().next().value!;this.leases.delete(id);this.retained-=lease.bytes;}
    const id=reference(value.kind);this.leases.set(id,{value:structuredClone(value),expires:performance.now()+300000,bytes:size});this.retained+=size;return id;
  }
  private get<T extends Ref['kind']>(id:string,kind:T):Extract<Ref,{kind:T}>{
    const lease=this.leases.get(id);
    if(!lease||lease.expires<performance.now()||lease.value.kind!==kind)throw new Error('INVALID_REFERENCE');
    return lease.value as Extract<Ref,{kind:T}>;
  }
  private async snapshotRef(id:string){
    const ref=this.get(id,'snapshot'),current=await this.current();
    if(current.document.revision!==ref.snapshot.document.revision||!sameNotebookSource(current.file,ref.snapshot.file)||!sameMathState(current,ref.snapshot))throw new Error('STALE_SNAPSHOT');
    return {...ref,snapshot:current};
  }
  private envelope(snapshot:ToolSnapshot,data:Record<string,unknown>,pagination={truncated:false,next_cursor:null as string|null}):ToolReply{
    return {ok:true,status:'completed',request_ref:reference('request'),operation_ref:null,document:snapshot.document,data,error:null,pagination};
  }
  invoke(name:ToolName,argsJson:string,callId:string):Promise<ToolReply>{
    let result!:ToolReply;
    const operation=this.serial.then(async()=>{result=await this.dispatch(name,argsJson,callId);
      await this.store.append(this.options.documentId,this.options.sessionId,{kind:'tool',callId,name,operationRef:result.operation_ref,argumentsJson:argsJson,resultJson:JSON.stringify(result)});
    });
    this.serial=operation.catch(()=>{});return operation.then(()=>result);
  }
  private async dispatch(name:ToolName,argsJson:string,callId:string):Promise<ToolReply>{
    let snapshot:ToolSnapshot|undefined;
    try{
      if(!toolNames.includes(name))throw new Error('INVALID_ARGUMENT');
      try{validate_agent_payload(name,'args',argsJson);}catch{throw new Error('INVALID_ARGUMENT');}
      this.check();if(this.options.mode==='discuss'&&blockedDiscuss.has(name))throw new Error('PERMISSION_DENIED');
      snapshot=await this.current();const args=JSON.parse(argsJson) as Record<string,unknown>;
      let result:ToolReply;
      switch(name){
        case 'read_notebook':result=await this.read(snapshot,args);break;
        case 'search_functions':result=await this.search(snapshot,args);break;
        case 'get_function_docs':result=await this.docs(snapshot,args);break;
        case 'preview_source':result=await this.preview(args);break;
        case 'apply_notebook_patch':result=await this.apply(args,callId);break;
        case 'run_cells':result=await this.run(args,callId);break;
        case 'inspect_result':result=await this.inspect(args);break;
        case 'evaluate_scratch':result=await this.scratch(args);break;
        case 'undo_transaction':result=await this.undo(args,callId);break;
        case 'get_operation_status':result=await this.operation(snapshot,args);break;
      }
      validate_agent_payload(name,'result',JSON.stringify(result));return result;
    }catch(error){
      const message=error instanceof Error?error.message:String(error);
      const code:ToolErrorCode=codes.has(message as ToolErrorCode)?message as ToolErrorCode:message==='NONUNIQUE_REPLACEMENT'||message==='INCOMPLETE_SOURCE'?'INVALID_PATCH':'NOT_AVAILABLE';
      return {ok:false,status:code==='CANCELLED'?'cancelled':'failed',request_ref:reference('request'),operation_ref:null,document:snapshot?.document??null,data:null,error:{code,message:code,recovery:code==='STALE_SNAPSHOT'||code==='INVALID_REFERENCE'?'refresh_snapshot':code==='LIMIT_EXCEEDED'?'reduce_request':'none',details:{}},pagination:{truncated:false,next_cursor:null}};
    }
  }
  private result(snapshot:ToolSnapshot,cellId:string|null,output:CellOutput,origin:'notebook'|'scratch',status:string,producer?:{source:string;execCount:number|undefined;outputId:string|undefined}){
    const cell=cellId?snapshot.cells.find(cell=>cell.id===cellId):undefined;
    return this.put({kind:'result',snapshot,origin,cellId,sourceHash:producer?.source??cell?.resultSource?.source??(cellId?snapshot.file.cells.find(cell=>cell.id===cellId)?.source??null:null),execCount:producer?.execCount??cell?.exec_count,outputId:producer?.outputId??cell?.outputId,output,status});
  }
  private async read(snapshot:ToolSnapshot,args:Record<string,unknown>){
    let offset=0,ids=args.cell_ids as string[]|undefined,view=(args.view as string|undefined)??'outline';
    if(args.snapshot_ref)snapshot=(await this.snapshotRef(args.snapshot_ref as string)).snapshot;
    if(args.cursor){
      const cursor=this.get(args.cursor as string,'cursor');
      if(args.snapshot_ref&&args.snapshot_ref!==cursor.owner)throw new Error('INVALID_REFERENCE');
      const lease=await this.snapshotRef(cursor.owner);snapshot=lease.snapshot;
      if(cursor.part==='source'){
        const cell=snapshot.file.cells.find(cell=>cell.id===cursor.ids?.[0]);if(!cell)throw new Error('INVALID_REFERENCE');if(snapshot.composing.includes(cell.id))throw new Error('EDITING_BUSY');
        let text=cell.source.slice(cursor.offset,cursor.offset+32000);if(/[\uD800-\uDBFF]$/.test(text))text=text.slice(0,-1);
        const nextOffset=cursor.offset+text.length,next=nextOffset<cell.source.length?this.put({...cursor,offset:nextOffset}):null;
        const meta=snapshot.cells.find(meta=>meta.id===cell.id);
        return this.envelope(snapshot,{snapshot_ref:cursor.owner,is_current:true,title:snapshot.file.title,cells:[{cell_id:cell.id,kind:cell.kind,dialect:cell.dialect,source_hash:await sha256(cell.source),revision:meta?.revision??0,status:meta?.status??'Stale',source:text,source_complete:false,source_cursor:next,result_ref:null}],symbols:[],recent_transaction_refs:this.recent.slice(-32)},{truncated:next!==null,next_cursor:next});
      }
      if(!cursor.part.startsWith('notebook:'))throw new Error('INVALID_REFERENCE');
      view=cursor.part.slice('notebook:'.length);offset=cursor.offset;ids=cursor.ids;
    }
    const limit=(args.limit as number|undefined)??20;
    if(view==='symbols'){
      const all=await this.port.variables(),symbols=all.slice(offset,offset+limit).map(([name,info])=>({name,...info,execution_status:snapshot.cells.find(cell=>cell.id===info.cell_id)?.status??'unavailable',is_current:snapshot.cells.find(cell=>cell.id===info.cell_id)?.status==='Done'}));
      const root=this.put({kind:'snapshot',snapshot,exposed:new Set<string>()});
      const next=offset+limit<all.length?this.put({kind:'cursor',owner:root,part:'notebook:symbols',offset:offset+limit}):null;
      return this.envelope(snapshot,{snapshot_ref:root,is_current:true,title:snapshot.file.title,cells:[],symbols,recent_transaction_refs:this.recent.slice(-32)},{truncated:next!==null,next_cursor:next});
    }
    if(ids?.some(id=>!snapshot.file.cells.some(cell=>cell.id===id)))throw new Error('INVALID_REFERENCE');
    const chosen=ids?snapshot.file.cells.filter(cell=>ids!.includes(cell.id)):snapshot.file.cells;
    const exposed=new Set<string>(),readable=new Set<string>(),longSources:{index:number;id:string;offset:number}[]=[];
    const cells:{cell_id:string;kind:string;dialect:Dialect;source_hash:string;revision:number;status:string;source:string|null;source_complete:boolean;source_cursor:string|null;result_ref:string|null}[]=[];
    let size=0,index=offset;
    for(;index<chosen.length&&cells.length<limit;index++){
      const cell=chosen[index]!,meta=snapshot.cells.find(meta=>meta.id===cell.id);
      let source=cell.source.slice(0,view==='cells'?32000:160);if(/[\uD800-\uDBFF]$/.test(source))source=source.slice(0,-1);
      const composing=snapshot.composing.includes(cell.id),complete=!composing&&view==='cells'&&source.length===cell.source.length;
      const row={cell_id:cell.id,kind:cell.kind,dialect:cell.dialect,source_hash:await sha256(cell.source),revision:meta?.revision??0,status:meta?.status??'Stale',source:composing?null:source,source_complete:complete,source_cursor:null as string|null,result_ref:null as string|null};
      const rowSize=bytes(row);if(cells.length&&size+rowSize>196608)break;size+=rowSize;
      if(complete)exposed.add(cell.id);if(view==='cells'&&!composing)readable.add(cell.id);
      if(view==='cells'&&!complete&&!composing)longSources.push({index:cells.length,id:cell.id,offset:source.length});
      if(args.include_outputs&&meta?.output)row.result_ref=this.result(snapshot,cell.id,meta.output,'notebook',meta.status);
      cells.push(row);
    }
    const root=this.put({kind:'snapshot',snapshot,exposed,readable});
    for(const source of longSources)cells[source.index]!.source_cursor=this.put({kind:'cursor',owner:root,part:'source',offset:source.offset,ids:[source.id]});
    const next=index<chosen.length?this.put({kind:'cursor',owner:root,part:'notebook:'+view,offset:index,...(ids?{ids}:{})}):null;
    return this.envelope(snapshot,{snapshot_ref:root,is_current:true,title:snapshot.file.title,cells,symbols:[],recent_transaction_refs:this.recent.slice(-32)},{truncated:next!==null,next_cursor:next});
  }
  private async search(snapshot:ToolSnapshot,args:Record<string,unknown>){
    const result=await this.port.math(snapshot,{type:'catalog'});if(result.type!=='catalog')throw new Error('NOT_AVAILABLE');
    const catalog=result.catalog,query=(args.query as string).toLocaleLowerCase(),limit=(args.limit as number|undefined)??8;
    const functions=catalog.functions.filter(fn=>!args.category||fn.category.toLocaleLowerCase()===(args.category as string).toLocaleLowerCase())
      .map(fn=>({fn,exact:[fn.descriptor.name,fn.descriptor.modern_name,fn.descriptor.id,...fn.descriptor.aliases].some(name=>name.toLocaleLowerCase()===query)}))
      .filter(({fn,exact})=>exact||[fn.descriptor.name,fn.descriptor.modern_name,...fn.descriptor.aliases,fn.signature,fn.summary_zh,fn.summary_en,fn.category].join(' ').toLocaleLowerCase().includes(query))
      .sort((a,b)=>Number(b.exact)-Number(a.exact)||a.fn.descriptor.name.localeCompare(b.fn.descriptor.name)).slice(0,limit)
      .map(({fn})=>({function_id:fn.descriptor.id,name:fn.descriptor.modern_name||fn.descriptor.name,signature:fn.signature,support:fn.descriptor.support,metadata_version:catalog.metadata_version}));
    return this.envelope(snapshot,{functions,metadata_version:catalog.metadata_version});
  }
  private async docs(snapshot:ToolSnapshot,args:Record<string,unknown>){
    const result=await this.port.math(snapshot,{type:'catalog'});if(result.type!=='catalog')throw new Error('NOT_AVAILABLE');
    const {catalog,context}=result;
    if(args.metadata_version!==undefined&&args.metadata_version!==catalog.metadata_version)throw new Error('STALE_SNAPSHOT');
    const owner=args.name?context.owned.find(owner=>owner.symbol===args.name):undefined;
    if(owner){
      const cell=snapshot.cells.find(cell=>cell.id===owner.cell_id);if(cell?.status!=='Done')throw new Error('CONTEXT_NOT_READY');
      return this.envelope(snapshot,{function:{kind:'user_binding',name:args.name,callable:context.known_functions.includes(args.name as string),cell_id:owner.cell_id,source:null,source_complete:false,support:'Current accepted user binding overrides the built-in alias; read its cell for source.'},examples:[],metadata_version:catalog.metadata_version});
    }
    const fn=catalog.functions.find(fn=>args.function_id?fn.descriptor.id===args.function_id:fn.descriptor.name===args.name)
      ??catalog.functions.find(fn=>args.name&&[fn.descriptor.modern_name,...fn.descriptor.aliases].some(name=>name.toLocaleLowerCase()===(args.name as string).toLocaleLowerCase()));
    if(!fn)throw new Error('NOT_AVAILABLE');
    return this.envelope(snapshot,{function:fn,examples:fn.examples,metadata_version:catalog.metadata_version});
  }
  private async preview(args:Record<string,unknown>){
    const {snapshot,exposed,readable}=await this.snapshotRef(args.snapshot_ref as string);
    if(snapshot.composing.length)throw new Error('EDITING_BUSY');
    const input=args.input as {kind:'source';source:string;dialect:Dialect}|{kind:'patch';operations:PatchOperation[]};
    if(input.kind==='source'){
      const result=await this.port.math(snapshot,{type:'source_preview',source:input.source,dialect:input.dialect});if(result.type!=='source_preview')throw new Error('NOT_AVAILABLE');
      const issues=result.diagnostics.map(item=>diag(item,null));
      return this.envelope(snapshot,{preview_kind:'source',valid:!issues.some(item=>item.severity==='error'),preview_ref:null,plan_hash:await sha256(JSON.stringify(input)),diagnostics:issues.slice(0,100),affected_cell_ids:[],assigned_cell_ids:{}});
    }
    const plan=await buildPatch(snapshot.file,input.operations,exposed,undefined,readable??exposed),file=plan.after;
    const result=await this.port.math(snapshot,{type:'preview',file});if(result.type!=='preview')throw new Error('NOT_AVAILABLE');
    const changed=new Set(plan?.changedCellIds??[file.cells.at(-1)!.id]);
    const issues=result.analysis.cells.filter(cell=>changed.has(cell.cell_id)).flatMap(cell=>cell.diagnostics.map(item=>diag(item,cell.cell_id)));
    for(const conflict of result.analysis.conflicts)if(conflict.cell_ids.some(id=>changed.has(id)))issues.push({code:'MULTIPLE_DEFINITIONS',message:conflict.symbol,severity:'error',cell_id:conflict.cell_ids[0]??null,start_utf8:null,end_utf8:null});
    const valid=!issues.some(item=>item.severity==='error');
    if(plan)plan.invalidatedCellIds=result.affected_cell_ids;
    const previewRef=valid&&plan?this.put({kind:'preview',snapshot,plan,operationId:reference('operation')}):null;
    if(previewRef)await this.store.append(this.options.documentId,this.options.sessionId,{kind:'preview',reference:previewRef,before:plan.before,after:plan.after,changedCellIds:plan.changedCellIds});
    return this.envelope(snapshot,{preview_kind:input.kind,valid,preview_ref:previewRef,plan_hash:plan?.planHash??await sha256(JSON.stringify(file)),diagnostics:issues.slice(0,100),affected_cell_ids:[...new Set([...result.affected_cell_ids,...changed])],assigned_cell_ids:plan?.assignedCellIds??{}});
  }
  private async begin(tool:ToolName,args:unknown,callId:string,id?:string,intent?:AgentOperationRecord['intent']){
    const operation:AgentOperationRecord={id:id??'operation-'+await sha256(this.options.taskId+':'+callId),documentId:this.options.documentId,sessionId:this.options.sessionId,taskId:this.options.taskId,tool,inputHash:await sha256(JSON.stringify(args)),state:'prepared',effectCommitted:null,transactionRef:null,resultJson:null,resultRefs:[],updatedAt:Date.now()};
    return this.store.beginOperation({...operation,...(intent?{intent}:{})});
  }
  private async confirmOperation(record:AgentOperationRecord,snapshot:ToolSnapshot){
    if(record.state==='completed'||record.state==='failed'||record.state==='cancelled')return record;
    const transaction=record.intent?.transaction;
    if(!transaction||!await this.port.sourceWasAccepted?.(record.id,transaction.before,transaction.after))return record;
    const confirmed={...record,state:'completed' as const,effectCommitted:true,transactionRef:transaction.id,updatedAt:Date.now()};
    return this.store.settleOperation(confirmed,transaction,{...snapshot.document,file:snapshot.file,updatedAt:Date.now()});
  }
  private async pending(snapshot:ToolSnapshot){
    for(const record of await this.store.pendingOperations(this.options.documentId)){
      const checked=await this.confirmOperation(record,snapshot);if(checked.state!=='completed'&&checked.state!=='failed'&&checked.state!=='cancelled')return checked;
    }
    return undefined;
  }
  private recordedReply(snapshot:ToolSnapshot,record:AgentOperationRecord):ToolReply{
    if(!['completed','failed','cancelled'].includes(record.state))return this.unknown(snapshot,record);
    if(record.resultJson)return JSON.parse(record.resultJson) as ToolReply;
    return {ok:false,status:'failed',request_ref:reference('request'),operation_ref:record.id,document:snapshot.document,data:null,error:{code:'NOT_AVAILABLE',message:'The operation is known; query its receipt instead of replaying it.',recovery:'query_operation',details:{}},pagination:{truncated:false,next_cursor:null}};
  }
  private async sourceEffect(name:'apply_notebook_patch'|'undo_transaction',snapshot:ToolSnapshot,plan:PatchPlan,callId:string,id:string|undefined,undoes?:string[]){
    const transaction:AgentTransactionRecord={id:reference('transaction'),documentId:this.options.documentId,sessionId:this.options.sessionId,taskId:this.options.taskId,before:plan.before,after:plan.after,changedCellIds:plan.changedCellIds,createdAt:Date.now(),undoneBy:null,...(undoes?{undoes}:{})};
    const pending=await this.pending(snapshot);if(pending)return this.unknown(snapshot,pending);
    const start=await this.begin(name,{plan_hash:plan.planHash},callId,id,{transaction,planHash:plan.planHash});
    if(!start.created)return this.recordedReply(snapshot,start.record);
    let committed=false;
    try{
      this.check();await this.port.commit(snapshot,plan.after,start.record.id);committed=true;
      const after=await this.current(true),snapshotRef=this.put({kind:'snapshot',snapshot:after,exposed:new Set<string>()});
      const invalidated=plan.invalidatedCellIds??after.cells.filter(cell=>cell.status==='Stale'&&snapshot.cells.find(old=>old.id===cell.id)?.status!=='Stale').map(cell=>cell.id);
      const data=name==='apply_notebook_patch'?{transaction_ref:transaction.id,snapshot_ref:snapshotRef,changed_cell_ids:plan.changedCellIds,invalidated_cell_ids:invalidated,plan_hash:plan.planHash,saved:false}:{undo_transaction_ref:transaction.id,snapshot_ref:snapshotRef,changed_cell_ids:plan.changedCellIds,invalidated_cell_ids:invalidated,saved:false};
      const reply={...this.envelope(after,data),operation_ref:start.record.id};
      validate_agent_payload(name,'result',JSON.stringify(reply));
      await this.store.settleOperation({...start.record,state:'completed',effectCommitted:true,transactionRef:transaction.id,resultJson:JSON.stringify(reply),updatedAt:Date.now()},transaction,{...after.document,file:after.file,updatedAt:Date.now()});
      this.recent.push(transaction.id);return reply;
    }catch(error){
      const code=error instanceof Error?error.message:'';
      if(!committed&&['STALE_SNAPSHOT','PERMISSION_DENIED','EDITING_BUSY','CANCELLED'].includes(code)){
        const reply:ToolReply={ok:false,status:code==='CANCELLED'?'cancelled':'failed',request_ref:reference('request'),operation_ref:start.record.id,document:snapshot.document,data:null,error:{code:code as ToolErrorCode,message:code,recovery:code==='STALE_SNAPSHOT'?'refresh_snapshot':'none',details:{}},pagination:{truncated:false,next_cursor:null}};await this.store.settleOperation({...start.record,state:code==='CANCELLED'?'cancelled':'failed',effectCommitted:false,resultJson:JSON.stringify(reply),updatedAt:Date.now()});return reply;
      }
      const unknown={...start.record,state:'unknown' as const,effectCommitted:committed?true:null,updatedAt:Date.now()};await this.store.settleOperation(unknown).catch(()=>{});return this.unknown(snapshot,unknown);
    }
  }
  private unknown(snapshot:ToolSnapshot,operation:AgentOperationRecord):ToolReply{
    return {ok:false,status:'unknown',request_ref:reference('request'),operation_ref:operation.id,document:snapshot.document,data:null,error:{code:'UNKNOWN_OUTCOME',message:'UNKNOWN_OUTCOME',recovery:'query_operation',details:{}},pagination:{truncated:false,next_cursor:null}};
  }
  private async apply(args:Record<string,unknown>,callId:string){
    const preview=this.get(args.preview_ref as string,'preview');
    const old=await this.store.operation(this.options.documentId,preview.operationId);
    if(old)return this.recordedReply(await this.current(),await this.confirmOperation(old,await this.current()));
    const current=await this.current();if(current.document.revision!==preview.snapshot.document.revision||!sameNotebookSource(current.file,preview.plan.before)||!sameMathState(current,preview.snapshot))throw new Error('STALE_SNAPSHOT');
    return this.sourceEffect('apply_notebook_patch',current,preview.plan,callId,preview.operationId);
  }
  private async run(args:Record<string,unknown>,callId:string){
    const {snapshot}=await this.snapshotRef(args.snapshot_ref as string),ids=args.cell_ids as string[];
    const plan=await this.port.math(snapshot,{type:'plan',cell_ids:ids,scope:(args.scope as 'selected'|'required'|'affected'|undefined)??'required',allowed_cell_ids:snapshot.file.cells.filter(cell=>cell.kind==='Math').map(cell=>cell.id)});
    if(plan.type!=='plan')throw new Error('NOT_AVAILABLE');
    if(plan.plan.cycle_cell_ids.length||plan.plan.blocked_cell_ids.length||plan.plan.unavailable_prerequisite_cell_ids.length)throw new Error('DEPENDENCY_NOT_READY');
    const pending=await this.pending(snapshot);if(pending)return this.unknown(snapshot,pending);
    const start=await this.begin('run_cells',args,callId);if(!start.created)return this.recordedReply(snapshot,start.record);
    const cells:Record<string,unknown>[]=[],executed:string[]=[],refs:string[]=[];
    let cancelled=false,failed=false,unknown=false;
    for(const id of plan.plan.ordered_cell_ids){
      try{
        this.check();const current=await this.current();
        if(!sameNotebookSource(snapshot.file,current.file))throw new Error('STALE_SNAPSHOT');
        const result=await this.port.math(current,{type:'run',cell_id:id});if(result.type!=='ran')throw new Error('UNKNOWN_OUTCOME');
        executed.push(id);const fresh=await this.current(true);
        const resultRef=result.output?this.result(fresh,id,result.output,'notebook',result.status,{source:snapshot.file.cells.find(cell=>cell.id===id)!.source,execCount:result.state.cells.find(cell=>cell.id===id)?.exec_count,outputId:result.outputId}):null;if(resultRef)refs.push(resultRef);
        cells.push({cell_id:id,source_hash:await sha256(snapshot.file.cells.find(cell=>cell.id===id)!.source),status:result.status==='Done'?'done':'error',result_ref:resultRef,diagnostics:result.output?diagnostics(result.output,id):[]});
        if(result.status!=='Done'){failed=true;if(!args.continue_on_error)break;}
      }catch(error){const code=error instanceof Error?error.message:'';cancelled=code==='CANCELLED'||this.options.signal.aborted;failed=!cancelled;unknown=code==='UNKNOWN_OUTCOME';break;}
    }
    const outcome=cancelled?'cancelled':failed?executed.length?'partial':'failed':'completed';
    const reply=this.envelope(snapshot,{run_ref:start.record.id,run_outcome:outcome,cells,executed_cell_ids:executed});reply.operation_ref=start.record.id;
    if(failed||cancelled){reply.ok=false;reply.status=outcome==='partial'?'partial':cancelled?'cancelled':'failed';reply.error={code:cancelled?'CANCELLED':'EXECUTION_FAILED',message:cancelled?'CANCELLED':'EXECUTION_FAILED',recovery:'none',details:{}};}
    const record={...start.record,state:unknown?'unknown' as const:cancelled?'cancelled' as const:failed?'failed' as const:'completed' as const,effectCommitted:unknown?null:executed.length>0,resultJson:JSON.stringify(reply),resultRefs:refs,updatedAt:Date.now()};
    try{validate_agent_payload('run_cells','result',JSON.stringify(reply));}catch{const invalid={...record,state:'unknown' as const,resultJson:null};await this.store.settleOperation(invalid);return this.unknown(snapshot,invalid);}
    await this.store.settleOperation(record);return unknown?this.unknown(snapshot,record):reply;
  }
  private async scratch(args:Record<string,unknown>){
    const {snapshot}=await this.snapshotRef(args.snapshot_ref as string);
    const result=await this.port.math(snapshot,{type:'scratch',source:args.source as string,dialect:args.dialect as Dialect,use_notebook_definitions:(args.use_notebook_definitions as boolean|undefined)??true});
    if(result.type!=='scratch'||!result.output)throw new Error('NOT_AVAILABLE');
    const ref=this.result(snapshot,null,result.output,'scratch',result.status);
    const last=result.output.items.at(-1),unevaluated=last?.type==='expr'&&last.unchanged===true&&last.nature==='symbolic';
    return this.envelope(snapshot,{result_ref:ref,evaluation_status:result.status==='Error'?'error':unevaluated?'unevaluated':'done',output_summary:summary(result.output),diagnostics:diagnostics(result.output,null),notebook_modified:false});
  }
  private async inspect(args:Record<string,unknown>){
    const refId=args.result_ref as string,ref=this.get(refId,'result'),current=await this.current(),part=(args.part as string|undefined)??'summary';
    const cell=ref.cellId?current.cells.find(cell=>cell.id===ref.cellId):undefined,source=ref.cellId?current.file.cells.find(cell=>cell.id===ref.cellId):undefined;
    const isCurrent=ref.origin==='scratch'||!!cell&&cell.status!=='Stale'&&cell.status!=='Running'&&cell.exec_count===ref.execCount&&(ref.outputId!==undefined?cell.outputId===ref.outputId:JSON.stringify(cell.output)===JSON.stringify(ref.output))&&source?.source===ref.sourceHash&&JSON.stringify(current.general)===JSON.stringify(ref.snapshot.general);
    if(!isCurrent)throw new Error('STALE_RESULT');
    let payload:unknown=part==='summary'?summary(ref.output):part==='diagnostics'?[...ref.output.messages.map(message=>({type:'message',message})),...ref.output.items.filter(item=>item.type==='error')]:part==='steps'?ref.output.items.flatMap(item=>item.type==='solutions'&&item.steps?[{out_index:item.out_index,steps:item.steps}]:[]):part==='geometry'?ref.output.items.flatMap<unknown>(item=>item.type==='plot'?[{type:item.type,data:item.data}]:item.type==='scene3_d'?[{type:item.type,data:item.data}]:[]):part==='preview'?{available:false,reason:'No rendered image has been registered for this result'}:ref.output.items;
    let next:string|null=null;
    const limit=(args.limit as number|undefined)??20;
    const cursor=args.cursor?this.get(args.cursor as string,'cursor'):undefined;
    if(cursor&&(cursor.owner!==refId||cursor.part!==part))throw new Error('INVALID_REFERENCE');
    if(cursor?.chunks){
      const index=cursor.offset;payload={representation:'json_chunks',json_chunk:cursor.chunks[index],index,total:cursor.chunks.length,complete:index===cursor.chunks.length-1};
      next=index+1<cursor.chunks.length?this.put({...cursor,offset:index+1}):cursor.after??null;
    }else{
      const array=Array.isArray(payload)?payload:typeof payload==='object'&&payload!==null&&Array.isArray((payload as {items?:unknown}).items)?(payload as {items:unknown[]}).items:undefined;
      if(cursor&&!cursor.array)throw new Error('INVALID_REFERENCE');
      if(array&&(cursor||array.length>limit)){
        const offset=cursor?.offset??0,items=array.slice(offset,offset+limit);
        payload=Array.isArray(payload)?{items,total:array.length}:{...(payload as Record<string,unknown>),items,total:array.length};
        if(offset+limit<array.length)next=this.put({kind:'cursor',owner:refId,part,offset:offset+limit,array:true});
      }
      if(bytes(payload)>16000){
        const scalars=Array.from(JSON.stringify(payload)),chunks:string[]=[];for(let i=0;i<scalars.length;i+=4000)chunks.push(scalars.slice(i,i+4000).join(''));
        payload={representation:'json_chunks',json_chunk:chunks[0],index:0,total:chunks.length,complete:chunks.length===1};
        if(chunks.length>1)next=this.put({kind:'cursor',owner:refId,part,offset:1,chunks,...(next?{after:next}:{})});
      }
    }
    return this.envelope(current,{origin:ref.origin,is_current:isCurrent,part,payload,render_state:part==='geometry'||part==='preview'?'data_only':'not_applicable',media_ref:null},{truncated:next!==null,next_cursor:next});
  }
  private async undo(args:Record<string,unknown>,callId:string){
    const {snapshot}=await this.snapshotRef(args.snapshot_ref as string),refs=args.transaction_refs as string[];
    let after=structuredClone(snapshot.file);const changed=new Set<string>();
    const transactions=await Promise.all(refs.map(id=>this.store.sourceTransaction(this.options.documentId,id)));
    if(transactions.some(tx=>!tx))throw new Error('INVALID_REFERENCE');
    const active=transactions.filter((tx):tx is AgentTransactionRecord=>!!tx&&!tx.undoneBy);
    if(!active.length)return this.envelope(snapshot,{undo_transaction_ref:transactions[0]!.undoneBy,snapshot_ref:args.snapshot_ref,changed_cell_ids:[],invalidated_cell_ids:[],saved:false});
    for(const tx of active.sort((a,b)=>b.createdAt-a.createdAt)){after=invertSourceTransaction(after,tx.before,tx.after);tx.changedCellIds.forEach(id=>changed.add(id));}
    const assessed=await this.port.math(snapshot,{type:'preview',file:after});if(assessed.type!=='preview')throw new Error('NOT_AVAILABLE');
    const plan:PatchPlan={before:snapshot.file,after,assignedCellIds:{},changedCellIds:[...changed],invalidatedCellIds:assessed.affected_cell_ids,planHash:await sha256(JSON.stringify({before:snapshot.file,after,refs}))};
    return this.sourceEffect('undo_transaction',snapshot,plan,callId,undefined,active.map(tx=>tx.id));
  }
  private async operation(snapshot:ToolSnapshot,args:Record<string,unknown>){
    const saved=await this.store.operation(this.options.documentId,args.operation_ref as string);if(!saved)throw new Error('INVALID_REFERENCE');
    const record=await this.confirmOperation(saved,snapshot);
    const known=['completed','failed','cancelled'].includes(record.state),state=known?record.state:record.state==='running'?'running':'unknown';
    return this.envelope(snapshot,{operation_state:state,outcome_known:known,effect_committed:record.effectCommitted,transaction_ref:record.transactionRef,run_ref:record.tool==='run_cells'?record.id:null,result_refs:record.resultRefs,recovery_hint:known?'':'Do not replay this operation; its outcome has not been confirmed.'});
  }
}
