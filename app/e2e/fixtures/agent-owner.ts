import {WasmClient} from '../../src/kernel/wasmClient';
import {NotebookController} from '../../src/state/notebookStore';
import {BrowserAgentService} from '../../src/agent/service';
import {AgentStore} from '../../src/agent/store';
import type {AgentRuntimeEvent,BrowserAgentModel} from '../../src/agent/runtimeTypes';
import pkg from '../../package.json';
export async function runAgentOwnerProbe(endpoint:string,mode:'execute'|'discuss'='execute'){
  const kernel=new WasmClient(),controller=new NotebookController(kernel);await controller.initialize();
  const a=controller.add('Math','let a=2');await controller.run(a);
  const b=controller.add('Math','a+1');await controller.run(b);
  const u=controller.add('Math','let unrelated=99');await controller.run(u);
  const before=structuredClone(controller.store.getState().cells.find(cell=>cell.id===u));
  const service=new BrowserAgentService(controller,pkg.version),events:AgentRuntimeEvent[]=[];
  service.onEvent(event=>events.push(event));
  const model:BrowserAgentModel={apiModel:'loopback',displayName:'Loopback',supportsTools:true,binding:{provider_ref:'provider-fixture',route_ref:'route-fixture',model_ref:'model-fixture',adapter_version:'single-turn-v1',protocol:'chat_completions'},url:endpoint,headers:[['Content-Type','application/json'],['Authorization','Bearer synthetic-owner-key']],maxOutputTokens:1024,timeoutMs:5000,contextWindow:65536};
  try{
    const handle=service.start(mode==='execute'?'把a改成5并更新相关结果。':'如果把a改成5，a+1是多少？',mode,model);
    const accepted=await handle.accepted,settled=await handle.finished;
    const cells=controller.store.getState().cells;
    const store=new AgentStore();let after=0;const journal=[];
    do{const page=await store.events(service.document.documentId,service.document.sessionId,after,100);journal.push(...page.items);if(page.next===null)break;after=page.next;}while(after>0);
    const toolEvents=events.filter((event):event is Extract<AgentRuntimeEvent,{type:'tool'}>=>event.type==='tool');
    const transactions=await Promise.all(toolEvents.flatMap(event=>typeof event.reply.data?.transaction_ref==='string'?[store.sourceTransaction(service.document.documentId,event.reply.data.transaction_ref)]:[]));
    store.close();
    return {accepted:accepted.task_id===handle.taskId&&accepted.role==='user',settled,events,
      a:cells.find(cell=>cell.id===a),b:cells.find(cell=>cell.id===b),unrelatedUntouched:['source','kind','dialect','status','exec_count','output','outputId','defines','uses','resultSource','revision'].every(key=>JSON.stringify(before?.[key as keyof typeof before])===JSON.stringify(cells.find(cell=>cell.id===u)?.[key as keyof typeof before])),
      file:controller.file(),transactions,journalCount:journal.length,dispatchCount:journal.filter(event=>event.payload.kind==='dispatch').length,journalHasCredentials:JSON.stringify(journal).includes('synthetic-owner-key'),inputs:journal.filter(event=>event.payload.kind==='input').map(event=>event.payload)};
  }finally{await service.dispose();controller.dispose();kernel.dispose();}
}
