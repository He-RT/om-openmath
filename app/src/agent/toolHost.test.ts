import {beforeAll,afterAll,expect,test,vi} from 'vitest';
import {readFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {webcrypto} from 'node:crypto';
import {initSync,Kernel} from '../kernel/wasm/om_wasm.js';
import {ToolHost} from './toolHost';
import {AgentStore,type AgentOperationRecord,type AgentTransactionRecord} from './store';
import type {ToolPort,ToolSnapshot} from './toolPort';
import type {MathResult} from './hostTypes';
import {toolNames,type ToolName} from './toolTypes';
import type {Request,Response} from '../kernel/client';
import type {CellOutput} from '../kernel/generated/CellOutput';
import {sameNotebookSource} from '../state/sourceTransaction';
beforeAll(()=>{vi.stubGlobal('crypto',webcrypto);initSync({module:new Uint8Array(readFileSync(resolve(process.cwd(),'src/kernel/wasm/om_wasm_bg.wasm')))});});
afterAll(()=>vi.unstubAllGlobals());
async function fixture(){
  const kernel=new Kernel(),store=new AgentStore('mock-journal');let sequence=0,generation=0,revision=0,calls=0,commits=0,lost=false;
  const request=(body:Request)=>JSON.parse(kernel.request(JSON.stringify({id:++sequence,body}))).response.body as Response;
  const outputs=new Map<string,{output:CellOutput;source:string;dialect:string}>();
  for(const [id,source]of [['a','let a=2'],['b','a+1']]){const result=request({type:'evaluate',cell_id:id!,source:source!,dialect:'Modern'});if(result.type==='evaluated')outputs.set(id!,{output:result.output,source:source!,dialect:'Modern'});}
  const operations=new Map<string,AgentOperationRecord>(),transactions=new Map<string,AgentTransactionRecord>(),accepted=new Map<string,{before:ToolSnapshot['file'];after:ToolSnapshot['file']}>();
  // These tests exercise handlers and real CAS/schema. IndexedDB atomicity is tested in the browser.
  vi.spyOn(store,'createSession').mockImplementation(async(documentId,id,title)=>({id,documentId,title,createdAt:0,nextSequence:0}));
  vi.spyOn(store,'putDocument').mockResolvedValue();
  vi.spyOn(store,'append').mockImplementation(async(documentId,sessionId,payload)=>({id:'event-'+(++sequence),documentId,sessionId,sequence,time:0,payload:structuredClone(payload)}));
  vi.spyOn(store,'beginOperation').mockImplementation(async record=>{const old=operations.get(record.id);if(!old)operations.set(record.id,structuredClone(record));return {created:!old,record:structuredClone(old??record)};});
  vi.spyOn(store,'settleOperation').mockImplementation(async(record,transaction)=>{operations.set(record.id,structuredClone(record));if(transaction)transactions.set(transaction.id,structuredClone(transaction));return structuredClone(record);});
  vi.spyOn(store,'operation').mockImplementation(async(documentId,id)=>{const record=operations.get(id);return record?.documentId===documentId?structuredClone(record):undefined;});
  vi.spyOn(store,'pendingOperations').mockImplementation(async()=>[...operations.values()].filter(record=>!['completed','failed','cancelled'].includes(record.state)).map(record=>structuredClone(record)));
  vi.spyOn(store,'sourceTransaction').mockImplementation(async(_documentId,id)=>transactions.get(id)??[...operations.values()].find(record=>record.effectCommitted===true&&record.intent?.transaction.id===id)?.intent?.transaction);
  const snapshot=async():Promise<ToolSnapshot>=>{
    const result=request({type:'get_notebook_state'}),config=request({type:'get_config'});if(result.type!=='notebook_state'||config.type!=='config')throw new Error('No fixture state');
    return {document:{id:'unit-document',generation,revision},file:result.state.file,general:config.config.general,busy:false,composing:[],cells:result.state.cells.map(cell=>{const value=outputs.get(cell.id);return {...cell,revision,...(value?{output:value.output,resultSource:{source:value.source,dialect:value.dialect}}:{})};})};
  };
  const port:ToolPort={snapshot,catalog:async()=>{const value=request({type:'get_function_catalog'});if(value.type!=='function_catalog')throw new Error('No catalog');return value.catalog;},variables:async()=>{const value=request({type:'get_variables'});return value.type==='variables'?value.items:[];},
    math:async(snapshot,command)=>{const result=JSON.parse(kernel.agent_math(JSON.stringify({expected:snapshot.file,general:snapshot.general,command}))) as MathResult;if(result.type==='ran'&&result.output){const source=snapshot.file.cells.find(cell=>cell.id===result.cell_id)!;outputs.set(source.id,{output:result.output,source:source.source,dialect:source.dialect});}return result;},
    commit:async(snapshot,after,id)=>{kernel.commit_source_file(JSON.stringify(snapshot.file),JSON.stringify(after));revision++;commits++;if(id)accepted.set(id,{before:snapshot.file,after});if(lost){lost=false;throw new Error('UNKNOWN_OUTCOME');}},
    sourceWasAccepted:async(id,before,after)=>{const receipt=accepted.get(id);return !!receipt&&sameNotebookSource(receipt.before,before)&&sameNotebookSource(receipt.after,after);}};
  const host=new ToolHost(port,store,{documentId:'unit-document',generation,sessionId:'unit-session',taskId:'unit-task',mode:'execute',signal:new AbortController().signal});await host.initialize();
  const invoke=(name:ToolName,args:unknown)=>host.invoke(name,JSON.stringify(args),'call-'+(++calls));
  const read=await invoke('read_notebook',{view:'cells',cell_ids:['a','b'],include_outputs:true});if(!read.ok)throw new Error('Fixture read failed');
  const source=read.data!.snapshot_ref as string,cells=read.data!.cells as {cell_id:string;source_hash:string;result_ref:string}[];
  const patch=async()=>{
    const reply=await invoke('preview_source',{snapshot_ref:source,input:{kind:'patch',operations:[{type:'replace_text',target:{cell_id:'a'},expected_source_hash:cells[0]!.source_hash,match:'2',replacement:'5'}]}});expect(reply.ok).toBe(true);return reply.data!.preview_ref as string;
  };
  const args=async(name:ToolName):Promise<unknown>=>{
    if(name==='read_notebook')return {view:'cells',cell_ids:['a']};
    if(name==='search_functions')return {query:'solve'};
    if(name==='get_function_docs')return {name:'solve'};
    if(name==='preview_source')return {snapshot_ref:source,input:{kind:'source',source:'let a=5',dialect:'Modern'}};
    if(name==='apply_notebook_patch')return {preview_ref:await patch()};
    if(name==='run_cells')return {snapshot_ref:source,cell_ids:['b'],scope:'selected'};
    if(name==='inspect_result')return {result_ref:cells[1]!.result_ref};
    if(name==='evaluate_scratch')return {snapshot_ref:source,source:'let local=5; local+1',dialect:'Modern',use_notebook_definitions:false};
    const applied=await invoke('apply_notebook_patch',{preview_ref:await patch()});expect(applied.ok).toBe(true);
    if(name==='get_operation_status')return {operation_ref:applied.operation_ref};
    const current=await invoke('read_notebook',{});return {snapshot_ref:current.data!.snapshot_ref,transaction_refs:[applied.data!.transaction_ref]};
  };
  return {host,store,invoke,args,patch,snapshot,port,evaluate:(id:string,source:string)=>{const reply=request({type:'evaluate',cell_id:id,source,dialect:'Modern'});if(reply.type!=='evaluated')throw new Error('No evaluation');outputs.set(id,{output:reply.output,source,dialect:'Modern'});revision++;return reply.output;},expire:()=>generation++,loseAck:()=>{lost=true;},commits:()=>commits,close:()=>{host.dispose();store.close();kernel.free();}};
}
for(const name of toolNames)test(`${name}: valid result, closed bad arguments and expired document scope`,async()=>{
  const f=await fixture();try{
    const args=await f.args(name),valid=await f.invoke(name,args);expect(valid,JSON.stringify(valid)).toMatchObject({ok:true,status:'completed',error:null});
    if(name==='preview_source')expect(valid.data).toMatchObject({valid:true,preview_ref:null,affected_cell_ids:[]});
    const bad=await f.invoke(name,{unexpected:true});expect(bad.error?.code).toBe('INVALID_ARGUMENT');
    f.expire();const stale=await f.invoke(name,args);expect(stale.error?.code).toBe('STALE_SNAPSHOT');
  }finally{f.close();}
});
test('a lost source ACK is confirmed by the owner receipt, never by matching bytes or replaying a write',async()=>{
  const f=await fixture();try{
    const preview=await f.patch();f.loseAck();const lost=await f.invoke('apply_notebook_patch',{preview_ref:preview});
    expect(lost.status).toBe('unknown');expect(f.commits()).toBe(1);
    const confirmed=await f.invoke('get_operation_status',{operation_ref:lost.operation_ref});
    expect(confirmed.data).toMatchObject({operation_state:'completed',outcome_known:true,effect_committed:true});
    const replay=await f.invoke('apply_notebook_patch',{preview_ref:preview});expect(replay.error?.code).toBe('NOT_AVAILABLE');expect(f.commits()).toBe(1);
    const read=await f.invoke('read_notebook',{}),undo=await f.invoke('undo_transaction',{snapshot_ref:read.data!.snapshot_ref,transaction_refs:[confirmed.data!.transaction_ref]});
    expect(undo.ok).toBe(true);expect(f.commits()).toBe(2);
  }finally{f.close();}
});

test('long Unicode source has bounded cursor pages; partial source can use a unique replacement but not whole-cell overwrite',async()=>{
  const f=await fixture();try{
    const current=await f.snapshot(),source='#'+ '注'.repeat(40000)+'🙂 end',after={...current.file,cells:current.file.cells.map(cell=>cell.id==='a'?{...cell,source}:cell)};
    await f.port.commit(current,after);
    const first=await f.invoke('read_notebook',{view:'cells',cell_ids:['a']});expect(first.ok).toBe(true);
    const cell=(first.data!.cells as {source:string;source_hash:string;source_complete:boolean;source_cursor:string|null}[])[0]!;
    expect(cell.source_complete).toBe(false);expect(cell.source_cursor).toBeTypeOf('string');expect(cell.source.length).toBeLessThan(source.length);
    let joined=cell.source,cursor=cell.source_cursor;
    while(cursor){const page=await f.invoke('read_notebook',{cursor});expect(page.ok).toBe(true);const next=(page.data!.cells as typeof cell[])[0]!;joined+=next.source;cursor=next.source_cursor;}
    expect(joined).toBe(source);
    const full=await f.invoke('preview_source',{snapshot_ref:first.data!.snapshot_ref,input:{kind:'patch',operations:[{type:'update_cell',target:{cell_id:'a'},expected_source_hash:cell.source_hash,source:'2+2'}]}});
    expect(full.error?.code).toBe('INVALID_PATCH');
    const partial=await f.invoke('preview_source',{snapshot_ref:first.data!.snapshot_ref,input:{kind:'patch',operations:[{type:'replace_text',target:{cell_id:'a'},expected_source_hash:cell.source_hash,match:'end',replacement:'changed'}]}});
    expect(partial.data?.valid).toBe(true);
  }finally{f.close();}
});
test('inspect value honors row limits and reconstructs large exact JSON through bounded cursor chunks',async()=>{
  const f=await fixture();try{
    f.evaluate('multi','1\n2\n3');
    const read=await f.invoke('read_notebook',{view:'cells',cell_ids:['multi'],include_outputs:true});
    const ref=(read.data!.cells as {result_ref:string}[])[0]!.result_ref;
    let page=await f.invoke('inspect_result',{result_ref:ref,part:'value',limit:1}),cursor=page.pagination.next_cursor;
    const rows=[...(page.data!.payload as {items:{input_form:string}[]}).items];
    while(cursor){page=await f.invoke('inspect_result',{result_ref:ref,part:'value',limit:1,cursor});rows.push(...(page.data!.payload as {items:{input_form:string}[]}).items);cursor=page.pagination.next_cursor;}
    expect(rows.map(row=>row.input_form)).toEqual(['1','2','3']);
    const output=f.evaluate('large','['+Array.from({length:4000},(_,i)=>String(i)).join(',')+']');
    const largeRead=await f.invoke('read_notebook',{view:'cells',cell_ids:['large'],include_outputs:true}),largeRef=(largeRead.data!.cells as {result_ref:string}[])[0]!.result_ref;
    page=await f.invoke('inspect_result',{result_ref:largeRef,part:'value',limit:1});
    expect(page.data!.payload).toMatchObject({representation:'json_chunks',index:0});
    let json=(page.data!.payload as {json_chunk:string}).json_chunk;cursor=page.pagination.next_cursor;
    while(cursor){page=await f.invoke('inspect_result',{result_ref:largeRef,part:'value',cursor,limit:1});json+=(page.data!.payload as {json_chunk:string}).json_chunk;cursor=page.pagination.next_cursor;}
    expect(JSON.parse(json)).toEqual(output.items);
  }finally{f.close();}
});

test('inspected products freeze complete host counts before geometry chunking and reject stale results',async()=>{
  const f=await fixture();try{
    f.evaluate('figure','plot([sin(x), cos(x)], x: -pi..pi)');
    const read=await f.invoke('read_notebook',{view:'cells',cell_ids:['figure'],include_outputs:true}),ref=(read.data!.cells as {result_ref:string}[])[0]!.result_ref;
    const inspected=await f.invoke('inspect_result',{result_ref:ref,part:'geometry'});expect(inspected.ok).toBe(true);expect(inspected.pagination.truncated).toBe(true);
    const facts=()=>vi.mocked(f.store.append).mock.calls.map(call=>call[2]).filter(event=>event.kind==='product');expect(facts().at(-1)).toMatchObject({origin:'notebook',items:[{type:'plot',geometry_exists:true,curves:2}]});
    const count=facts().length;f.evaluate('figure','1');expect((await f.invoke('inspect_result',{result_ref:ref})).error?.code).toBe('STALE_RESULT');expect(facts()).toHaveLength(count);
  }finally{f.close();}
});
