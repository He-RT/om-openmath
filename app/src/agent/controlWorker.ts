/// <reference lib="webworker" />
import init from '../kernel/wasm/om_wasm.js';
import {ToolHost} from './toolHost';
import {WorkerToolPort} from './rpc';
import {AgentStore} from './store';
self.onmessage=(event:MessageEvent<{documentId:string;generation:number;sessionId:string;taskId:string;refs:string[];port:MessagePort}>)=>{
  const data=event.data,owner=new WorkerToolPort(data.port),store=new AgentStore();
  void(async()=>{
    let host:ToolHost|undefined;
    try{await init();host=new ToolHost(owner,store,{...data,mode:'execute',signal:new AbortController().signal});await host.initialize();const snapshot=await host.invoke('read_notebook','{}','control-read');if(!snapshot.ok)throw new Error(snapshot.error?.code);const reply=await host.invoke('undo_transaction',JSON.stringify({snapshot_ref:snapshot.data!.snapshot_ref,transaction_refs:data.refs}),'control-undo');self.postMessage({reply});}
    catch(error){self.postMessage({error:error instanceof Error?error.message:'Undo unavailable'});}
    finally{host?.dispose();owner.close();store.close();}
  })();
};
