import {test,expect} from '@playwright/test';
import {readFile,readdir} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {resolve} from 'node:path';
import {build} from 'vite';
import type {ModelResult,CanonicalRecord} from '../src/agent/modelTypes';
const appRoot=fileURLToPath(new URL('../',import.meta.url));
const output=fileURLToPath(new URL('../../target/plan-5/f1-browser-build/',import.meta.url));
const assets=new Map<string,Buffer>();
test.beforeAll(async()=>{
  await build({configFile:resolve(appRoot,'vite.config.ts'),root:appRoot,base:'./',publicDir:false,
    build:{outDir:output,emptyOutDir:true,rollupOptions:{input:{worker:resolve(appRoot,'e2e/fixtures/agent-worker.ts'),owner:resolve(appRoot,'e2e/fixtures/agent-owner.ts')},preserveEntrySignatures:'strict',output:{entryFileNames:'[name].js',assetFileNames:'assets/[name]-[hash][extname]'}}}});
  for(const file of await readdir(output,{recursive:true}))if(/\.(?:js|wasm)$/.test(file))assets.set(file,await readFile(resolve(output,file)));
});
for(const mode of ['normal','incomplete','cancel'] as const)test(`production Pi Worker uses Rust codec/HTTP driver and respects ${mode}`,async({page,context})=>{
  const bodies:Record<string,unknown>[]=[];
  const heldResponse=Promise.withResolvers<void>();
  await context.route('**/agent-worker-probe/**',async route=>{
    const name=new URL(route.request().url()).pathname.slice('/agent-worker-probe/'.length),body=assets.get(name);
    if(!body){await route.abort();return;}
    await route.fulfill({contentType:name.endsWith('.wasm')?'application/wasm':'text/javascript',body});
  });
  await context.route('https://agent-loopback.invalid/**',async route=>{
    bodies.push(route.request().postDataJSON());
    if(mode==='cancel')await heldResponse.promise;
    const first=bodies.length===1;
    const delta=first?{tool_calls:[{index:0,id:'provider-call-read-17',type:'function',function:{name:'read_notebook',arguments:'{}'}}]}:{content:'已读取 2 个单元格。'};
    const frame={id:'response-'+bodies.length,model:'loopback',choices:[{index:0,delta,finish_reason:mode==='incomplete'?null:first?'tool_calls':'stop'}]};
    await route.fulfill({status:200,contentType:'text/event-stream',headers:{'access-control-allow-origin':'*'},body:`data: ${JSON.stringify(frame)}\n\n${mode==='incomplete'?'':'data: [DONE]\n\n'}`}).catch(()=>{});
  });
  await page.goto('/');
  type Result={ok:boolean;requests:number;reads:number;rounds:ModelResult[];history:CanonicalRecord[];reason:string;error:string|null};
  try{
    const result=await page.evaluate(mode=>new Promise<Result>((resolve,reject)=>{
      const worker=new Worker('/agent-worker-probe/worker.js',{type:'module'});
      const timer=setTimeout(()=>{worker.terminate();reject(new Error('Agent Worker timed out'));},15000);
      worker.onerror=event=>{clearTimeout(timer);worker.terminate();reject(new Error(event.message));};
      worker.onmessage=event=>{
        if(event.data.type==='request'&&mode==='cancel')worker.postMessage({type:'stop'});
        if(event.data.type==='result'){clearTimeout(timer);worker.terminate();resolve(event.data);}
      };
      worker.postMessage({type:'start',endpoint:'https://agent-loopback.invalid/v1/chat/completions'});
    }),mode);
    if(mode!=='normal'){
      expect(result.ok).toBe(false);expect(result.reads).toBe(0);expect(result.requests).toBe(1);
      expect(result.reason).toBe(mode==='cancel'?'aborted':'error');return;
    }
    expect(result).toMatchObject({ok:true,requests:2,reads:1,reason:'stop'});
    expect(bodies).toHaveLength(2);
    const second=bodies[1]!.messages as {role:string;tool_calls?:{id:string}[];tool_call_id?:string;content?:string}[];
    expect(second.find(message=>message.role==='assistant')?.tool_calls?.[0]?.id).toBe('provider-call-read-17');
    const read=second.find(message=>message.role==='tool');expect(read?.tool_call_id).toBe('provider-call-read-17');
    expect(JSON.parse(read!.content!).cells.map((cell:{source:string})=>cell.source)).toEqual(['let a=2','a+1']);
    expect(result.rounds[0]?.message.message_id).toBe('assistant-fixture-1');
    expect(result.rounds[0]?.usage.input).toBeNull();
    expect(result.rounds[1]?.message.parts).toContainEqual(expect.objectContaining({kind:'text',text:'已读取 2 个单元格。'}));
  }finally{heldResponse.resolve();}
});

test('browser agent journal persists ordered events and immutable receipts separately per document',async({page,context})=>{
  await context.route('**/agent-worker-probe/**',async route=>{
    const name=new URL(route.request().url()).pathname.slice('/agent-worker-probe/'.length),body=assets.get(name);
    if(!body){await route.abort();return;}
    await route.fulfill({contentType:name.endsWith('.wasm')?'application/wasm':'text/javascript',body});
  });
  await page.goto('/');
  const result=await page.evaluate(()=>new Promise<Record<string,unknown>>((resolve,reject)=>{
    const worker=new Worker('/agent-worker-probe/worker.js',{type:'module'});
    const timer=setTimeout(()=>{worker.terminate();reject(new Error('Storage Worker timed out'));},15000);
    worker.onerror=event=>{clearTimeout(timer);worker.terminate();reject(new Error(event.message));};
    worker.onmessage=event=>{if(event.data.type==='result'){clearTimeout(timer);worker.terminate();resolve(event.data);}};
    worker.postMessage({type:'store'});
  }));
  expect(result).toMatchObject({ok:true,firstCreated:true,duplicateCreated:false,conflict:true,foreign:true,stale:true,hidden:true,sequences:[1,2,3,4,5],metadataSafe:true,pageNext:2,tailLength:3,tailNext:null,immutableState:'completed'});
  expect(result.receipt).toMatchObject({state:'completed',effectCommitted:true,resultJson:'committed-body'});
  expect(result.firstPayload).toMatchObject({kind:'message',record:{canonical:{parts:[{text:'original'}]}}});
  expect(result.source).toMatchObject({cells:[{id:'a',source:'5'}]});
  expect(result.transaction).toMatchObject({id:'transaction-a',before:{cells:[{source:'2'}]},after:{cells:[{source:'5'}]}});
});

test('all ten real tools validate arguments and document fences, execute a source transaction, inspect and undo',async({page,context})=>{
  await context.route('**/agent-worker-probe/**',async route=>{
    const name=new URL(route.request().url()).pathname.slice('/agent-worker-probe/'.length),body=assets.get(name);
    if(!body){await route.abort();return;}await route.fulfill({contentType:name.endsWith('.wasm')?'application/wasm':'text/javascript',body});
  });
  await page.goto('/');
  const result=await page.evaluate(()=>new Promise<Record<string,unknown>>((resolve,reject)=>{
    const worker=new Worker('/agent-worker-probe/worker.js',{type:'module'});
    const timer=setTimeout(()=>{worker.terminate();reject(new Error('Tool Worker timed out'));},20000);
    worker.onerror=event=>{clearTimeout(timer);worker.terminate();reject(new Error(event.message));};
    worker.onmessage=event=>{if(event.data.type==='result'){clearTimeout(timer);worker.terminate();resolve(event.data);}};worker.postMessage({type:'tools'});
  }));
  expect(result,JSON.stringify(result)).toMatchObject({ok:true,replayed:true,stale:'STALE_SNAPSHOT',staleResult:'STALE_RESULT',commits:3,parentHasTemp:false});
  expect(Object.keys(result.normal as object)).toHaveLength(10);
  expect(Object.values(result.invalid as object)).toEqual(Array(10).fill('INVALID_ARGUMENT'));
  expect(Object.values(result.expired as object)).toEqual(Array(10).fill('STALE_SNAPSHOT'));
  expect(result.inspected).toMatchObject({is_current:true,payload:{items:[{input_form:'6'}]}});
  expect(result.scratch).toMatchObject({notebook_modified:false,output_summary:{items:[{input_form:'13'}]}});
  expect(result.after).toMatchObject({cells:[{id:'a',source:'let a=2'},{id:'b',source:'a+1'},{id:'u',source:'let unrelated=101'}]});
});

for(const mode of ['execute','discuss'] as const)test(`actual notebook owner and Pi Worker complete ${mode} S1 using loopback HTTP`,async({page,context})=>{
  const bodies:Record<string,unknown>[]=[],called:string[]=[];
  await context.route('**/agent-worker-probe/**',async route=>{
    const name=new URL(route.request().url()).pathname.slice('/agent-worker-probe/'.length),body=assets.get(name);
    if(!body){await route.abort();return;}await route.fulfill({contentType:name.endsWith('.wasm')?'application/wasm':'text/javascript',body});
  });
  await context.route('https://agent-owner-loopback.invalid/**',async route=>{
    const body=route.request().postDataJSON() as Record<string,unknown>;bodies.push(body);
    const messages=body.messages as {role:string;content?:string}[];
    const previous=messages.filter(message=>message.role==='tool').map(message=>JSON.parse(message.content!) as {ok:boolean;data:Record<string,unknown>;error?:unknown});
    const phase=previous.length;let name='',args:Record<string,unknown>={},text='';
    if(phase===0){name='read_notebook';args={view:'outline'};}
    else if(mode==='discuss'){
      if(phase===1){name='evaluate_scratch';args={snapshot_ref:previous[0]!.data.snapshot_ref,source:'let a=5; a+1',dialect:'Modern',use_notebook_definitions:false};}
      else{text='临时计算得到6，笔记本没有修改。';}
    }else if(phase===1){name='read_notebook';args={view:'cells',cell_ids:(previous[0]!.data.cells as {cell_id:string}[]).map(cell=>cell.cell_id),include_outputs:true};}
    else if(phase===2){
      const cells=previous[1]!.data.cells as {cell_id:string;source:string;source_hash:string}[],a=cells.find(cell=>cell.source==='let a=2')!;
      name='preview_source';args={snapshot_ref:previous[1]!.data.snapshot_ref,input:{kind:'patch',operations:[{type:'replace_text',target:{cell_id:a.cell_id},expected_source_hash:a.source_hash,match:'2',replacement:'5'}]}};
    }else if(phase===3){name='apply_notebook_patch';args={preview_ref:previous[2]!.data.preview_ref};}
    else if(phase===4){const cells=previous[1]!.data.cells as {cell_id:string;source:string}[];name='run_cells';args={snapshot_ref:previous[3]!.data.snapshot_ref,cell_ids:[cells.find(cell=>cell.source==='let a=2')!.cell_id],scope:'affected'};}
    else if(phase===5){const cells=previous[4]!.data.cells as {cell_id:string;result_ref:string}[],source=previous[1]!.data.cells as {cell_id:string;source:string}[];name='inspect_result';args={result_ref:cells.find(cell=>cell.cell_id===source.find(cell=>cell.source==='a+1')!.cell_id)!.result_ref};}
    else{text='已修改a=5并运行相关单元格，a+1=6；尚未保存到文件。';}
    if(name)called.push(name);
    const delta=name?{tool_calls:[{index:0,id:'provider-owner-'+bodies.length,type:'function',function:{name,arguments:JSON.stringify(args)}}]}:{content:text};
    await route.fulfill({status:200,contentType:'text/event-stream',headers:{'access-control-allow-origin':'*'},body:`data: ${JSON.stringify({id:'owner-response-'+bodies.length,model:'loopback',choices:[{index:0,delta,finish_reason:name?'tool_calls':'stop'}]})}\n\ndata: [DONE]\n\n`});
  });
  await page.goto('/');
  const result=await page.evaluate(async mode=>{
    const modulePath='/agent-worker-probe/owner.js';const fixture=await import(/* @vite-ignore */modulePath) as {runAgentOwnerProbe:(endpoint:string,mode:'execute'|'discuss')=>Promise<Record<string,unknown>>};
    return fixture.runAgentOwnerProbe('https://agent-owner-loopback.invalid/v1/chat/completions',mode);
  },mode);
  expect(result,JSON.stringify({settled:result.settled,unrelatedUntouched:result.unrelatedUntouched,a:result.a,b:result.b})).toMatchObject({accepted:true,settled:{reason:'ended'},unrelatedUntouched:true,journalHasCredentials:false});
  expect((result.settled as {requests:number}).requests).toBe(bodies.length);expect(result.dispatchCount).toBe(bodies.length);
  expect(result.a).toMatchObject({source:mode==='execute'?'let a=5':'let a=2',status:'Done'});
  expect(result.b).toMatchObject({source:'a+1',status:'Done',output:{items:[{input_form:mode==='execute'?'6':'3'}]}});
  expect((bodies[0]!.messages as {role:string;content:string}[])[0]!.content).toContain('笔记本操作说明');
  const names=(bodies[0]!.tools as {function:{name:string}}[]).map(tool=>tool.function.name);
  if(mode==='execute'){expect(called).toEqual(['read_notebook','read_notebook','preview_source','apply_notebook_patch','run_cells','inspect_result']);expect(result.transactions).toHaveLength(1);expect(names).toHaveLength(10);}
  else{expect(called).toEqual(['read_notebook','evaluate_scratch']);expect(result.transactions).toHaveLength(0);expect(names).not.toContain('run_cells');expect(names).not.toContain('apply_notebook_patch');}
});
