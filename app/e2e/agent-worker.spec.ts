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
    build:{outDir:output,emptyOutDir:true,rollupOptions:{input:resolve(appRoot,'e2e/fixtures/agent-worker.ts'),output:{entryFileNames:'worker.js',assetFileNames:'assets/[name]-[hash][extname]'}}}});
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
