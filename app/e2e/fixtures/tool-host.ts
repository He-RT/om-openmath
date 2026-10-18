import init,{Kernel} from '../../src/kernel/wasm/om_wasm.js';
import {ToolHost} from '../../src/agent/toolHost';
import {AgentStore} from '../../src/agent/store';
import type {ToolSnapshot,ToolPort} from '../../src/agent/toolPort';
import type {MathResult} from '../../src/agent/hostTypes';
import type {ToolName,ToolReply} from '../../src/agent/toolTypes';
import {toolNames} from '../../src/agent/toolTypes';
import type {Request,Response} from '../../src/kernel/client';
import type {CellOutput} from '../../src/kernel/generated/CellOutput';
import type {NotebookFile} from '../../src/kernel/generated/NotebookFile';
export async function toolHostProbe(){
  await init();const kernel=new Kernel(),store=new AgentStore('openmath.agent.tools.test.'+crypto.randomUUID());
  let sequence=0,revision=0,generation=0,commits=0;
  const request=(body:Request):Response=>JSON.parse(kernel.request(JSON.stringify({id:++sequence,body}))).response.body as Response;
  const outputs=new Map<string,{output:CellOutput;source:string;dialect:string}>();
  for(const [id,source]of [['a','let a=2'],['b','a+1'],['u','let unrelated=99']]){
    const result=request({type:'evaluate',cell_id:id!,source:source!,dialect:'Modern'});
    if(result.type==='evaluated')outputs.set(id!,{output:result.output,source:source!,dialect:'Modern'});
  }
  const snapshot=async():Promise<ToolSnapshot>=>{
    const state=request({type:'get_notebook_state'}),config=request({type:'get_config'});
    if(state.type!=='notebook_state'||config.type!=='config')throw new Error('snapshot failed');
    return {document:{id:'tools-doc',generation,revision},file:state.state.file,general:config.config.general,busy:false,composing:[],cells:state.state.cells.map(cell=>{
      const result=outputs.get(cell.id);return {...cell,revision,...(result?{output:result.output,resultSource:{source:result.source,dialect:result.dialect}}:{})};
    })};
  };
  const port:ToolPort={snapshot,catalog:async()=>{const response=request({type:'get_function_catalog'});if(response.type!=='function_catalog')throw new Error('catalog failed');return response.catalog;},
    variables:async()=>{const response=request({type:'get_variables'});if(response.type!=='variables')throw new Error('variables failed');return response.items;},
    math:async(snapshot,command)=>{
      const result=JSON.parse(kernel.agent_math(JSON.stringify({expected:snapshot.file,general:snapshot.general,command}))) as MathResult;
      if(result.type==='ran'&&result.output){const cell=snapshot.file.cells.find(cell=>cell.id===result.cell_id)!;outputs.set(cell.id,{output:result.output,source:cell.source,dialect:cell.dialect});}
      return result;
    },commit:async(snapshot,desired)=>{kernel.commit_source_file(JSON.stringify(snapshot.file),JSON.stringify(desired));revision++;commits++;}};
  const controller=new AbortController(),options={documentId:'tools-doc',generation,sessionId:'tools-session',taskId:'tools-task',mode:'execute' as const,signal:controller.signal};
  const host=new ToolHost(port,store,options),normal:Record<string,ToolReply>={};let calls=0;
  const invoke=(name:ToolName,args:unknown)=>host.invoke(name,JSON.stringify(args),'call-'+(++calls));
  const required=async(name:ToolName,args:unknown)=>{const reply=await invoke(name,args);normal[name]=reply;if(!reply.ok)throw new Error(name+':'+JSON.stringify(reply));return reply.data!;};
  try{
    await host.initialize();
    const read=await required('read_notebook',{view:'cells',cell_ids:['a','b','u'],include_outputs:true});
    const oldRef=read.snapshot_ref as string,cells=read.cells as {cell_id:string;source_hash:string}[];
    const search=await required('search_functions',{query:'solve'}),found=(search.functions as {function_id:string}[])[0]!;
    await required('get_function_docs',{function_id:found.function_id});
    const preview=await required('preview_source',{snapshot_ref:oldRef,input:{kind:'patch',operations:[{type:'replace_text',target:{cell_id:'a'},expected_source_hash:cells.find(cell=>cell.cell_id==='a')!.source_hash,match:'2',replacement:'5'}]}});
    const applied=await required('apply_notebook_patch',{preview_ref:preview.preview_ref}),applyOperation=normal.apply_notebook_patch!.operation_ref!;
    const replay=await invoke('apply_notebook_patch',{preview_ref:preview.preview_ref});
    const stale=await invoke('run_cells',{snapshot_ref:oldRef,cell_ids:['b']});
    const fresh=await required('read_notebook',{view:'cells',cell_ids:['a','b','u']});
    const ran=await required('run_cells',{snapshot_ref:fresh.snapshot_ref,cell_ids:['a'],scope:'affected'});
    const b=(ran.cells as {cell_id:string;result_ref:string}[]).find(cell=>cell.cell_id==='b')!;
    const inspected=await required('inspect_result',{result_ref:b.result_ref});
    const scratchSnapshot=await required('read_notebook',{});
    const scratch=await required('evaluate_scratch',{snapshot_ref:scratchSnapshot.snapshot_ref,source:'let temp=8; a+temp',dialect:'Modern',use_notebook_definitions:true});
    const variables=await port.variables();
    const beforeUndo=await snapshot(),changed:NotebookFile={...beforeUndo.file,cells:beforeUndo.file.cells.map(cell=>cell.id==='u'?{...cell,source:'let unrelated=101'}:cell)};
    await port.commit(beforeUndo,changed);
    const undoSnapshot=await required('read_notebook',{view:'cells',cell_ids:['a','u']});
    await required('undo_transaction',{snapshot_ref:undoSnapshot.snapshot_ref,transaction_refs:[applied.transaction_ref]});
    await required('get_operation_status',{operation_ref:applyOperation});
    const after=await snapshot(),staleResult=await invoke('inspect_result',{result_ref:b.result_ref});
    const invalid:Record<string,string>={},expired:Record<string,string>={};
    for(const name of toolNames)invalid[name]=(await invoke(name,{unexpected:true})).error?.code??'no error';
    generation++;
    const validArgs:Record<ToolName,unknown>={read_notebook:{},search_functions:{query:'solve'},get_function_docs:{name:'solve'},preview_source:{snapshot_ref:oldRef,input:{kind:'source',source:'2+2',dialect:'Modern'}},apply_notebook_patch:{preview_ref:preview.preview_ref},run_cells:{snapshot_ref:oldRef,cell_ids:['a']},inspect_result:{result_ref:b.result_ref},evaluate_scratch:{snapshot_ref:oldRef,source:'2+2',dialect:'Modern'},undo_transaction:{snapshot_ref:oldRef,transaction_refs:[applied.transaction_ref]},get_operation_status:{operation_ref:applyOperation}};
    for(const name of toolNames)expired[name]=(await invoke(name,validArgs[name])).error?.code??'no error';
    const journal=await store.events('tools-doc','tools-session',0,100);
    return {ok:true,normal,invalid,expired,replayed:replay.operation_ref===applyOperation,stale:stale.error?.code,commits,
      inspected,scratch,staleResult:staleResult.error?.code,parentHasTemp:variables.some(([name])=>name==='temp'),after:after.file,journalTools:journal.items.filter(event=>event.payload.kind==='tool').length};
  }finally{host.dispose();store.close();kernel.free();}
}
